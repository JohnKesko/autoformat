//! Configuration for AutoFormat.
//!
//! AutoFormat has ten options, all in the `[formatting]` table of `autoformat.toml`:
//!
//! ```toml
//! [formatting]
//! indent_size = 4
//! indent_style = "spaces"
//! column_width = 140
//! comment_alignment = true
//! line_ending = "lf"
//! brace_style = "same_line"
//! blank_lines = true
//! space_around_operators = true
//! final_newline = true
//! trim_trailing_whitespace = true
//! continuation_indent = false
//!
//! [formatting.csharp]
//! brace_style = "allman"
//! ```
//!
//! Any option can be set again for one language in a `[formatting.<language>]` table.
//! A language uses the `[formatting]` value for every option its own table leaves out.
//!
//! Missing options use their defaults. Unknown options and invalid values are errors:
//! they are never silently ignored.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The name of the configuration file AutoFormat looks for.
pub const CONFIG_FILE_NAME: &str = "autoformat.toml";

pub const MIN_INDENT_SIZE: usize = 1;
pub const MAX_INDENT_SIZE: usize = 16;
pub const MIN_COLUMN_WIDTH: usize = 20;
pub const MAX_COLUMN_WIDTH: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndentStyle {
    Spaces,
    Tabs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

/// Where the opening brace of a code block goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BraceStyle {
    /// `void f() {` - the brace stays at the end of the line (K&R).
    SameLine,
    /// The brace goes on its own line, at the same indentation as the line before it.
    Allman,
    /// The brace goes on its own line, indented one level more; the block's contents are
    /// indented one level more than the brace.
    Gnu,
}

impl IndentStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            IndentStyle::Spaces => "spaces",
            IndentStyle::Tabs => "tabs",
        }
    }
}

impl LineEnding {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LineEnding::Lf => "lf",
            LineEnding::Crlf => "crlf",
        }
    }
}

impl BraceStyle {
    pub fn as_str(&self) -> &'static str {
        match self {
            BraceStyle::SameLine => "same_line",
            BraceStyle::Allman => "allman",
            BraceStyle::Gnu => "gnu",
        }
    }
}

/// The language names that may have their own `[formatting.<language>]` table.
pub const LANGUAGE_SECTIONS: &[&str] = &[
    "rust",
    "toml",
    "javascript",
    "typescript",
    "csharp",
    "powershell",
];

/// The complete, validated formatting configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub indent_size: usize,
    pub indent_style: IndentStyle,
    pub column_width: usize,
    pub comment_alignment: bool,
    pub line_ending: LineEnding,
    pub brace_style: BraceStyle,
    /// Put one blank line between declarations (functions, classes, methods...) when one
    /// of them spans several lines.
    pub blank_lines: bool,
    /// Put single spaces around operators such as `=`, `+` and `=>`. When false, the
    /// spacing around operators is kept as written.
    pub space_around_operators: bool,
    /// End the file with a line break. When false, the file ends right after its last
    /// character of code.
    pub final_newline: bool,
    /// Remove spaces and tabs at the end of lines (never inside strings).
    pub trim_trailing_whitespace: bool,
    /// When a statement goes on after a block in its middle (PowerShell:
    /// `... | ForEach-Object { ... } | ...`): indent that block and the rest of the
    /// statement one extra level, so the statement's first line stands out.
    pub continuation_indent: bool,
    /// Settings from `[formatting.<language>]` tables, in the order of
    /// [`LANGUAGE_SECTIONS`].
    pub language_overrides: Vec<(String, Overrides)>,
}

/// The options one `[formatting.<language>]` table sets. `None` means "use the
/// `[formatting]` value".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Overrides {
    pub indent_size: Option<usize>,
    pub indent_style: Option<IndentStyle>,
    pub column_width: Option<usize>,
    pub comment_alignment: Option<bool>,
    pub line_ending: Option<LineEnding>,
    pub brace_style: Option<BraceStyle>,
    pub blank_lines: Option<bool>,
    pub space_around_operators: Option<bool>,
    pub final_newline: Option<bool>,
    pub trim_trailing_whitespace: Option<bool>,
    pub continuation_indent: Option<bool>,
}

impl Overrides {
    /// Copies every option that is set into `config`.
    fn apply_to(&self, config: &mut Config) {
        if let Some(value) = self.indent_size {
            config.indent_size = value;
        }
        if let Some(value) = self.indent_style {
            config.indent_style = value;
        }
        if let Some(value) = self.column_width {
            config.column_width = value;
        }
        if let Some(value) = self.comment_alignment {
            config.comment_alignment = value;
        }
        if let Some(value) = self.line_ending {
            config.line_ending = value;
        }
        if let Some(value) = self.brace_style {
            config.brace_style = value;
        }
        if let Some(value) = self.blank_lines {
            config.blank_lines = value;
        }
        if let Some(value) = self.space_around_operators {
            config.space_around_operators = value;
        }
        if let Some(value) = self.final_newline {
            config.final_newline = value;
        }
        if let Some(value) = self.trim_trailing_whitespace {
            config.trim_trailing_whitespace = value;
        }
        if let Some(value) = self.continuation_indent {
            config.continuation_indent = value;
        }
    }

    /// The options that are set, as `name = value` lines in `autoformat.toml` syntax.
    fn lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if let Some(value) = self.indent_size {
            lines.push(format!("indent_size = {}", value));
        }
        if let Some(value) = self.indent_style {
            lines.push(format!("indent_style = \"{}\"", value.as_str()));
        }
        if let Some(value) = self.column_width {
            lines.push(format!("column_width = {}", value));
        }
        if let Some(value) = self.comment_alignment {
            lines.push(format!("comment_alignment = {}", value));
        }
        if let Some(value) = self.line_ending {
            lines.push(format!("line_ending = \"{}\"", value.name()));
        }
        if let Some(value) = self.brace_style {
            lines.push(format!("brace_style = \"{}\"", value.as_str()));
        }
        if let Some(value) = self.blank_lines {
            lines.push(format!("blank_lines = {}", value));
        }
        if let Some(value) = self.space_around_operators {
            lines.push(format!("space_around_operators = {}", value));
        }
        if let Some(value) = self.final_newline {
            lines.push(format!("final_newline = {}", value));
        }
        if let Some(value) = self.trim_trailing_whitespace {
            lines.push(format!("trim_trailing_whitespace = {}", value));
        }
        if let Some(value) = self.continuation_indent {
            lines.push(format!("continuation_indent = {}", value));
        }
        lines
    }
}

impl Config {
    /// The settings for one language: the `[formatting]` values with that language's own
    /// table applied on top. `language` is a name from [`LANGUAGE_SECTIONS`].
    pub fn for_language(&self, language: &str) -> Config {
        let mut config = self.clone();
        config.language_overrides = Vec::new();
        for (name, overrides) in &self.language_overrides {
            if name == language {
                overrides.apply_to(&mut config);
            }
        }
        config
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            indent_size: 4,
            indent_style: IndentStyle::Spaces,
            column_width: 140,
            comment_alignment: true,
            line_ending: LineEnding::Lf,
            brace_style: BraceStyle::SameLine,
            blank_lines: true,
            space_around_operators: true,
            final_newline: true,
            trim_trailing_whitespace: true,
            continuation_indent: false,
            language_overrides: Vec::new(),
        }
    }
}

impl fmt::Display for Config {
    /// Prints the configuration in `autoformat.toml` syntax.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "[formatting]")?;
        writeln!(f, "indent_size = {}", self.indent_size)?;
        writeln!(f, "indent_style = \"{}\"", self.indent_style.as_str())?;
        writeln!(f, "column_width = {}", self.column_width)?;
        writeln!(f, "comment_alignment = {}", self.comment_alignment)?;
        writeln!(f, "line_ending = \"{}\"", self.line_ending.name())?;
        writeln!(f, "brace_style = \"{}\"", self.brace_style.as_str())?;
        writeln!(f, "blank_lines = {}", self.blank_lines)?;
        writeln!(
            f,
            "space_around_operators = {}",
            self.space_around_operators
        )?;
        writeln!(f, "final_newline = {}", self.final_newline)?;
        writeln!(
            f,
            "trim_trailing_whitespace = {}",
            self.trim_trailing_whitespace
        )?;
        writeln!(f, "continuation_indent = {}", self.continuation_indent)?;
        for (language, overrides) in &self.language_overrides {
            let lines = overrides.lines();
            if lines.is_empty() {
                continue;
            }
            writeln!(f)?;
            writeln!(f, "[formatting.{}]", language)?;
            for line in lines {
                writeln!(f, "{}", line)?;
            }
        }
        Ok(())
    }
}

/// A problem with a configuration file. The message is meant for people.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub path: Option<PathBuf>,
    pub message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{}: {}", path.display(), self.message),
            None => write!(f, "{}: {}", CONFIG_FILE_NAME, self.message),
        }
    }
}

impl std::error::Error for ConfigError {}

/// The file as written by the user. Every field is optional so that missing options can
/// fall back to their defaults. Unknown fields are rejected.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileContents {
    formatting: Option<FormattingTable>,
}

/// `[formatting]`: the ten options, plus one optional table per language.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FormattingTable {
    indent_size: Option<i64>,
    indent_style: Option<String>,
    column_width: Option<i64>,
    comment_alignment: Option<bool>,
    line_ending: Option<String>,
    brace_style: Option<String>,
    blank_lines: Option<bool>,
    space_around_operators: Option<bool>,
    final_newline: Option<bool>,
    trim_trailing_whitespace: Option<bool>,
    continuation_indent: Option<bool>,
    rust: Option<SettingsTable>,
    toml: Option<SettingsTable>,
    javascript: Option<SettingsTable>,
    typescript: Option<SettingsTable>,
    csharp: Option<SettingsTable>,
    powershell: Option<SettingsTable>,
}

/// `[formatting.<language>]`: the same ten options.
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct SettingsTable {
    indent_size: Option<i64>,
    indent_style: Option<String>,
    column_width: Option<i64>,
    comment_alignment: Option<bool>,
    line_ending: Option<String>,
    brace_style: Option<String>,
    blank_lines: Option<bool>,
    space_around_operators: Option<bool>,
    final_newline: Option<bool>,
    trim_trailing_whitespace: Option<bool>,
    continuation_indent: Option<bool>,
}

/// Parses and validates the text of an `autoformat.toml` file.
pub fn parse_config(text: &str) -> Result<Config, ConfigError> {
    let contents: FileContents = match toml::from_str(text) {
        Ok(contents) => contents,
        Err(error) => {
            return Err(ConfigError {
                path: None,
                message: error.message().to_string() + &describe_span(text, error.span()),
            })
        }
    };

    let mut config = Config::default();
    let table = match contents.formatting {
        Some(table) => table,
        None => return Ok(config),
    };

    let global = SettingsTable {
        indent_size: table.indent_size,
        indent_style: table.indent_style,
        column_width: table.column_width,
        comment_alignment: table.comment_alignment,
        line_ending: table.line_ending,
        brace_style: table.brace_style,
        blank_lines: table.blank_lines,
        space_around_operators: table.space_around_operators,
        final_newline: table.final_newline,
        trim_trailing_whitespace: table.trim_trailing_whitespace,
        continuation_indent: table.continuation_indent,
    };
    read_settings(&global, "")?.apply_to(&mut config);

    let language_tables = [
        ("rust", table.rust),
        ("toml", table.toml),
        ("javascript", table.javascript),
        ("typescript", table.typescript),
        ("csharp", table.csharp),
        ("powershell", table.powershell),
    ];
    for (language, language_table) in language_tables {
        if let Some(language_table) = language_table {
            let prefix = format!("formatting.{}.", language);
            let overrides = read_settings(&language_table, &prefix)?;
            config
                .language_overrides
                .push((language.to_string(), overrides));
        }
    }
    Ok(config)
}

/// Checks the values of one table. `prefix` goes in front of option names in error
/// messages, so a mistake in `[formatting.csharp]` says `formatting.csharp.indent_size`.
fn read_settings(table: &SettingsTable, prefix: &str) -> Result<Overrides, ConfigError> {
    let name = |option: &str| format!("{}{}", prefix, option);
    let mut overrides = Overrides::default();
    if let Some(value) = table.indent_size {
        overrides.indent_size = Some(check_range(
            &name("indent_size"),
            value,
            MIN_INDENT_SIZE,
            MAX_INDENT_SIZE,
        )?);
    }
    if let Some(value) = table.column_width {
        overrides.column_width = Some(check_range(
            &name("column_width"),
            value,
            MIN_COLUMN_WIDTH,
            MAX_COLUMN_WIDTH,
        )?);
    }
    if let Some(value) = &table.indent_style {
        overrides.indent_style = Some(match value.as_str() {
            "spaces" => IndentStyle::Spaces,
            "tabs" => IndentStyle::Tabs,
            _ => {
                return Err(invalid_choice(
                    &name("indent_style"),
                    value,
                    "\"spaces\" or \"tabs\"",
                ))
            }
        });
    }
    if let Some(value) = &table.line_ending {
        overrides.line_ending = Some(match value.as_str() {
            "lf" => LineEnding::Lf,
            "crlf" => LineEnding::Crlf,
            _ => {
                return Err(invalid_choice(
                    &name("line_ending"),
                    value,
                    "\"lf\" or \"crlf\"",
                ))
            }
        });
    }
    if let Some(value) = &table.brace_style {
        overrides.brace_style = Some(match value.as_str() {
            "same_line" => BraceStyle::SameLine,
            "allman" => BraceStyle::Allman,
            "gnu" => BraceStyle::Gnu,
            _ => {
                return Err(invalid_choice(
                    &name("brace_style"),
                    value,
                    "\"same_line\", \"allman\" or \"gnu\"",
                ))
            }
        });
    }
    overrides.comment_alignment = table.comment_alignment;
    overrides.blank_lines = table.blank_lines;
    overrides.space_around_operators = table.space_around_operators;
    overrides.final_newline = table.final_newline;
    overrides.trim_trailing_whitespace = table.trim_trailing_whitespace;
    overrides.continuation_indent = table.continuation_indent;
    Ok(overrides)
}

/// Reads and validates one configuration file.
pub fn load_config_file(path: &Path) -> Result<Config, ConfigError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            return Err(ConfigError {
                path: Some(path.to_path_buf()),
                message: format!("cannot read the file: {}", error),
            })
        }
    };
    match parse_config(&text) {
        Ok(config) => Ok(config),
        Err(mut error) => {
            error.path = Some(path.to_path_buf());
            Err(error)
        }
    }
}

/// Looks for `autoformat.toml` in `start` (a file or a directory) and then in each parent
/// directory. Returns the first file found.
pub fn find_config_file(start: &Path) -> Option<PathBuf> {
    let mut directory: Option<&Path> = if start.is_dir() {
        Some(start)
    } else {
        start.parent()
    };
    while let Some(current) = directory {
        let candidate = current.join(CONFIG_FILE_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        directory = current.parent();
    }
    None
}

/// Finds and loads the configuration that applies to `start` (a file or directory).
/// Returns the defaults when no configuration file exists. Also returns the path of the
/// file that was used, if any.
pub fn discover_config(start: &Path) -> Result<(Config, Option<PathBuf>), ConfigError> {
    let absolute = if start.is_absolute() {
        start.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(current) => current.join(start),
            Err(_) => start.to_path_buf(),
        }
    };
    match find_config_file(&absolute) {
        Some(path) => {
            let config = load_config_file(&path)?;
            Ok((config, Some(path)))
        }
        None => Ok((Config::default(), None)),
    }
}

fn check_range(name: &str, value: i64, min: usize, max: usize) -> Result<usize, ConfigError> {
    if value < min as i64 || value > max as i64 {
        return Err(ConfigError {
            path: None,
            message: format!(
                "`{}` must be between {} and {}, found {}",
                name, min, max, value
            ),
        });
    }
    Ok(value as usize)
}

fn invalid_choice(name: &str, value: &str, expected: &str) -> ConfigError {
    ConfigError {
        path: None,
        message: format!("`{}` must be {}, found \"{}\"", name, expected, value),
    }
}

/// Turns a byte range from the TOML parser into " (line N)" for error messages.
fn describe_span(text: &str, span: Option<std::ops::Range<usize>>) -> String {
    match span {
        Some(range) => {
            let line = text[..range.start.min(text.len())].matches('\n').count() + 1;
            format!(" (line {})", line)
        }
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_file_gives_defaults() {
        assert_eq!(parse_config("").unwrap(), Config::default());
        assert_eq!(parse_config("[formatting]\n").unwrap(), Config::default());
    }

    #[test]
    fn defaults_match_the_documentation() {
        let config = Config::default();
        assert_eq!(config.indent_size, 4);
        assert_eq!(config.indent_style, IndentStyle::Spaces);
        assert_eq!(config.column_width, 140);
        assert!(config.comment_alignment);
        assert_eq!(config.line_ending, LineEnding::Lf);
        assert_eq!(config.brace_style, BraceStyle::SameLine);
        assert!(config.blank_lines);
        assert!(config.space_around_operators);
        assert!(config.final_newline);
        assert!(config.trim_trailing_whitespace);
        assert!(!config.continuation_indent);
        assert!(config.language_overrides.is_empty());
    }

    #[test]
    fn reads_every_option() {
        let text = "[formatting]\nindent_size = 2\nindent_style = \"tabs\"\ncolumn_width = 80\ncomment_alignment = false\nline_ending = \"crlf\"\nbrace_style = \"gnu\"\nblank_lines = false\nspace_around_operators = false\nfinal_newline = false\ntrim_trailing_whitespace = false\ncontinuation_indent = true\n";
        let config = parse_config(text).unwrap();
        assert!(!config.blank_lines);
        assert!(!config.space_around_operators);
        assert!(!config.final_newline);
        assert!(!config.trim_trailing_whitespace);
        assert!(config.continuation_indent);
        assert_eq!(config.indent_size, 2);
        assert_eq!(config.indent_style, IndentStyle::Tabs);
        assert_eq!(config.column_width, 80);
        assert!(!config.comment_alignment);
        assert_eq!(config.line_ending, LineEnding::Crlf);
        assert_eq!(config.brace_style, BraceStyle::Gnu);
    }

    #[test]
    fn reads_each_brace_style() {
        for (text, style) in [
            ("same_line", BraceStyle::SameLine),
            ("allman", BraceStyle::Allman),
            ("gnu", BraceStyle::Gnu),
        ] {
            let config =
                parse_config(&format!("[formatting]\nbrace_style = \"{}\"\n", text)).unwrap();
            assert_eq!(config.brace_style, style);
        }
    }

    #[test]
    fn language_brace_styles_override_the_global_one() {
        let text = "[formatting]\nbrace_style = \"same_line\"\n\n[formatting.csharp]\nbrace_style = \"allman\"\n\n[formatting.rust]\nbrace_style = \"gnu\"\n";
        let config = parse_config(text).unwrap();
        assert_eq!(
            config.for_language("csharp").brace_style,
            BraceStyle::Allman
        );
        assert_eq!(config.for_language("rust").brace_style, BraceStyle::Gnu);
        // Languages without their own table inherit the global style.
        assert_eq!(
            config.for_language("typescript").brace_style,
            BraceStyle::SameLine
        );
        assert_eq!(
            config.for_language("powershell").brace_style,
            BraceStyle::SameLine
        );
    }

    #[test]
    fn language_tables_inherit_a_non_default_global_style() {
        let text = "[formatting]\nbrace_style = \"allman\"\n\n[formatting.javascript]\nbrace_style = \"same_line\"\n\n[formatting.csharp]\n";
        let config = parse_config(text).unwrap();
        assert_eq!(
            config.for_language("javascript").brace_style,
            BraceStyle::SameLine
        );
        // An empty language table changes nothing.
        assert_eq!(
            config.for_language("csharp").brace_style,
            BraceStyle::Allman
        );
        assert_eq!(config.for_language("rust").brace_style, BraceStyle::Allman);
    }

    #[test]
    fn language_table_without_global_brace_style_uses_the_default_elsewhere() {
        let config = parse_config("[formatting.csharp]\nbrace_style = \"allman\"\n").unwrap();
        assert_eq!(
            config.for_language("csharp").brace_style,
            BraceStyle::Allman
        );
        assert_eq!(
            config.for_language("rust").brace_style,
            BraceStyle::SameLine
        );
        assert_eq!(config.indent_size, 4);
    }

    #[test]
    fn rejects_bad_brace_styles() {
        let error = parse_config("[formatting]\nbrace_style = \"k&r\"\n").unwrap_err();
        assert!(error.message.contains("brace_style"), "{}", error.message);
        assert!(error.message.contains("allman"), "{}", error.message);
        let error = parse_config("[formatting.csharp]\nbrace_style = \"Allman\"\n").unwrap_err();
        assert!(
            error.message.contains("formatting.csharp.brace_style"),
            "{}",
            error.message
        );
        assert!(parse_config("[formatting]\nbrace_style = 1\n").is_err());
    }

    #[test]
    fn rejects_unknown_languages_and_language_options() {
        let error = parse_config("[formatting.java]\nbrace_style = \"allman\"\n").unwrap_err();
        assert!(error.message.contains("java"), "{}", error.message);
        let error = parse_config("[formatting.csharp]\nquote_style = \"double\"\n").unwrap_err();
        assert!(error.message.contains("quote_style"), "{}", error.message);
        // Language tables cannot contain other language tables.
        assert!(parse_config("[formatting.csharp.rust]\nindent_size = 2\n").is_err());
    }

    #[test]
    fn every_option_can_be_overridden_per_language() {
        let text = "[formatting]\nindent_size = 4\nblank_lines = true\n\n[formatting.toml]\nindent_size = 2\nindent_style = \"tabs\"\ncolumn_width = 100\ncomment_alignment = false\nline_ending = \"crlf\"\nbrace_style = \"allman\"\nblank_lines = false\nspace_around_operators = false\nfinal_newline = false\ntrim_trailing_whitespace = false\ncontinuation_indent = true\n";
        let config = parse_config(text).unwrap();
        let toml = config.for_language("toml");
        assert_eq!(toml.indent_size, 2);
        assert_eq!(toml.indent_style, IndentStyle::Tabs);
        assert_eq!(toml.column_width, 100);
        assert!(!toml.comment_alignment);
        assert_eq!(toml.line_ending, LineEnding::Crlf);
        assert_eq!(toml.brace_style, BraceStyle::Allman);
        assert!(!toml.blank_lines);
        assert!(!toml.space_around_operators);
        assert!(!toml.final_newline);
        assert!(!toml.trim_trailing_whitespace);
        assert!(toml.continuation_indent);
        // Other languages keep the [formatting] values.
        let rust = config.for_language("rust");
        assert_eq!(rust.indent_size, 4);
        assert!(rust.blank_lines);
        assert!(rust.final_newline);
        assert!(rust.language_overrides.is_empty());
    }

    #[test]
    fn language_tables_only_change_what_they_set() {
        let text = "[formatting]\nindent_size = 3\nfinal_newline = false\n\n[formatting.csharp]\nspace_around_operators = false\n";
        let csharp = parse_config(text).unwrap().for_language("csharp");
        assert!(!csharp.space_around_operators);
        assert_eq!(csharp.indent_size, 3);
        assert!(!csharp.final_newline);
        assert!(csharp.trim_trailing_whitespace);
        assert!(!csharp.continuation_indent);
    }

    #[test]
    fn bad_values_in_language_tables_name_the_table() {
        let error = parse_config("[formatting.rust]\nindent_size = 99\n").unwrap_err();
        assert!(
            error.message.contains("formatting.rust.indent_size"),
            "{}",
            error.message
        );
        assert!(parse_config("[formatting.rust]\nfinal_newline = \"yes\"\n").is_err());
        assert!(parse_config("[formatting]\nblank_lines = 1\n").is_err());
        assert!(parse_config("[formatting]\ntrim_trailing_whitespace = \"true\"\n").is_err());
    }

    #[test]
    fn missing_options_use_defaults() {
        let config = parse_config("[formatting]\ncolumn_width = 100\n").unwrap();
        assert_eq!(config.column_width, 100);
        assert_eq!(config.indent_size, 4);
    }

    #[test]
    fn display_round_trips() {
        let config = Config {
            indent_size: 3,
            indent_style: IndentStyle::Tabs,
            column_width: 99,
            comment_alignment: false,
            line_ending: LineEnding::Crlf,
            brace_style: BraceStyle::Allman,
            blank_lines: false,
            space_around_operators: false,
            final_newline: false,
            trim_trailing_whitespace: false,
            continuation_indent: true,
            language_overrides: vec![
                (
                    "rust".to_string(),
                    Overrides {
                        brace_style: Some(BraceStyle::SameLine),
                        indent_size: Some(2),
                        ..Overrides::default()
                    },
                ),
                (
                    "csharp".to_string(),
                    Overrides {
                        brace_style: Some(BraceStyle::Gnu),
                        final_newline: Some(true),
                        line_ending: Some(LineEnding::Lf),
                        ..Overrides::default()
                    },
                ),
            ],
        };
        assert_eq!(parse_config(&config.to_string()).unwrap(), config);
    }

    #[test]
    fn rejects_unknown_option() {
        let error = parse_config("[formatting]\nquote_style = \"double\"\n").unwrap_err();
        assert!(error.message.contains("quote_style"), "{}", error.message);
    }

    #[test]
    fn rejects_unknown_table() {
        let error = parse_config("[format]\nindent_size = 2\n").unwrap_err();
        assert!(error.message.contains("format"), "{}", error.message);
    }

    #[test]
    fn rejects_bad_indent_style() {
        let error = parse_config("[formatting]\nindent_style = \"tab\"\n").unwrap_err();
        assert!(error.message.contains("indent_style"), "{}", error.message);
        assert!(error.message.contains("\"tab\""), "{}", error.message);
    }

    #[test]
    fn rejects_bad_line_ending() {
        let error = parse_config("[formatting]\nline_ending = \"cr\"\n").unwrap_err();
        assert!(error.message.contains("line_ending"), "{}", error.message);
    }

    #[test]
    fn rejects_out_of_range_numbers() {
        assert!(parse_config("[formatting]\nindent_size = 0\n").is_err());
        assert!(parse_config("[formatting]\nindent_size = 17\n").is_err());
        assert!(parse_config("[formatting]\ncolumn_width = 5\n").is_err());
        assert!(parse_config("[formatting]\ncolumn_width = 100000\n").is_err());
        assert!(parse_config("[formatting]\nindent_size = -4\n").is_err());
    }

    #[test]
    fn rejects_wrong_types() {
        let error = parse_config("[formatting]\nindent_size = \"four\"\n").unwrap_err();
        assert!(error.message.contains("line 2"), "{}", error.message);
        assert!(parse_config("[formatting]\ncomment_alignment = \"yes\"\n").is_err());
    }

    #[test]
    fn rejects_invalid_toml() {
        assert!(parse_config("[formatting\nindent_size = 2").is_err());
    }

    #[test]
    fn discovers_config_in_parent_directories() {
        let root =
            std::env::temp_dir().join(format!("autoformat-config-test-{}", std::process::id()));
        let nested = root.join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(
            root.join(CONFIG_FILE_NAME),
            "[formatting]\nindent_size = 2\n",
        )
        .unwrap();

        let file = nested.join("main.rs");
        std::fs::write(&file, "fn main() {}\n").unwrap();
        let (config, path) = discover_config(&file).unwrap();
        assert_eq!(config.indent_size, 2);
        assert_eq!(path.unwrap(), root.join(CONFIG_FILE_NAME));

        // A closer file wins over one further up.
        std::fs::write(
            nested.join(CONFIG_FILE_NAME),
            "[formatting]\nindent_size = 8\n",
        )
        .unwrap();
        let (config, _) = discover_config(&file).unwrap();
        assert_eq!(config.indent_size, 8);

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn missing_config_file_gives_defaults() {
        let root = std::env::temp_dir().join(format!("autoformat-noconfig-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        // Note: a config file in a parent of the temp dir would be found; the temp dir
        // normally has none.
        if find_config_file(&root).is_none() {
            let (config, path) = discover_config(&root).unwrap();
            assert_eq!(config, Config::default());
            assert!(path.is_none());
        }
        std::fs::remove_dir_all(&root).unwrap();
    }
}
