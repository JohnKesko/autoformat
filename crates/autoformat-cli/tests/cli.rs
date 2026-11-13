//! Tests that run the real `autoformat` binary.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_autoformat"))
}

/// A fresh temporary directory for one test.
fn temp_dir(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("autoformat-cli-{}-{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

fn run_with_stdin(arguments: &[&str], input: &str) -> (i32, String, String) {
    let mut child = binary()
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The program may exit before reading its input (for example on a configuration
    // error), so a failed write is expected in some tests.
    let _ = child.stdin.take().unwrap().write_all(input.as_bytes());
    let output = child.wait_with_output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn formats_a_file_in_place() {
    let directory = temp_dir("in-place");
    let file = directory.join("main.rs");
    std::fs::write(&file, "fn main(){let x=1;}\n").unwrap();
    let status = binary()
        .args(["format", file.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "fn main() { let x = 1; }\n"
    );
}

#[test]
fn formats_directories_and_skips_unknown_files() {
    let directory = temp_dir("directory");
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(directory.join("src/a.ts"), "let a=1\n").unwrap();
    std::fs::write(directory.join("b.toml"), "x=1\n").unwrap();
    std::fs::write(directory.join("notes.md"), "# keep  me\n").unwrap();
    let status = binary()
        .args(["format", directory.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        std::fs::read_to_string(directory.join("src/a.ts")).unwrap(),
        "let a = 1\n"
    );
    assert_eq!(
        std::fs::read_to_string(directory.join("b.toml")).unwrap(),
        "x = 1\n"
    );
    assert_eq!(
        std::fs::read_to_string(directory.join("notes.md")).unwrap(),
        "# keep  me\n"
    );
}

#[test]
fn check_mode_reports_without_writing() {
    let directory = temp_dir("check");
    let file = directory.join("a.js");
    std::fs::write(&file, "let a=1\n").unwrap();
    let output = binary()
        .args(["format", "--check", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "let a=1\n");

    std::fs::write(&file, "let a = 1\n").unwrap();
    let output = binary()
        .args(["format", "--check", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn stdin_mode_uses_the_file_name_and_config() {
    let directory = temp_dir("stdin");
    std::fs::write(
        directory.join("autoformat.toml"),
        "[formatting]\nindent_size = 2\n\n[formatting.csharp]\nbrace_style = \"allman\"\n",
    )
    .unwrap();
    let path = directory.join("app.cs");
    let (code, stdout, _) = run_with_stdin(
        &[
            "format",
            "--stdin",
            "--stdin-filepath",
            path.to_str().unwrap(),
        ],
        "class A\n{\nint x;\n}\n",
    );
    assert_eq!(code, 0);
    assert_eq!(stdout, "class A\n{\n  int x;\n}\n");
}

#[test]
fn stdin_mode_with_explicit_language() {
    let (code, stdout, _) = run_with_stdin(
        &["format", "--stdin", "--language", "powershell"],
        "if($a){ $b=1 }\n",
    );
    assert_eq!(code, 0);
    assert_eq!(stdout, "if ($a) { $b = 1 }\n");
}

#[test]
fn syntax_errors_fail_and_leave_the_file_alone() {
    let directory = temp_dir("syntax");
    let file = directory.join("broken.rs");
    std::fs::write(&file, "fn main( {\n").unwrap();
    let output = binary()
        .args(["format", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("syntax error"));
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "fn main( {\n");
}

#[test]
fn invalid_configuration_is_reported() {
    let directory = temp_dir("bad-config");
    std::fs::write(
        directory.join("autoformat.toml"),
        "[formatting]\nindent_style = \"tab\"\n",
    )
    .unwrap();
    let path = directory.join("a.rs");
    let (code, stdout, stderr) = run_with_stdin(
        &[
            "format",
            "--stdin",
            "--stdin-filepath",
            path.to_str().unwrap(),
        ],
        "fn a() {}\n",
    );
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("indent_style"), "{}", stderr);
}

#[test]
fn unknown_options_and_languages_are_usage_errors() {
    assert_eq!(
        binary()
            .args(["format", "--bogus", "x.rs"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        binary()
            .args(["format", "--stdin", "--language", "cobol"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        binary()
            .args(["frobnicate"])
            .output()
            .unwrap()
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn lists_languages_and_shows_config() {
    let output = binary().arg("languages").output().unwrap();
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    for name in [
        "rust",
        "toml",
        "javascript",
        "typescript",
        "tsx",
        "csharp",
        "powershell",
    ] {
        assert!(text.contains(name), "missing {} in\n{}", name, text);
    }
    let directory = temp_dir("show-config");
    let output = binary()
        .args(["config", directory.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains("indent_size"));
}

/// Wraps a message the way an editor sends it to `autoformat lsp`.
fn lsp_message(json: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", json.len(), json)
}

#[test]
fn lsp_mode_formats_documents_for_an_editor() {
    let directory = temp_dir("lsp");
    std::fs::write(
        directory.join("autoformat.toml"),
        "[formatting]\nindent_size = 2\n",
    )
    .unwrap();
    // Editors write file addresses with forward slashes on every system.
    let path = directory
        .join("main.rs")
        .to_string_lossy()
        .replace('\\', "/");
    let uri = if path.starts_with('/') {
        format!("file://{}", path)
    } else {
        format!("file:///{}", path)
    };

    let mut input = String::new();
    input.push_str(&lsp_message(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    ));
    input.push_str(&lsp_message(
        r#"{"jsonrpc":"2.0","method":"initialized","params":{}}"#,
    ));
    input.push_str(&lsp_message(&format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{}","languageId":"rust","version":1,"text":"fn main(){{\nlet x=1;\n}}\n"}}}}}}"#,
        uri
    )));
    input.push_str(&lsp_message(&format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/formatting","params":{{"textDocument":{{"uri":"{}"}},"options":{{"tabSize":4,"insertSpaces":true}}}}}}"#,
        uri
    )));
    input.push_str(&lsp_message(
        r#"{"jsonrpc":"2.0","id":3,"method":"shutdown"}"#,
    ));
    input.push_str(&lsp_message(r#"{"jsonrpc":"2.0","method":"exit"}"#));

    let (code, stdout, stderr) = run_with_stdin(&["lsp"], &input);
    assert_eq!(code, 0, "{}", stderr);
    assert!(stderr.is_empty(), "{}", stderr);
    // Three requests, three answers, and nothing else on standard output.
    assert_eq!(stdout.matches("Content-Length: ").count(), 3, "{}", stdout);
    assert!(stdout.starts_with("Content-Length: "), "{}", stdout);
    assert!(
        stdout.contains(r#""documentFormattingProvider":true"#),
        "{}",
        stdout
    );
    // The two spaces of indentation come from autoformat.toml next to the file.
    assert!(
        stdout.contains(r#""newText":"fn main() {\n  let x = 1;\n}\n""#),
        "{}",
        stdout
    );
    // The edit replaces everything, up to the end of the old text: line 3, column 0.
    assert!(stdout.contains(r#""line":3"#), "{}", stdout);
}

#[test]
fn lsp_mode_accepts_stdio_and_refuses_other_options() {
    let exit = lsp_message(r#"{"jsonrpc":"2.0","method":"exit"}"#);
    // `exit` without `shutdown` first ends with code 1, which shows the server was running.
    let (code, stdout, _) = run_with_stdin(&["lsp", "--stdio"], &exit);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());

    let (code, stdout, stderr) = run_with_stdin(&["lsp", "--port", "1234"], &exit);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("--port"), "{}", stderr);
}
