//! Configuration for AutoFormat.
//!
//! AutoFormat has six options, all in the `[formatting]` table of `autoformat.toml`:
//!
//! ```toml
//! [formatting]
//! indent_size = 4
//! indent_style = "spaces"
//! column_width = 140
//! comment_alignment = true
//! line_ending = "lf"
//! brace_style = "same_line"
//!
//! [formatting.csharp]
//! brace_style = "allman"
//! ```
//!
//! `brace_style` can be overridden per language in a `[formatting.<language>]` table.
//! A language without its own value uses the global one.
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

impl LineEnding {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
        }
    }
}

/// The complete, validated formatting configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub indent_size: usize,
    pub indent_style: IndentStyle,
    pub column_width: usize,
    pub comment_alignment: bool,
    pub line_ending: LineEnding,
    /// The global brace style.
    pub brace_style: BraceStyle,
    /// Brace styles set in `[formatting.<language>]` tables, in the order of
    /// [`LANGUAGE_SECTIONS`].
    pub language_brace_styles: Vec<(String, BraceStyle)>,
}

impl Config {
    /// The brace style for a language: its own setting if it has one, otherwise the
    /// global setting. `language` is a name from [`LANGUAGE_SECTIONS`].
    pub fn brace_style_for(&self, language: &str) -> BraceStyle {
        for (name, style) in &self.language_brace_styles {
            if name == language {
                return *style;
            }
        }
        self.brace_style
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
            language_brace_styles: Vec::new(),
        }
    }
}

impl fmt::Display for Config {
    /// Prints the configuration in `autoformat.toml` syntax.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let indent_style = match self.indent_style {
            IndentStyle::Spaces => "spaces",
            IndentStyle::Tabs => "tabs",
        };
        let line_ending = match self.line_ending {
            LineEnding::Lf => "lf",
            LineEnding::Crlf => "crlf",
        };
        writeln!(f, "[formatting]")?;
        writeln!(f, "indent_size = {}", self.indent_size)?;
        writeln!(f, "indent_style = \"{}\"", indent_style)?;
        writeln!(f, "column_width = {}", self.column_width)?;
        writeln!(f, "comment_alignment = {}", self.comment_alignment)?;
        writeln!(f, "line_ending = \"{}\"", line_ending)?;
        writeln!(f, "brace_style = \"{}\"", self.brace_style.as_str())?;
        for (language, style) in &self.language_brace_styles {
            writeln!(f)?;
            writeln!(f, "[formatting.{}]", language)?;
            writeln!(f, "brace_style = \"{}\"", style.as_str())?;
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FormattingTable {
    indent_size: Option<i64>,
    indent_style: Option<String>,
    column_width: Option<i64>,
    comment_alignment: Option<bool>,
    line_ending: Option<String>,
    brace_style: Option<String>,
    // One optional table per language: `[formatting.csharp]` and so on.
    rust: Option<LanguageTable>,
    toml: Option<LanguageTable>,
    javascript: Option<LanguageTable>,
    typescript: Option<LanguageTable>,
    csharp: Option<LanguageTable>,
    powershell: Option<LanguageTable>,
}

/// Settings that can differ per language. Only `brace_style` for now.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LanguageTable {
    brace_style: Option<String>,
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

    if let Some(value) = table.indent_size {
        config.indent_size = check_range("indent_size", value, MIN_INDENT_SIZE, MAX_INDENT_SIZE)?;
    }
    if let Some(value) = table.column_width {
        config.column_width =
            check_range("column_width", value, MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH)?;
    }
    if let Some(value) = table.indent_style {
        config.indent_style = match value.as_str() {
            "spaces" => IndentStyle::Spaces,
            "tabs" => IndentStyle::Tabs,
            _ => {
                return Err(invalid_choice(
                    "indent_style",
                    &value,
                    "\"spaces\" or \"tabs\"",
                ))
            }
        };
    }
    if let Some(value) = table.line_ending {
        config.line_ending = match value.as_str() {
            "lf" => LineEnding::Lf,
            "crlf" => LineEnding::Crlf,
            _ => return Err(invalid_choice("line_ending", &value, "\"lf\" or \"crlf\"")),
        };
    }
    if let Some(value) = table.comment_alignment {
        config.comment_alignment = value;
    }
    if let Some(value) = &table.brace_style {
        config.brace_style = parse_brace_style("brace_style", value)?;
    }

    let language_tables = [
        ("rust", &table.rust),
        ("toml", &table.toml),
        ("javascript", &table.javascript),
        ("typescript", &table.typescript),
        ("csharp", &table.csharp),
        ("powershell", &table.powershell),
    ];
    for (language, language_table) in language_tables {
        if let Some(LanguageTable {
            brace_style: Some(value),
        }) = language_table
        {
            let name = format!("formatting.{}.brace_style", language);
            let style = parse_brace_style(&name, value)?;
            config
                .language_brace_styles
                .push((language.to_string(), style));
        }
    }
    Ok(config)
}

fn parse_brace_style(name: &str, value: &str) -> Result<BraceStyle, ConfigError> {
    match value {
        "same_line" => Ok(BraceStyle::SameLine),
        "allman" => Ok(BraceStyle::Allman),
        "gnu" => Ok(BraceStyle::Gnu),
        _ => Err(invalid_choice(
            name,
            value,
            "\"same_line\", \"allman\" or \"gnu\"",
        )),
    }
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
        assert!(config.language_brace_styles.is_empty());
    }

    #[test]
    fn reads_every_option() {
        let text = "[formatting]\nindent_size = 2\nindent_style = \"tabs\"\ncolumn_width = 80\ncomment_alignment = false\nline_ending = \"crlf\"\nbrace_style = \"gnu\"\n";
        let config = parse_config(text).unwrap();
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
        assert_eq!(config.brace_style_for("csharp"), BraceStyle::Allman);
        assert_eq!(config.brace_style_for("rust"), BraceStyle::Gnu);
        // Languages without their own table inherit the global style.
        assert_eq!(config.brace_style_for("typescript"), BraceStyle::SameLine);
        assert_eq!(config.brace_style_for("powershell"), BraceStyle::SameLine);
    }

    #[test]
    fn language_tables_inherit_a_non_default_global_style() {
        let text = "[formatting]\nbrace_style = \"allman\"\n\n[formatting.javascript]\nbrace_style = \"same_line\"\n\n[formatting.csharp]\n";
        let config = parse_config(text).unwrap();
        assert_eq!(config.brace_style_for("javascript"), BraceStyle::SameLine);
        // An empty language table changes nothing.
        assert_eq!(config.brace_style_for("csharp"), BraceStyle::Allman);
        assert_eq!(config.brace_style_for("rust"), BraceStyle::Allman);
    }

    #[test]
    fn language_table_without_global_brace_style_uses_the_default_elsewhere() {
        let config = parse_config("[formatting.csharp]\nbrace_style = \"allman\"\n").unwrap();
        assert_eq!(config.brace_style_for("csharp"), BraceStyle::Allman);
        assert_eq!(config.brace_style_for("rust"), BraceStyle::SameLine);
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
        let error = parse_config("[formatting.csharp]\nindent_size = 2\n").unwrap_err();
        assert!(error.message.contains("indent_size"), "{}", error.message);
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
            language_brace_styles: vec![
                ("rust".to_string(), BraceStyle::SameLine),
                ("csharp".to_string(), BraceStyle::Gnu),
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
