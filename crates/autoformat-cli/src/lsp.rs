//! `autoformat lsp`: lets editors format through the Language Server Protocol.
//!
//! Editors such as Zed, Neovim and Helix use this protocol to talk to formatters. The
//! editor starts `autoformat lsp`, tells it the text of every open document, and asks it
//! to format one. The answer is a single edit that replaces the whole document.
//!
//! Only what formatting needs is implemented:
//! - `initialize`, `shutdown` and `exit`,
//! - `textDocument/didOpen`, `textDocument/didChange` and `textDocument/didClose`
//!   (the editor is asked to always send the full text),
//! - `textDocument/formatting`.
//!
//! Messages are JSON. Each one comes after a `Content-Length` header, on standard input
//! and standard output. Nothing else is ever written to standard output.
//!
//! A document that cannot be formatted (a syntax error, a broken `autoformat.toml`) is
//! left alone: the answer is "no edits", and the reason is sent to the editor's log.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use autoformat_config::discover_config;
use autoformat_core::{format_source, LanguageAdapter};
use autoformat_languages::Languages;
use serde_json::{json, Value};

/// Error codes from the JSON-RPC and Language Server Protocol specifications.
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

/// Message kinds for `window/showMessage` and `window/logMessage`.
const MESSAGE_ERROR: u8 = 1;
const MESSAGE_WARNING: u8 = 2;

/// `textDocumentSync.change` value that asks the editor to send the full text every time.
const SYNC_FULL: u8 = 1;

/// Messages larger than this are refused, so a damaged header cannot ask for all memory.
const MAX_MESSAGE_BYTES: usize = 256 * 1024 * 1024;

/// Runs `autoformat lsp` on standard input and output, and returns the exit code.
pub fn run() -> u8 {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    serve(&mut input, &mut output)
}

/// Reads messages from `input` and writes the answers to `output` until the editor says
/// `exit` or closes the connection. Returns the exit code.
fn serve(input: &mut dyn BufRead, output: &mut dyn Write) -> u8 {
    let mut server = Server::new();
    loop {
        let replies = match read_message(input) {
            Ok(Incoming::Message(message)) => server.handle(&message),
            Ok(Incoming::InvalidJson) => vec![error_reply(
                &Value::Null,
                PARSE_ERROR,
                "the message is not valid JSON",
            )],
            // The editor went away without saying `exit`.
            Ok(Incoming::End) => return 1,
            Err(message) => {
                eprintln!("error: {}", message);
                return 1;
            }
        };
        for reply in &replies {
            if let Err(error) = write_message(output, reply) {
                eprintln!("error: cannot write to the editor: {}", error);
                return 1;
            }
        }
        if let Some(code) = server.exit_code {
            return code;
        }
    }
}

/// What `read_message` found on the input.
enum Incoming {
    Message(Value),
    /// The content was not valid JSON. The next message can still be read.
    InvalidJson,
    /// The editor closed the connection.
    End,
}

/// Reads one message: header lines, an empty line, then `Content-Length` bytes of JSON.
/// An `Err` means the input can no longer be understood.
fn read_message(input: &mut dyn BufRead) -> Result<Incoming, String> {
    let mut content_length: Option<usize> = None;
    let mut read_any_header = false;
    loop {
        let mut line = Vec::new();
        let count = match input.read_until(b'\n', &mut line) {
            Ok(count) => count,
            Err(error) => return Err(format!("cannot read from the editor: {}", error)),
        };
        if count == 0 {
            if read_any_header {
                return Err("the editor stopped in the middle of a message".to_string());
            }
            return Ok(Incoming::End);
        }
        let line = String::from_utf8_lossy(&line);
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        read_any_header = true;
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                match value.trim().parse::<usize>() {
                    Ok(length) => content_length = Some(length),
                    Err(_) => return Err(format!("unreadable header: {}", line)),
                }
            }
        }
    }

    let length = match content_length {
        Some(length) => length,
        None => return Err("a message has no Content-Length header".to_string()),
    };
    if length > MAX_MESSAGE_BYTES {
        return Err(format!("a message of {} bytes is too large", length));
    }
    let mut body = vec![0u8; length];
    if let Err(error) = input.read_exact(&mut body) {
        return Err(format!("cannot read from the editor: {}", error));
    }
    match serde_json::from_slice(&body) {
        Ok(message) => Ok(Incoming::Message(message)),
        Err(_) => Ok(Incoming::InvalidJson),
    }
}

fn write_message(output: &mut dyn Write, message: &Value) -> std::io::Result<()> {
    let body = message.to_string();
    write!(output, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    output.flush()
}

/// An open document, as last sent by the editor.
struct Document {
    /// The editor's name for the language, for example `rust` or `typescriptreact`.
    language_id: String,
    text: String,
}

struct Server {
    languages: Languages,
    /// Open documents by their address (`file:///...`).
    documents: HashMap<String, Document>,
    /// The project folder. Documents that are not saved yet use its `autoformat.toml`.
    root: Option<PathBuf>,
    shutdown_requested: bool,
    /// Set when the editor says `exit`.
    exit_code: Option<u8>,
    /// The last configuration problem shown to the user, so that the same problem does
    /// not pop up again on every save.
    shown_config_error: Option<String>,
}

impl Server {
    fn new() -> Server {
        Server {
            languages: Languages::new(),
            documents: HashMap::new(),
            root: None,
            shutdown_requested: false,
            exit_code: None,
            shown_config_error: None,
        }
    }

    /// Handles one message and returns the messages to send back, in order.
    fn handle(&mut self, message: &Value) -> Vec<Value> {
        if !message.is_object() {
            return vec![error_reply(
                &Value::Null,
                INVALID_REQUEST,
                "the message is not a JSON object",
            )];
        }
        let method = match message.get("method").and_then(Value::as_str) {
            Some(method) => method,
            // An answer to a question. This server never asks the editor anything.
            None => return Vec::new(),
        };
        let params = &message["params"];
        match message.get("id") {
            Some(id) => self.handle_request(method, id, params),
            None => self.handle_notification(method, params),
        }
    }

    /// Requests have an `id` and always get exactly one answer.
    fn handle_request(&mut self, method: &str, id: &Value, params: &Value) -> Vec<Value> {
        if self.shutdown_requested {
            return vec![error_reply(
                id,
                INVALID_REQUEST,
                "autoformat is shutting down",
            )];
        }
        match method {
            "initialize" => {
                self.root = project_folder(params);
                vec![reply(id, initialize_result())]
            }
            "shutdown" => {
                self.shutdown_requested = true;
                vec![reply(id, Value::Null)]
            }
            "textDocument/formatting" => self.format(id, params),
            _ => vec![error_reply(
                id,
                METHOD_NOT_FOUND,
                &format!("autoformat does not support `{}`", method),
            )],
        }
    }

    /// Notifications have no `id` and get no answer. Unknown ones are ignored.
    fn handle_notification(&mut self, method: &str, params: &Value) -> Vec<Value> {
        match method {
            "textDocument/didOpen" => self.open(params),
            "textDocument/didChange" => return self.change(params),
            "textDocument/didClose" => self.close(params),
            "exit" => {
                // Leaving without `shutdown` first counts as a failure.
                let code = if self.shutdown_requested { 0 } else { 1 };
                self.exit_code = Some(code);
            }
            _ => {}
        }
        Vec::new()
    }

    fn open(&mut self, params: &Value) {
        let document = &params["textDocument"];
        let uri = document["uri"].as_str();
        let text = document["text"].as_str();
        if let (Some(uri), Some(text)) = (uri, text) {
            let language_id = document["languageId"].as_str().unwrap_or("").to_string();
            self.documents.insert(
                uri.to_string(),
                Document {
                    language_id,
                    text: text.to_string(),
                },
            );
        }
    }

    fn change(&mut self, params: &Value) -> Vec<Value> {
        let uri = match params["textDocument"]["uri"].as_str() {
            Some(uri) => uri,
            None => return Vec::new(),
        };
        let changes = match params["contentChanges"].as_array() {
            Some(changes) => changes,
            None => return Vec::new(),
        };
        for change in changes {
            let is_full_text = change.get("range").is_none_or(Value::is_null);
            let text = change["text"].as_str();
            if let (true, Some(text)) = (is_full_text, text) {
                match self.documents.get_mut(uri) {
                    Some(document) => document.text = text.to_string(),
                    None => {
                        let document = Document {
                            language_id: String::new(),
                            text: text.to_string(),
                        };
                        self.documents.insert(uri.to_string(), document);
                    }
                }
            } else {
                // The editor sent only a part of the text, although it was asked for all
                // of it. This server's copy can no longer be trusted, and formatting a
                // wrong copy would destroy the user's work. Forget the document instead.
                self.documents.remove(uri);
                return vec![log_message(
                    MESSAGE_ERROR,
                    &format!(
                        "{}: the editor sent a partial change, which autoformat does not \
                         support. The document is not formatted until it is opened again.",
                        uri
                    ),
                )];
            }
        }
        Vec::new()
    }

    fn close(&mut self, params: &Value) {
        if let Some(uri) = params["textDocument"]["uri"].as_str() {
            self.documents.remove(uri);
        }
    }

    /// Answers `textDocument/formatting` with one edit that replaces the whole document,
    /// with no edits when the text is already formatted, or with `null` when the document
    /// cannot be formatted.
    fn format(&mut self, id: &Value, params: &Value) -> Vec<Value> {
        let uri = match params["textDocument"]["uri"].as_str() {
            Some(uri) => uri,
            None => {
                return vec![error_reply(
                    id,
                    INVALID_PARAMS,
                    "textDocument.uri is missing",
                )]
            }
        };
        let document = match self.documents.get(uri) {
            Some(document) => document,
            None => {
                let text = format!("{}: the editor has not opened this document", uri);
                return vec![log_message(MESSAGE_WARNING, &text), reply(id, Value::Null)];
            }
        };
        let path = document_path(uri, self.root.as_deref());
        let adapter = match pick_language(&self.languages, &document.language_id, &path) {
            Some(adapter) => adapter,
            None => {
                let text = format!("{}: autoformat does not know this language", uri);
                return vec![log_message(MESSAGE_WARNING, &text), reply(id, Value::Null)];
            }
        };

        // The configuration is read again every time, so a changed autoformat.toml is
        // used right away.
        let config = match discover_config(&path) {
            Ok((config, _)) => {
                self.shown_config_error = None;
                config
            }
            Err(error) => {
                let text = error.to_string();
                let mut replies = vec![log_message(MESSAGE_ERROR, &text)];
                if self.shown_config_error.as_deref() != Some(text.as_str()) {
                    replies.push(show_message(MESSAGE_ERROR, &text));
                    self.shown_config_error = Some(text);
                }
                replies.push(reply(id, Value::Null));
                return replies;
            }
        };

        match format_source(adapter, &document.text, &config) {
            Ok(formatted) => {
                if formatted.text == document.text {
                    return vec![reply(id, json!([]))];
                }
                let (end_line, end_character) = end_position(&document.text);
                let edit = json!({
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": end_line, "character": end_character },
                    },
                    "newText": formatted.text,
                });
                vec![reply(id, json!([edit]))]
            }
            Err(error) => {
                let text = format!("{}: {}", path.display(), error);
                vec![log_message(MESSAGE_WARNING, &text), reply(id, Value::Null)]
            }
        }
    }
}

/// What this server tells the editor it can do.
fn initialize_result() -> Value {
    json!({
        "capabilities": {
            "textDocumentSync": { "openClose": true, "change": SYNC_FULL },
            "documentFormattingProvider": true,
        },
        "serverInfo": { "name": "autoformat", "version": env!("CARGO_PKG_VERSION") },
    })
}

/// The project folder from the `initialize` request, if the editor named one.
fn project_folder(params: &Value) -> Option<PathBuf> {
    let windows = cfg!(windows);
    if let Some(uri) = params["workspaceFolders"][0]["uri"].as_str() {
        if let Some(path) = file_uri_to_path(uri, windows) {
            return Some(PathBuf::from(path));
        }
    }
    if let Some(uri) = params["rootUri"].as_str() {
        if let Some(path) = file_uri_to_path(uri, windows) {
            return Some(PathBuf::from(path));
        }
    }
    params["rootPath"].as_str().map(PathBuf::from)
}

/// The path used to find `autoformat.toml` and, if needed, the language. Documents that
/// are not files (not saved yet) are treated as a file in the project folder, so the
/// project's configuration still applies.
fn document_path(uri: &str, root: Option<&Path>) -> PathBuf {
    if let Some(path) = file_uri_to_path(uri, cfg!(windows)) {
        return PathBuf::from(path);
    }
    match root {
        Some(root) => root.join("untitled"),
        None => PathBuf::from("untitled"),
    }
}

/// Picks the language from the editor's name for it. Editors use different names, so
/// names this server does not know fall back to the file extension.
fn pick_language<'a>(
    languages: &'a Languages,
    language_id: &str,
    path: &Path,
) -> Option<&'a dyn LanguageAdapter> {
    if let Some(name) = language_name(language_id) {
        if let Some(adapter) = languages.by_name(name) {
            return Some(adapter);
        }
    }
    languages.for_path(path)
}

/// Maps an editor's language name to an AutoFormat language name.
fn language_name(language_id: &str) -> Option<&'static str> {
    match language_id.to_lowercase().as_str() {
        "rust" => Some("rust"),
        "toml" => Some("toml"),
        "javascript" | "javascriptreact" | "jsx" => Some("javascript"),
        "typescript" => Some("typescript"),
        "typescriptreact" | "tsx" => Some("tsx"),
        "csharp" | "c-sharp" | "c#" | "cs" => Some("csharp"),
        "powershell" | "ps1" => Some("powershell"),
        _ => None,
    }
}

/// Turns `file:///home/me/a%20b.rs` into `/home/me/a b.rs`, and on Windows
/// `file:///c%3A/Users/me/a.rs` into `c:\Users\me\a.rs`. Returns `None` for anything that
/// is not a file on this computer.
fn file_uri_to_path(uri: &str, windows: bool) -> Option<String> {
    let scheme = uri.get(..7)?;
    if !scheme.eq_ignore_ascii_case("file://") {
        return None;
    }
    let rest = &uri[7..];
    // A `?` or `#` that belongs to a file name is always written as %3F or %23.
    let rest = match rest.find(['?', '#']) {
        Some(end) => &rest[..end],
        None => rest,
    };
    // Between `file://` and the next `/` is the computer's name, which is normally empty.
    let slash = rest.find('/')?;
    let host = &rest[..slash];
    let path = &rest[slash..];
    let path = percent_decode(path)?;
    let is_this_computer = host.is_empty() || host.eq_ignore_ascii_case("localhost");

    if !windows {
        if !is_this_computer {
            return None;
        }
        return Some(path);
    }
    if !is_this_computer {
        // A shared folder on another computer: \\server\share\file
        return Some(format!(r"\\{}{}", host, path.replace('/', r"\")));
    }
    // `/c:/Users/me` -> `c:\Users\me`
    let bytes = path.as_bytes();
    let has_drive = bytes.len() >= 3 && bytes[1].is_ascii_alphabetic() && bytes[2] == b':';
    let path = if has_drive { &path[1..] } else { &path[..] };
    Some(path.replace('/', r"\"))
}

/// Replaces `%20` style codes with the characters they stand for.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_value(*bytes.get(index + 1)?)?;
            let low = hex_value(*bytes.get(index + 2)?)?;
            decoded.push(high * 16 + low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// The position just after the last character of `text`, as the protocol counts it:
/// lines start at 0, and the place in a line is counted in UTF-16 units. A line ends at
/// `\n`, `\r\n` or `\r`.
fn end_position(text: &str) -> (u32, u32) {
    let mut line = 0;
    let mut character = 0;
    let mut characters = text.chars().peekable();
    while let Some(current) = characters.next() {
        if current == '\r' {
            if characters.peek() == Some(&'\n') {
                characters.next();
            }
            line += 1;
            character = 0;
        } else if current == '\n' {
            line += 1;
            character = 0;
        } else {
            character += current.len_utf16() as u32;
        }
    }
    (line, character)
}

fn reply(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_reply(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// A line for the editor's log of this server.
fn log_message(kind: u8, text: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "window/logMessage",
        "params": { "type": kind, "message": text },
    })
}

/// A message the editor shows to the user.
fn show_message(kind: u8, text: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "window/showMessage",
        "params": { "type": kind, "message": text },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wraps a message the way an editor sends it.
    fn framed(message: &Value) -> Vec<u8> {
        let body = message.to_string();
        format!("Content-Length: {}\r\n\r\n{}", body.len(), body).into_bytes()
    }

    /// Splits what the server wrote back into messages.
    fn unframed(output: &[u8]) -> Vec<Value> {
        let mut input = std::io::Cursor::new(output.to_vec());
        let mut messages = Vec::new();
        loop {
            match read_message(&mut input).unwrap() {
                Incoming::Message(message) => messages.push(message),
                Incoming::InvalidJson => panic!("the server wrote invalid JSON"),
                Incoming::End => return messages,
            }
        }
    }

    /// Sends `messages` to a fresh server. Returns its exit code and everything it sent.
    fn talk(messages: &[Value]) -> (u8, Vec<Value>) {
        let mut bytes = Vec::new();
        for message in messages {
            bytes.extend(framed(message));
        }
        let mut input = std::io::Cursor::new(bytes);
        let mut output = Vec::new();
        let code = serve(&mut input, &mut output);
        (code, unframed(&output))
    }

    fn request(id: u32, method: &str, params: Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
    }

    fn notification(method: &str, params: Value) -> Value {
        json!({ "jsonrpc": "2.0", "method": method, "params": params })
    }

    fn open(uri: &str, language_id: &str, text: &str) -> Value {
        notification(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri, "languageId": language_id, "version": 1, "text": text } }),
        )
    }

    fn change(uri: &str, text: &str) -> Value {
        notification(
            "textDocument/didChange",
            json!({ "textDocument": { "uri": uri, "version": 2 }, "contentChanges": [{ "text": text }] }),
        )
    }

    fn format(id: u32, uri: &str) -> Value {
        request(
            id,
            "textDocument/formatting",
            json!({ "textDocument": { "uri": uri }, "options": { "tabSize": 8, "insertSpaces": false } }),
        )
    }

    /// The answer to the request with this id.
    fn answer(messages: &[Value], id: u32) -> &Value {
        for message in messages {
            if message["id"] == json!(id) {
                return message;
            }
        }
        panic!("no answer to request {} in {:?}", id, messages);
    }

    /// All messages of one kind, for example `window/logMessage`.
    fn sent<'a>(messages: &'a [Value], method: &str) -> Vec<&'a Value> {
        messages
            .iter()
            .filter(|message| message["method"] == json!(method))
            .collect()
    }

    /// A fresh folder for one test, and the address of a file in it.
    fn temp_folder(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("autoformat-lsp-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn file_uri(path: &Path) -> String {
        let text = path.to_string_lossy().replace('\\', "/");
        if text.starts_with('/') {
            format!("file://{}", text)
        } else {
            format!("file:///{}", text)
        }
    }

    #[test]
    fn says_what_it_can_do() {
        let (_, messages) = talk(&[request(1, "initialize", json!({ "capabilities": {} }))]);
        let result = &answer(&messages, 1)["result"];
        assert_eq!(
            result["capabilities"]["documentFormattingProvider"],
            json!(true)
        );
        assert_eq!(
            result["capabilities"]["textDocumentSync"],
            json!({ "openClose": true, "change": 1 })
        );
        assert_eq!(result["serverInfo"]["name"], json!("autoformat"));
        assert_eq!(
            result["serverInfo"]["version"],
            json!(env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn formats_an_open_document_with_one_edit() {
        let uri = "file:///nowhere/main.rs";
        let (_, messages) = talk(&[
            request(1, "initialize", json!({})),
            notification("initialized", json!({})),
            open(uri, "rust", "fn main(){let x=1;}\n"),
            format(2, uri),
        ]);
        assert_eq!(
            answer(&messages, 2)["result"],
            json!([{
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 1, "character": 0 },
                },
                "newText": "fn main() { let x = 1; }\n",
            }])
        );
    }

    #[test]
    fn formatted_text_needs_no_edits() {
        let uri = "file:///nowhere/main.rs";
        let (_, messages) = talk(&[
            open(uri, "rust", "fn main() { let x = 1; }\n"),
            format(1, uri),
        ]);
        assert_eq!(answer(&messages, 1)["result"], json!([]));
    }

    #[test]
    fn uses_the_latest_text_after_a_change() {
        let uri = "file:///nowhere/a.ts";
        let (_, messages) = talk(&[
            open(uri, "typescript", "let a=1\n"),
            change(uri, "let b=2"),
            format(1, uri),
        ]);
        let edit = &answer(&messages, 1)["result"][0];
        assert_eq!(edit["newText"], json!("let b = 2\n"));
        // The old text has one line without a line break at its end.
        assert_eq!(edit["range"]["end"], json!({ "line": 0, "character": 7 }));
    }

    #[test]
    fn closed_documents_are_forgotten() {
        let uri = "file:///nowhere/main.rs";
        let (_, messages) = talk(&[
            open(uri, "rust", "fn main(){}\n"),
            notification(
                "textDocument/didClose",
                json!({ "textDocument": { "uri": uri } }),
            ),
            format(1, uri),
        ]);
        assert_eq!(answer(&messages, 1)["result"], Value::Null);
        assert_eq!(sent(&messages, "window/logMessage").len(), 1);
    }

    #[test]
    fn a_partial_change_stops_formatting_instead_of_guessing() {
        let uri = "file:///nowhere/main.rs";
        let partial = notification(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{
                    "range": {
                        "start": { "line": 0, "character": 0 },
                        "end": { "line": 0, "character": 2 },
                    },
                    "text": "pub fn",
                }],
            }),
        );
        let (_, messages) = talk(&[
            open(uri, "rust", "fn main(){}\n"),
            partial.clone(),
            format(1, uri),
        ]);
        assert_eq!(answer(&messages, 1)["result"], Value::Null);
        let logs = sent(&messages, "window/logMessage");
        assert!(logs[0]["params"]["message"]
            .as_str()
            .unwrap()
            .contains("partial change"));

        // Sending the full text again makes the document usable again.
        let full = notification(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": uri, "version": 3 },
                "contentChanges": [{ "range": null, "text": "fn a(){}\n" }],
            }),
        );
        let (_, messages) = talk(&[
            open(uri, "rust", "fn main(){}\n"),
            partial,
            full,
            format(1, uri),
        ]);
        assert_eq!(
            answer(&messages, 1)["result"][0]["newText"],
            json!("fn a() {}\n")
        );
    }

    #[test]
    fn syntax_errors_change_nothing_and_are_logged() {
        let uri = "file:///nowhere/broken.rs";
        let (_, messages) = talk(&[open(uri, "rust", "fn main( {\n"), format(1, uri)]);
        assert_eq!(answer(&messages, 1)["result"], Value::Null);
        let logs = sent(&messages, "window/logMessage");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0]["params"]["type"], json!(2));
        assert!(logs[0]["params"]["message"]
            .as_str()
            .unwrap()
            .contains("syntax error"));
        // A syntax error is normal while typing, so nothing pops up.
        assert!(sent(&messages, "window/showMessage").is_empty());
    }

    #[test]
    fn the_editors_language_name_wins_over_the_file_extension() {
        // A .txt file that the user told the editor to treat as Rust.
        let uri = "file:///nowhere/notes.txt";
        let (_, messages) = talk(&[open(uri, "rust", "fn main(){}\n"), format(1, uri)]);
        assert_eq!(
            answer(&messages, 1)["result"][0]["newText"],
            json!("fn main() {}\n")
        );
    }

    #[test]
    fn unknown_language_names_fall_back_to_the_file_extension() {
        let uri = "file:///nowhere/App.cs";
        let (_, messages) = talk(&[
            open(uri, "some-editors-name", "class A{int x;}\n"),
            format(1, uri),
        ]);
        assert_eq!(
            answer(&messages, 1)["result"][0]["newText"],
            json!("class A { int x; }\n")
        );
    }

    #[test]
    fn unknown_languages_are_left_alone() {
        let uri = "file:///nowhere/README.md";
        let (_, messages) = talk(&[open(uri, "markdown", "#  Title\n"), format(1, uri)]);
        assert_eq!(answer(&messages, 1)["result"], Value::Null);
    }

    #[test]
    fn tsx_and_jsx_use_the_right_language() {
        let tsx = "file:///nowhere/list.tsx";
        let jsx = "file:///nowhere/list.jsx";
        let source = "const a=<div className=\"x\">{b}</div>;\n";
        let (_, messages) = talk(&[
            open(tsx, "typescriptreact", source),
            open(jsx, "javascript", source),
            format(1, tsx),
            format(2, jsx),
        ]);
        let expected = json!("const a = <div className=\"x\">{b}</div>;\n");
        assert_eq!(answer(&messages, 1)["result"][0]["newText"], expected);
        assert_eq!(answer(&messages, 2)["result"][0]["newText"], expected);
    }

    #[test]
    fn uses_the_autoformat_toml_next_to_the_file() {
        let folder = temp_folder("config");
        std::fs::write(
            folder.join("autoformat.toml"),
            "[formatting]\nindent_size = 2\n\n[formatting.csharp]\nbrace_style = \"allman\"\n",
        )
        .unwrap();
        let uri = file_uri(&folder.join("app.cs"));
        let (_, messages) = talk(&[
            open(&uri, "csharp", "class A\n{\nint x;\n}\n"),
            format(1, &uri),
        ]);
        assert_eq!(
            answer(&messages, 1)["result"][0]["newText"],
            json!("class A\n{\n  int x;\n}\n")
        );
    }

    #[test]
    fn a_changed_autoformat_toml_is_used_right_away_in_both_directions() {
        let folder = temp_folder("config-change");
        let config = folder.join("autoformat.toml");
        let uri = file_uri(&folder.join("main.rs"));
        let source = "fn main() {\nlet x = 1;\n}\n";
        let mut server = Server::new();
        server.handle(&open(&uri, "rust", source));

        let mut results = Vec::new();
        for indent_size in [2, 8, 2] {
            std::fs::write(
                &config,
                format!("[formatting]\nindent_size = {}\n", indent_size),
            )
            .unwrap();
            let replies = server.handle(&format(1, &uri));
            results.push(answer(&replies, 1)["result"][0]["newText"].clone());
        }
        assert_eq!(results[0], json!("fn main() {\n  let x = 1;\n}\n"));
        assert_eq!(results[1], json!("fn main() {\n        let x = 1;\n}\n"));
        assert_eq!(results[2], results[0]);
    }

    #[test]
    fn a_broken_autoformat_toml_is_shown_once_and_again_after_it_changes() {
        let folder = temp_folder("bad-config");
        let config = folder.join("autoformat.toml");
        std::fs::write(&config, "[formatting]\nindent_style = \"tab\"\n").unwrap();
        let uri = file_uri(&folder.join("a.rs"));
        let mut server = Server::new();
        server.handle(&open(&uri, "rust", "fn a(){}\n"));

        let first = server.handle(&format(1, &uri));
        assert_eq!(answer(&first, 1)["result"], Value::Null);
        let shown = sent(&first, "window/showMessage");
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0]["params"]["type"], json!(1));
        assert!(shown[0]["params"]["message"]
            .as_str()
            .unwrap()
            .contains("indent_style"));

        // The same problem again: logged, but no second pop-up.
        let second = server.handle(&format(2, &uri));
        assert_eq!(answer(&second, 2)["result"], Value::Null);
        assert!(sent(&second, "window/showMessage").is_empty());
        assert_eq!(sent(&second, "window/logMessage").len(), 1);

        // Fixed: formatting works again.
        std::fs::write(&config, "[formatting]\nindent_style = \"tabs\"\n").unwrap();
        let third = server.handle(&format(3, &uri));
        assert_eq!(
            answer(&third, 3)["result"][0]["newText"],
            json!("fn a() {}\n")
        );

        // Broken again: shown again.
        std::fs::write(&config, "[formatting]\nindent_style = \"tab\"\n").unwrap();
        let fourth = server.handle(&format(4, &uri));
        assert_eq!(sent(&fourth, "window/showMessage").len(), 1);
    }

    #[test]
    fn unsaved_documents_use_the_project_folders_config() {
        let folder = temp_folder("untitled");
        std::fs::write(
            folder.join("autoformat.toml"),
            "[formatting]\nindent_size = 2\n",
        )
        .unwrap();
        let folder_uri = file_uri(&folder);
        let uri = "untitled:Untitled-1";
        for initialize_params in [
            json!({ "workspaceFolders": [{ "uri": folder_uri, "name": "project" }] }),
            json!({ "rootUri": folder_uri }),
            json!({ "rootPath": folder.to_string_lossy() }),
        ] {
            let (_, messages) = talk(&[
                request(1, "initialize", initialize_params),
                open(uri, "rust", "fn main() {\nlet x = 1;\n}\n"),
                format(2, uri),
            ]);
            assert_eq!(
                answer(&messages, 2)["result"][0]["newText"],
                json!("fn main() {\n  let x = 1;\n}\n")
            );
        }
    }

    #[test]
    fn shutdown_then_exit_ends_with_success() {
        let (code, messages) = talk(&[
            request(1, "initialize", json!({})),
            request(2, "shutdown", Value::Null),
            notification("exit", Value::Null),
        ]);
        assert_eq!(code, 0);
        assert_eq!(answer(&messages, 2)["result"], Value::Null);
        assert!(answer(&messages, 2).get("error").is_none());
    }

    #[test]
    fn exit_without_shutdown_and_a_closed_connection_are_failures() {
        let (code, _) = talk(&[notification("exit", Value::Null)]);
        assert_eq!(code, 1);
        let (code, _) = talk(&[request(1, "initialize", json!({}))]);
        assert_eq!(code, 1);
    }

    #[test]
    fn requests_after_shutdown_are_refused() {
        let uri = "file:///nowhere/main.rs";
        let (_, messages) = talk(&[
            open(uri, "rust", "fn main(){}\n"),
            request(1, "shutdown", Value::Null),
            format(2, uri),
        ]);
        assert_eq!(
            answer(&messages, 2)["error"]["code"],
            json!(INVALID_REQUEST)
        );
    }

    #[test]
    fn unknown_requests_get_an_error_and_unknown_notifications_are_ignored() {
        let (_, messages) = talk(&[
            notification("$/setTrace", json!({ "value": "off" })),
            notification(
                "workspace/didChangeConfiguration",
                json!({ "settings": {} }),
            ),
            request(7, "textDocument/hover", json!({})),
            json!({ "jsonrpc": "2.0", "id": 99, "result": null }),
        ]);
        assert_eq!(messages.len(), 1);
        assert_eq!(
            answer(&messages, 7)["error"]["code"],
            json!(METHOD_NOT_FOUND)
        );
    }

    #[test]
    fn request_ids_can_be_text() {
        let message = json!({ "jsonrpc": "2.0", "id": "abc", "method": "shutdown" });
        let (_, messages) = talk(&[message]);
        assert_eq!(messages[0]["id"], json!("abc"));
        assert_eq!(messages[0]["result"], Value::Null);
    }

    #[test]
    fn invalid_json_gets_an_error_and_the_next_message_still_works() {
        let mut bytes = b"Content-Length: 5\r\n\r\n{oops".to_vec();
        bytes.extend(framed(&json!([1, 2])));
        bytes.extend(framed(&request(1, "shutdown", Value::Null)));
        let mut input = std::io::Cursor::new(bytes);
        let mut output = Vec::new();
        serve(&mut input, &mut output);
        let messages = unframed(&output);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0]["error"]["code"], json!(PARSE_ERROR));
        assert_eq!(messages[0]["id"], Value::Null);
        assert_eq!(messages[1]["error"]["code"], json!(INVALID_REQUEST));
        assert_eq!(messages[2]["result"], Value::Null);
    }

    #[test]
    fn reads_headers_in_any_case_and_skips_other_headers() {
        let body = request(1, "shutdown", Value::Null).to_string();
        let text = format!(
            "content-type: application/vscode-jsonrpc; charset=utf-8\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let mut input = std::io::Cursor::new(text.into_bytes());
        let mut output = Vec::new();
        serve(&mut input, &mut output);
        assert_eq!(unframed(&output).len(), 1);
    }

    #[test]
    fn damaged_input_stops_the_server() {
        for text in [
            "Content-Type: x\r\n\r\n{}",
            "Content-Length: many\r\n\r\n{}",
            "Content-Length: 50\r\n\r\n{}",
            "Content-Length: 2\r\n",
            "Content-Length: 99999999999\r\n\r\n{}",
        ] {
            let mut input = std::io::Cursor::new(text.as_bytes().to_vec());
            let mut output = Vec::new();
            assert_eq!(serve(&mut input, &mut output), 1, "{:?}", text);
            assert!(output.is_empty(), "{:?}", text);
        }
    }

    #[test]
    fn answers_carry_the_exact_byte_length() {
        let uri = "file:///nowhere/a.js";
        let mut bytes = framed(&open(uri, "javascript", "let a='å∫😀'\n"));
        bytes.extend(framed(&format(1, uri)));
        let mut input = std::io::Cursor::new(bytes);
        let mut output = Vec::new();
        serve(&mut input, &mut output);
        let text = String::from_utf8(output).unwrap();
        let (header, body) = text.split_once("\r\n\r\n").unwrap();
        assert_eq!(header, format!("Content-Length: {}", body.len()));
        let message: Value = serde_json::from_str(body).unwrap();
        assert_eq!(message["result"][0]["newText"], json!("let a = 'å∫😀'\n"));
    }

    #[test]
    fn finds_the_end_of_a_document() {
        assert_eq!(end_position(""), (0, 0));
        assert_eq!(end_position("abc"), (0, 3));
        assert_eq!(end_position("abc\n"), (1, 0));
        assert_eq!(end_position("a\nbc"), (1, 2));
        assert_eq!(end_position("a\r\nb\r\n"), (2, 0));
        assert_eq!(end_position("a\rb"), (1, 1));
        assert_eq!(end_position("a\r\rb"), (2, 1));
        assert_eq!(end_position("\n\n"), (2, 0));
        // å is one UTF-16 unit, 😀 is two.
        assert_eq!(end_position("å😀"), (0, 3));
    }

    #[test]
    fn reads_file_addresses() {
        let unix = |uri: &str| file_uri_to_path(uri, false);
        assert_eq!(
            unix("file:///home/me/main.rs").as_deref(),
            Some("/home/me/main.rs")
        );
        assert_eq!(
            unix("file:///home/me/a%20b%23c.rs").as_deref(),
            Some("/home/me/a b#c.rs")
        );
        assert_eq!(
            unix("file:///home/%C3%A5/a.rs").as_deref(),
            Some("/home/å/a.rs")
        );
        assert_eq!(
            unix("FILE://localhost/tmp/a.rs").as_deref(),
            Some("/tmp/a.rs")
        );
        assert_eq!(
            unix("file:///tmp/a.rs?x=1#top").as_deref(),
            Some("/tmp/a.rs")
        );
        assert_eq!(unix("file://server/share/a.rs"), None);
        assert_eq!(unix("file:///tmp/%zz.rs"), None);
        assert_eq!(unix("file:///tmp/%2"), None);
        assert_eq!(unix("file:///tmp/%FF.rs"), None);
        assert_eq!(unix("file://"), None);
        assert_eq!(unix("file"), None);
        assert_eq!(unix("untitled:Untitled-1"), None);
        assert_eq!(unix("https://example.com/a.rs"), None);
        // Text that is not plain ASCII right where the scheme ends must not panic.
        assert_eq!(unix("fileå//x"), None);

        let windows = |uri: &str| file_uri_to_path(uri, true);
        assert_eq!(
            windows("file:///c%3A/Users/me/a%20b.rs").as_deref(),
            Some(r"c:\Users\me\a b.rs")
        );
        assert_eq!(
            windows("file:///C:/Users/me/a.rs").as_deref(),
            Some(r"C:\Users\me\a.rs")
        );
        assert_eq!(
            windows("file://server/share/a.rs").as_deref(),
            Some(r"\\server\share\a.rs")
        );
    }

    #[test]
    fn knows_the_language_names_editors_use() {
        assert_eq!(language_name("rust"), Some("rust"));
        assert_eq!(language_name("Rust"), Some("rust"));
        assert_eq!(language_name("javascriptreact"), Some("javascript"));
        assert_eq!(language_name("typescriptreact"), Some("tsx"));
        assert_eq!(language_name("cs"), Some("csharp"));
        assert_eq!(language_name("ps1"), Some("powershell"));
        assert_eq!(language_name("python"), None);
        assert_eq!(language_name(""), None);
        // Every name it gives is a language that exists.
        let languages = Languages::new();
        for id in [
            "rust",
            "toml",
            "javascript",
            "typescript",
            "tsx",
            "csharp",
            "powershell",
        ] {
            assert!(
                languages.by_name(language_name(id).unwrap()).is_some(),
                "{}",
                id
            );
        }
    }
}
