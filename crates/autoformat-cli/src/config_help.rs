//! Help while `autoformat.toml` is edited: marks on mistakes, suggestions while typing,
//! and an explanation for each setting.
//!
//! Everything here takes the text of the file (and a place in it) and answers in the
//! shapes the Language Server Protocol uses. `lsp.rs` sends the answers to the editor.
//!
//! Marks come from the real TOML reader in `autoformat-config`. Suggestions and
//! explanations must also work in a half-written file, which no TOML reader can make
//! sense of, so they look at the lines themselves: `[table]` lines and `name = value`
//! lines. Names, allowed values and texts all come from the one list of settings.

use std::path::Path;

use autoformat_config::{
    check_config, find_setting, CONFIG_FILE_NAME, LANGUAGE_SECTIONS, SETTINGS,
};
use serde_json::{json, Value};

/// The characters that make an editor ask for suggestions right away.
pub const SUGGESTION_TRIGGERS: &[&str] = &["[", ".", "=", "\"", "'"];

/// `DiagnosticSeverity.Error` in the protocol.
const SEVERITY_ERROR: u8 = 1;

/// `CompletionItemKind` values in the protocol. They only pick the small picture an
/// editor shows next to a suggestion.
const KIND_TABLE: u8 = 9;
const KIND_SETTING: u8 = 10;
const KIND_VALUE: u8 = 12;

/// Only files with this exact name get help. Other TOML files are left alone.
pub fn is_config_file(path: &Path) -> bool {
    match path.file_name() {
        Some(name) => name == CONFIG_FILE_NAME,
        None => false,
    }
}

/// One line of the text, without its line break.
struct Line<'a> {
    /// Where the line starts, as a byte position in the whole text.
    start: usize,
    text: &'a str,
}

/// Splits the text into lines the way editors count them: a line ends at `\n`, `\r\n`
/// or `\r`.
fn split_lines(text: &str) -> Vec<Line<'_>> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\n' || bytes[index] == b'\r' {
            lines.push(Line {
                start,
                text: &text[start..index],
            });
            if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                index += 1;
            }
            start = index + 1;
        }
        index += 1;
    }
    lines.push(Line {
        start,
        text: &text[start..],
    });
    lines
}

/// Turns a byte position in the whole text into the editor's (line, character). Both
/// start at 0, and the character is counted in UTF-16 units, as the protocol asks.
fn position_of(lines: &[Line], offset: usize) -> (usize, usize) {
    let mut number = 0;
    for (index, line) in lines.iter().enumerate() {
        if line.start <= offset {
            number = index;
        }
    }
    let line = &lines[number];
    let end = (offset - line.start).min(line.text.len());
    let before = line.text.get(..end).unwrap_or(line.text);
    (number, utf16_length(before))
}

fn utf16_length(text: &str) -> usize {
    text.encode_utf16().count()
}

/// Turns the editor's character number in a line into a byte position in that line.
fn byte_index(line: &str, character: usize) -> usize {
    let mut units = 0;
    for (index, current) in line.char_indices() {
        if units >= character {
            return index;
        }
        units += current.len_utf16();
    }
    line.len()
}

/// A range inside one line, from byte position `start` to `end` of that line.
fn range_in_line(line_number: usize, line: &str, start: usize, end: usize) -> Value {
    json!({
        "start": { "line": line_number, "character": utf16_length(&line[..start]) },
        "end": { "line": line_number, "character": utf16_length(&line[..end]) },
    })
}

/// The marks for every mistake in the text, as the protocol's diagnostics.
pub fn marks(text: &str) -> Vec<Value> {
    let lines = split_lines(text);
    let (_, mistakes) = check_config(text);
    let mut marks = Vec::new();
    for mistake in mistakes {
        let (start_line, start_character) = position_of(&lines, mistake.start);
        let (end_line, end_character) = position_of(&lines, mistake.end);
        marks.push(json!({
            "range": {
                "start": { "line": start_line, "character": start_character },
                "end": { "line": end_line, "character": end_character },
            },
            "severity": SEVERITY_ERROR,
            "source": "autoformat",
            "message": mistake.message,
        }));
    }
    marks
}

/// The name in a `[table]` line, for example `formatting.csharp`. `None` for other lines.
fn table_name(line: &str) -> Option<&str> {
    let line = line.trim_start();
    if !line.starts_with('[') {
        return None;
    }
    let name = line.trim_start_matches('[');
    let end = name.find([']', '#']).unwrap_or(name.len());
    Some(name[..end].trim())
}

/// The setting name in the text before a `=`. Only the last part counts, so
/// `csharp.brace_style` is about `brace_style`.
fn setting_name(key: &str) -> &str {
    let name = key.trim().rsplit('.').next().unwrap_or("");
    name.trim().trim_matches(['"', '\''])
}

/// True when the text ends inside a comment: there is a `#` outside quotes.
fn in_comment(text: &str) -> bool {
    let mut quote: Option<char> = None;
    for current in text.chars() {
        match quote {
            Some(open) if current == open => quote = None,
            Some(_) => {}
            None if current == '"' || current == '\'' => quote = Some(current),
            None if current == '#' => return true,
            None => {}
        }
    }
    false
}

/// How much of the text after the cursor still belongs to the word being typed. A
/// suggestion replaces that too, so picking one in the middle of a word leaves no rest.
fn rest_of_word(after: &str) -> usize {
    let stop = |current: char| current.is_whitespace() || "#]=".contains(current);
    after.find(stop).unwrap_or(after.len())
}

/// What to suggest at this place: table names after `[`, the allowed values after `=`,
/// and otherwise the settings that are not in this table yet.
pub fn suggestions(text: &str, line_number: usize, character: usize) -> Vec<Value> {
    let lines = split_lines(text);
    let line = match lines.get(line_number) {
        Some(line) => line.text,
        None => return Vec::new(),
    };
    let cursor = byte_index(line, character);
    let before = &line[..cursor];
    let after = &line[cursor..];
    if in_comment(before) {
        return Vec::new();
    }
    // Where the first word of the line starts.
    let word_start = before.len() - before.trim_start().len();
    let replace_end = cursor + rest_of_word(after);

    if before[word_start..].starts_with('[') {
        let name_start = before.len()
            - before
                .trim_start()
                .trim_start_matches('[')
                .trim_start()
                .len();
        let range = range_in_line(line_number, line, name_start, replace_end);
        return table_suggestions(&lines, line_number, after, range);
    }
    if let Some(equals) = before.find('=') {
        let value = &before[equals + 1..];
        let value_start = before.len() - value.trim_start().len();
        // After a finished value and a space there is nothing more to suggest.
        if before[value_start..].contains(char::is_whitespace) {
            return Vec::new();
        }
        let range = range_in_line(line_number, line, value_start, replace_end);
        return value_suggestions(setting_name(&before[..equals]), range);
    }
    let range = range_in_line(line_number, line, word_start, replace_end);
    name_suggestions(&lines, line_number, after, range)
}

/// `formatting` and `formatting.<language>`, without the tables the file already has.
fn table_suggestions(lines: &[Line], line_number: usize, after: &str, range: Value) -> Vec<Value> {
    let mut names = vec!["formatting".to_string()];
    for language in LANGUAGE_SECTIONS {
        names.push(format!("formatting.{}", language));
    }
    // Editors often type the closing `]` themselves. Add it only when it is missing.
    let closing = if after.contains(']') { "" } else { "]" };

    let mut items = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let mut already_written = false;
        for (other_number, other) in lines.iter().enumerate() {
            if other_number != line_number && table_name(other.text) == Some(name.as_str()) {
                already_written = true;
            }
        }
        if already_written {
            continue;
        }
        let detail = match name.strip_prefix("formatting.") {
            Some(language) => format!("Settings for {} files only", language),
            None => "Settings for every language".to_string(),
        };
        items.push(json!({
            "label": name,
            "kind": KIND_TABLE,
            "detail": detail,
            "sortText": format!("{:02}", index),
            "filterText": name,
            "textEdit": { "range": range, "newText": format!("{}{}", name, closing) },
        }));
    }
    items
}

/// The values the list of settings allows for this setting.
fn value_suggestions(name: &str, range: Value) -> Vec<Value> {
    let setting = match find_setting(name) {
        Some(setting) => setting,
        None => return Vec::new(),
    };
    let values = setting.values();
    let mut items = Vec::new();
    for (index, value) in values.iter().enumerate() {
        // A number setting offers only its default, so it also says what else is allowed.
        let detail = if values.len() == 1 {
            format!("default. Allowed: {}", setting.allowed())
        } else if value == setting.default {
            "default".to_string()
        } else {
            String::new()
        };
        items.push(json!({
            "label": value,
            "kind": KIND_VALUE,
            "detail": detail,
            "sortText": format!("{:02}", index),
            "filterText": value,
            "textEdit": { "range": range, "newText": value },
        }));
    }
    items
}

/// The settings that are not written in this table yet. Outside a table, the only thing
/// to write is `[formatting]`.
fn name_suggestions(lines: &[Line], line_number: usize, after: &str, range: Value) -> Vec<Value> {
    // The table this line is in starts at the closest `[table]` line above it.
    let mut table_line = None;
    for number in (0..line_number).rev() {
        if table_name(lines[number].text).is_some() {
            table_line = Some(number);
            break;
        }
    }
    let table_line = match table_line {
        Some(number) => number,
        None => return first_table_suggestion(lines, range),
    };
    let table = table_name(lines[table_line].text).unwrap_or("");
    let is_settings_table = match table.strip_prefix("formatting.") {
        Some(language) => LANGUAGE_SECTIONS.contains(&language),
        None => table == "formatting",
    };
    if !is_settings_table {
        return Vec::new();
    }

    // The settings on the other lines of this table, up to the next `[table]` line.
    let mut written = Vec::new();
    for (number, line) in lines.iter().enumerate().skip(table_line + 1) {
        if table_name(line.text).is_some() {
            break;
        }
        if let (true, Some((key, _))) = (number != line_number, line.text.split_once('=')) {
            written.push(setting_name(key));
        }
    }

    // When the line already has its `=`, only the name is replaced.
    let ending = if after.contains('=') { "" } else { " = " };
    let mut items = Vec::new();
    for (index, setting) in SETTINGS.iter().enumerate() {
        if written.contains(&setting.name) {
            continue;
        }
        items.push(json!({
            "label": setting.name,
            "kind": KIND_SETTING,
            "detail": setting.allowed(),
            "documentation": { "kind": "markdown", "value": setting.help() },
            "sortText": format!("{:02}", index),
            "filterText": setting.name,
            "textEdit": { "range": range, "newText": format!("{}{}", setting.name, ending) },
        }));
    }
    items
}

/// `[formatting]`, for a file that does not have it yet.
fn first_table_suggestion(lines: &[Line], range: Value) -> Vec<Value> {
    for line in lines {
        if table_name(line.text) == Some("formatting") {
            return Vec::new();
        }
    }
    vec![json!({
        "label": "[formatting]",
        "kind": KIND_TABLE,
        "detail": "Settings for every language",
        "filterText": "formatting",
        "textEdit": { "range": range, "newText": "[formatting]" },
    })]
}

/// The explanation for the setting name at this place, as the protocol's hover answer.
/// `None` when the place is not on the name of a setting.
pub fn explanation(text: &str, line_number: usize, character: usize) -> Option<Value> {
    let lines = split_lines(text);
    let line = lines.get(line_number)?.text;
    let cursor = byte_index(line, character);
    if in_comment(&line[..cursor]) {
        return None;
    }
    let key = &line[..line.find('=')?];
    // The name is the last word before the `=`.
    let name_end = key.trim_end().len();
    let name_start = match key[..name_end].rfind(['.', ' ', '\t']) {
        Some(index) => index + 1,
        None => 0,
    };
    if cursor < name_start || cursor > name_end {
        return None;
    }
    let setting = find_setting(setting_name(&key[name_start..name_end]))?;
    Some(json!({
        "contents": {
            "kind": "markdown",
            "value": format!("**{}**\n\n{}", setting.name, setting.help()),
        },
        "range": range_in_line(line_number, line, name_start, name_end),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The labels of the suggestions at the place marked with `|` in `text`.
    fn suggested(text: &str) -> Vec<String> {
        suggested_items(text)
            .iter()
            .map(|item| item["label"].as_str().unwrap().to_string())
            .collect()
    }

    fn suggested_items(text: &str) -> Vec<Value> {
        let (line, character, text) = cursor_place(text);
        suggestions(&text, line, character)
    }

    /// Takes the `|` out of the text and says where it was.
    fn cursor_place(text: &str) -> (usize, usize, String) {
        let offset = text.find('|').expect("the text needs a | for the cursor");
        let clean = text.replacen('|', "", 1);
        let (line, character) = position_of(&split_lines(&clean), offset);
        (line, character, clean)
    }

    /// What the text looks like after the suggestion with this label is picked.
    fn pick(text: &str, label: &str) -> String {
        let (_, _, clean) = cursor_place(text);
        for item in suggested_items(text) {
            if item["label"] == json!(label) {
                let range = &item["textEdit"]["range"];
                let line_number = range["start"]["line"].as_u64().unwrap() as usize;
                assert_eq!(range["end"]["line"], range["start"]["line"]);
                let mut lines: Vec<String> = clean.split('\n').map(str::to_string).collect();
                let line = &lines[line_number];
                let start =
                    byte_index(line, range["start"]["character"].as_u64().unwrap() as usize);
                let end = byte_index(line, range["end"]["character"].as_u64().unwrap() as usize);
                let new_text = item["textEdit"]["newText"].as_str().unwrap();
                lines[line_number] = format!("{}{}{}", &line[..start], new_text, &line[end..]);
                return lines.join("\n");
            }
        }
        panic!("`{}` is not suggested in {:?}", label, text);
    }

    fn setting_names() -> Vec<String> {
        SETTINGS
            .iter()
            .map(|setting| setting.name.to_string())
            .collect()
    }

    #[test]
    fn only_files_named_autoformat_toml_get_help() {
        assert!(is_config_file(Path::new("/project/autoformat.toml")));
        assert!(is_config_file(Path::new("autoformat.toml")));
        assert!(!is_config_file(Path::new("/project/Cargo.toml")));
        assert!(!is_config_file(Path::new("/project/my-autoformat.toml")));
        assert!(!is_config_file(Path::new(
            "/project/autoformat.toml/main.rs"
        )));
        assert!(!is_config_file(Path::new("untitled")));
    }

    #[test]
    fn marks_say_line_and_character_of_each_mistake() {
        let found = marks("[formatting]\nindent_size = 99\n\n[formatting.python]\n");
        assert_eq!(found.len(), 2);
        assert_eq!(
            found[0],
            json!({
                "range": {
                    "start": { "line": 1, "character": 14 },
                    "end": { "line": 1, "character": 16 },
                },
                "severity": 1,
                "source": "autoformat",
                "message": "`indent_size` must be a whole number from 1 to 16, found 99",
            })
        );
        assert_eq!(
            found[1]["range"],
            json!({
                "start": { "line": 3, "character": 12 },
                "end": { "line": 3, "character": 18 },
            })
        );
        assert!(marks("[formatting]\nindent_size = 2\n").is_empty());
    }

    #[test]
    fn marks_count_lines_and_characters_the_way_editors_do() {
        // Windows line endings, and characters before the mistake that are one (å) and
        // two (😀) UTF-16 units long.
        let found = marks("# å😀\r\n[formatting]\r\n\"å😀\" = 1\r\nindent_size = 0\r\n");
        assert_eq!(found.len(), 2);
        assert_eq!(
            found[0]["range"]["start"],
            json!({ "line": 2, "character": 0 })
        );
        assert_eq!(
            found[0]["range"]["end"],
            json!({ "line": 2, "character": 5 })
        );
        assert_eq!(
            found[1]["range"]["start"],
            json!({ "line": 3, "character": 14 })
        );
        // A line can also end with a lone `\r`. Editors count that as a line, so the
        // places after it must be counted the same way.
        let lines = split_lines("a\rbc\r\nd");
        assert_eq!(lines.len(), 3);
        assert_eq!(position_of(&lines, 3), (1, 1));
        assert_eq!(position_of(&lines, 6), (2, 0));
        assert_eq!(position_of(&lines, 7), (2, 1));
    }

    #[test]
    fn suggests_the_settings_that_are_not_in_the_table_yet() {
        assert_eq!(suggested("[formatting]\n|"), setting_names());
        assert_eq!(suggested("[formatting.csharp]\nbra|"), setting_names());

        let left = suggested("[formatting]\nindent_size = 2\n|\nbrace_style = \"gnu\"\n\n[formatting.rust]\ncolumn_width = 80\n");
        assert!(!left.contains(&"indent_size".to_string()));
        assert!(!left.contains(&"brace_style".to_string()));
        // `column_width` is only written in another table.
        assert!(left.contains(&"column_width".to_string()));
        assert_eq!(left.len(), SETTINGS.len() - 2);

        // The line being typed does not count as already written.
        assert!(suggested("[formatting]\nindent_size| = 2\n").contains(&"indent_size".to_string()));
    }

    #[test]
    fn a_suggested_setting_comes_with_its_explanation() {
        let items = suggested_items("[formatting]\n|");
        let brace_style = items
            .iter()
            .find(|item| item["label"] == json!("brace_style"))
            .unwrap();
        assert_eq!(
            brace_style["detail"],
            json!("\"same_line\", \"allman\" or \"gnu\"")
        );
        let help = brace_style["documentation"]["value"].as_str().unwrap();
        assert!(help.contains("Default: \"same_line\"."), "{}", help);
    }

    #[test]
    fn picking_a_setting_writes_the_name_and_the_equals_sign() {
        assert_eq!(
            pick("[formatting]\nbra|", "brace_style"),
            "[formatting]\nbrace_style = "
        );
        assert_eq!(
            pick("[formatting]\n  inde|", "indent_size"),
            "[formatting]\n  indent_size = "
        );
        // The rest of a half-changed name is replaced, and an `=` that is there is kept.
        assert_eq!(
            pick("[formatting]\ninde|nt_siz = 2", "indent_style"),
            "[formatting]\nindent_style = 2"
        );
    }

    #[test]
    fn suggests_the_allowed_values_after_the_equals_sign() {
        assert_eq!(
            suggested("[formatting]\nbrace_style = |"),
            ["\"same_line\"", "\"allman\"", "\"gnu\""]
        );
        assert_eq!(
            suggested("[formatting]\nbrace_style=|"),
            ["\"same_line\"", "\"allman\"", "\"gnu\""]
        );
        assert_eq!(
            suggested("[formatting]\nindent_style = \"t|"),
            ["\"spaces\"", "\"tabs\""]
        );
        assert_eq!(
            suggested("[formatting.rust]\nfinal_newline = |"),
            ["true", "false"]
        );
        assert_eq!(
            suggested("[formatting]\ncsharp.line_ending = |"),
            ["\"lf\"", "\"crlf\""]
        );

        let items = suggested_items("[formatting]\nindent_size = |");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["label"], json!("4"));
        assert_eq!(
            items[0]["detail"],
            json!("default. Allowed: a whole number from 1 to 16")
        );
        let items = suggested_items("[formatting]\nline_ending = |");
        assert_eq!(items[0]["detail"], json!("default"));
        assert_eq!(items[1]["detail"], json!(""));

        assert!(suggested("[formatting]\nquote_style = |").is_empty());
        assert!(suggested("[formatting]\nbrace_style = \"gnu\" |").is_empty());
    }

    #[test]
    fn picking_a_value_replaces_what_was_typed_of_it() {
        assert_eq!(
            pick("[formatting]\nbrace_style = |", "\"gnu\""),
            "[formatting]\nbrace_style = \"gnu\""
        );
        assert_eq!(
            pick("[formatting]\nbrace_style = \"al|", "\"allman\""),
            "[formatting]\nbrace_style = \"allman\""
        );
        // The editor already typed the closing quote.
        assert_eq!(
            pick("[formatting]\nbrace_style = \"|\"", "\"allman\""),
            "[formatting]\nbrace_style = \"allman\""
        );
        assert_eq!(
            pick(
                "[formatting]\nbrace_style = \"g|nu\" # note",
                "\"same_line\""
            ),
            "[formatting]\nbrace_style = \"same_line\" # note"
        );
        assert_eq!(
            pick("[formatting]\nfinal_newline =t|", "true"),
            "[formatting]\nfinal_newline =true"
        );
    }

    #[test]
    fn suggests_table_names_after_a_bracket() {
        let all = suggested("[|");
        assert_eq!(all[0], "formatting");
        assert_eq!(all[1], "formatting.rust");
        assert_eq!(all.len(), 1 + LANGUAGE_SECTIONS.len());

        let left =
            suggested("[formatting]\nindent_size = 2\n\n[formatting.csharp]\n\n[formatting.|");
        assert!(!left.contains(&"formatting".to_string()));
        assert!(!left.contains(&"formatting.csharp".to_string()));
        assert!(left.contains(&"formatting.kotlin".to_string()));

        assert_eq!(
            pick("[formatting]\n\n[formatting.cs|", "formatting.csharp"),
            "[formatting]\n\n[formatting.csharp]"
        );
        // The editor already typed the closing bracket.
        assert_eq!(pick("[form|]", "formatting.java"), "[formatting.java]");
        assert_eq!(pick("[ |", "formatting"), "[ formatting]");
    }

    #[test]
    fn suggests_the_first_table_in_a_file_without_one() {
        assert_eq!(suggested("|"), ["[formatting]"]);
        assert_eq!(
            pick("# my settings\nfor|", "[formatting]"),
            "# my settings\n[formatting]"
        );
        assert!(suggested("|\n[formatting]\n").is_empty());
    }

    #[test]
    fn suggests_nothing_in_comments_unknown_tables_and_outside_the_text() {
        assert!(suggested("[formatting]\n# inde|").is_empty());
        assert!(suggested("[formatting]\nbrace_style = \"gnu\" # or |").is_empty());
        assert!(suggested("[package]\n|").is_empty());
        assert!(suggested("[formatting.python]\n|").is_empty());
        assert!(suggestions("[formatting]\n", 9, 0).is_empty());
        // A place after the end of a line counts as the end of that line.
        assert_eq!(
            suggestions("[formatting]\nfinal_newline = ", 1, 99).len(),
            2
        );
    }

    #[test]
    fn explains_the_setting_under_the_cursor() {
        let text = "[formatting]\n  brace_style = \"allman\" # braces\n\n[formatting.csharp]\nindent_size=2\n";
        let answer = explanation(text, 1, 5).unwrap();
        assert_eq!(answer["contents"]["kind"], json!("markdown"));
        assert_eq!(
            answer["contents"]["value"],
            json!(format!(
                "**brace_style**\n\n{}",
                find_setting("brace_style").unwrap().help()
            ))
        );
        assert_eq!(
            answer["range"],
            json!({ "start": { "line": 1, "character": 2 }, "end": { "line": 1, "character": 13 } })
        );
        // The first and the last place of the name both count.
        assert!(explanation(text, 1, 2).is_some());
        assert!(explanation(text, 1, 13).is_some());
        assert!(explanation(text, 4, 0).unwrap()["contents"]["value"]
            .as_str()
            .unwrap()
            .starts_with("**indent_size**"));

        // Not on a name: the spaces before it, the value, a table line, an empty line.
        assert!(explanation(text, 1, 1).is_none());
        assert!(explanation(text, 1, 20).is_none());
        assert!(explanation(text, 0, 3).is_none());
        assert!(explanation(text, 2, 0).is_none());
        assert!(explanation(text, 99, 0).is_none());
        // Unknown settings and comments have no explanation.
        assert!(explanation("quote_style = 1", 0, 3).is_none());
        assert!(explanation("# indent_size = 2", 0, 5).is_none());
        // The last part of a dotted name is what counts.
        assert!(explanation("csharp.final_newline = true", 0, 10).is_some());
        assert!(explanation("csharp.final_newline = true", 0, 3).is_none());
    }
}
