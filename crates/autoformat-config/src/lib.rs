//! Configuration for AutoFormat.
//!
//! AutoFormat has eleven options, all in the `[formatting]` table of `autoformat.toml`:
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
//!
//! The list of settings (names, allowed values, defaults and descriptions) is in
//! `settings.rs`. Reading and checking the file use that list, and so does the help
//! editors show while the file is edited.

use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

use toml::de::{DeTable, DeValue};
use toml::Spanned;

mod settings;

pub use settings::{find_setting, setting_names, Setting, SettingKind, SETTINGS};

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
    /// The style with this name in `autoformat.toml`.
    pub fn from_name(name: &str) -> Option<IndentStyle> {
        match name {
            "spaces" => Some(IndentStyle::Spaces),
            "tabs" => Some(IndentStyle::Tabs),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            IndentStyle::Spaces => "spaces",
            IndentStyle::Tabs => "tabs",
        }
    }
}

impl LineEnding {
    /// The line ending with this name in `autoformat.toml`.
    pub fn from_name(name: &str) -> Option<LineEnding> {
        match name {
            "lf" => Some(LineEnding::Lf),
            "crlf" => Some(LineEnding::Crlf),
            _ => None,
        }
    }

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
    /// The brace style with this name in `autoformat.toml`.
    pub fn from_name(name: &str) -> Option<BraceStyle> {
        match name {
            "same_line" => Some(BraceStyle::SameLine),
            "allman" => Some(BraceStyle::Allman),
            "gnu" => Some(BraceStyle::Gnu),
            _ => None,
        }
    }

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
    "json",
    "javascript",
    "typescript",
    "csharp",
    "java",
    "kotlin",
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

/// A mistake in an `autoformat.toml` file, and where it is. `start` and `end` are byte
/// positions in the text of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mistake {
    pub start: usize,
    pub end: usize,
    pub message: String,
}

/// Parses and validates the text of an `autoformat.toml` file.
pub fn parse_config(text: &str) -> Result<Config, ConfigError> {
    let (config, mistakes) = check_config(text);
    match mistakes.first() {
        None => Ok(config),
        Some(mistake) => {
            let line = text[..mistake.start].matches('\n').count() + 1;
            Err(ConfigError {
                path: None,
                message: format!("{} (line {})", mistake.message, line),
            })
        }
    }
}

/// What the TOML reader says when a name is written twice in one table.
const DUPLICATE_KEY: &str = "duplicate key";

/// Reads the text of an `autoformat.toml` file and finds every mistake in it, in the
/// order of the file. The configuration is only complete when there are no mistakes.
pub fn check_config(text: &str) -> (Config, Vec<Mistake>) {
    // The TOML reader tells where every name and value is, and reads on after a mistake.
    let (document, errors) = DeTable::parse_recoverable(text);
    let mut reader = Reader {
        text,
        first_syntax_error: None,
        mistakes: Vec::new(),
    };
    for error in &errors {
        let span = error.span().unwrap_or(0..0);
        if error.message() == DUPLICATE_KEY {
            let name = text.get(span.clone()).unwrap_or("");
            let message = format!(
                "`{}` is written twice. Every setting and every table may only be written once",
                name
            );
            reader.add(span, message);
        } else {
            let first = reader.first_syntax_error.unwrap_or(span.start);
            reader.first_syntax_error = Some(first.min(span.start));
            let message = syntax_message(text, span.start, error.message());
            reader.add(span, message);
        }
    }

    let mut config = Config::default();
    for (key, value) in document.get_ref() {
        if !reader.can_trust(key.span()) {
            continue;
        }
        let name: &str = key.get_ref();
        match value.get_ref() {
            DeValue::Table(table) if name == "formatting" => {
                reader.read_formatting(table, &mut config)
            }
            _ if name == "formatting" => reader.add(
                value.span(),
                "`formatting` must be a table, written as `[formatting]`".to_string(),
            ),
            _ => reader.add(
                key.span(),
                format!(
                    "unknown name `{}`. All settings go under `[formatting]` or `[formatting.<language>]`",
                    name
                ),
            ),
        }
    }
    reader.mistakes.sort_by_key(|mistake| mistake.start);
    (config, reader.mistakes)
}

/// Walks through what the TOML reader found and collects the mistakes.
struct Reader<'a> {
    text: &'a str,
    /// Where the first syntax error is, if there is one.
    first_syntax_error: Option<usize>,
    mistakes: Vec<Mistake>,
}

impl Reader<'_> {
    fn add(&mut self, span: Range<usize>, message: String) {
        let (start, end) = widen(self.text, span);
        self.mistakes.push(Mistake {
            start,
            end,
            message,
        });
    }

    /// What comes after a syntax error cannot be trusted: the TOML reader had to guess
    /// there. Only names and values before it are checked.
    fn can_trust(&self, span: Range<usize>) -> bool {
        match self.first_syntax_error {
            Some(position) => span.end < position,
            None => true,
        }
    }

    /// Reads `[formatting]`: the settings, plus one optional table per language.
    fn read_formatting(&mut self, table: &DeTable, config: &mut Config) {
        let mut global = Overrides::default();
        let mut language_tables: Vec<(&str, Overrides)> = Vec::new();
        for (key, value) in table {
            if !self.can_trust(key.span()) {
                continue;
            }
            let name: &str = key.get_ref();
            if let Some(setting) = find_setting(name) {
                self.read_setting(setting, "", value, &mut global);
                continue;
            }
            let language = LANGUAGE_SECTIONS.iter().find(|language| **language == name);
            match (language, value.get_ref()) {
                (Some(language), DeValue::Table(language_table)) => {
                    let overrides = self.read_language(language, language_table);
                    language_tables.push((language, overrides));
                }
                (Some(language), _) => self.add(
                    value.span(),
                    format!(
                        "`{}` is a language, so it must be a table, written as `[formatting.{}]`",
                        language, language
                    ),
                ),
                (None, DeValue::Table(_)) => self.add(
                    key.span(),
                    format!(
                        "unknown language `{}`. The languages are: {}",
                        name,
                        LANGUAGE_SECTIONS.join(", ")
                    ),
                ),
                (None, _) => self.add_unknown_setting(key.span(), name),
            }
        }

        global.apply_to(config);
        for language in LANGUAGE_SECTIONS {
            for (name, overrides) in &language_tables {
                if name == language {
                    config
                        .language_overrides
                        .push((name.to_string(), overrides.clone()));
                }
            }
        }
    }

    /// Reads `[formatting.<language>]`: the same settings as `[formatting]`.
    fn read_language(&mut self, language: &str, table: &DeTable) -> Overrides {
        // The prefix goes in front of setting names in messages, so a mistake in
        // `[formatting.csharp]` says `formatting.csharp.indent_size`.
        let prefix = format!("formatting.{}.", language);
        let mut overrides = Overrides::default();
        for (key, value) in table {
            if !self.can_trust(key.span()) {
                continue;
            }
            let name: &str = key.get_ref();
            match find_setting(name) {
                Some(setting) => self.read_setting(setting, &prefix, value, &mut overrides),
                None => self.add_unknown_setting(key.span(), name),
            }
        }
        overrides
    }

    /// Checks one value against the list of settings, and stores it when it is allowed.
    fn read_setting(
        &mut self,
        setting: &Setting,
        prefix: &str,
        value: &Spanned<DeValue>,
        overrides: &mut Overrides,
    ) {
        if !self.can_trust(value.span()) {
            return;
        }
        match check_value(setting, value.get_ref()) {
            Some(checked) => overrides.store(setting.name, checked),
            None => {
                let found = self.text.get(value.span()).unwrap_or("");
                let message = format!(
                    "`{}{}` must be {}, found {}",
                    prefix,
                    setting.name,
                    setting.allowed(),
                    shorten(found)
                );
                self.add(value.span(), message);
            }
        }
    }

    fn add_unknown_setting(&mut self, span: Range<usize>, name: &str) {
        let message = format!(
            "unknown setting `{}`. The settings are: {}",
            name,
            setting_names()
        );
        self.add(span, message);
    }
}

/// A value that is allowed for its setting.
enum Checked {
    Number(usize),
    Word(&'static str),
    Switch(bool),
}

/// Returns the value when the list of settings allows it for this setting.
fn check_value(setting: &Setting, value: &DeValue) -> Option<Checked> {
    match (setting.kind, value) {
        (SettingKind::Number { min, max }, DeValue::Integer(number)) => {
            let number = i64::from_str_radix(number.as_str(), number.radix()).ok()?;
            if number < min as i64 || number > max as i64 {
                return None;
            }
            Some(Checked::Number(number as usize))
        }
        (SettingKind::Choice(words), DeValue::String(text)) => {
            let word = words.iter().find(|word| **word == text.as_ref())?;
            Some(Checked::Word(word))
        }
        (SettingKind::Switch, DeValue::Boolean(switch)) => Some(Checked::Switch(*switch)),
        _ => None,
    }
}

impl Overrides {
    /// Puts a checked value in the field of its setting.
    fn store(&mut self, name: &str, value: Checked) {
        match (name, value) {
            ("indent_size", Checked::Number(number)) => self.indent_size = Some(number),
            ("indent_style", Checked::Word(word)) => {
                self.indent_style = IndentStyle::from_name(word)
            }
            ("column_width", Checked::Number(number)) => self.column_width = Some(number),
            ("comment_alignment", Checked::Switch(switch)) => self.comment_alignment = Some(switch),
            ("line_ending", Checked::Word(word)) => self.line_ending = LineEnding::from_name(word),
            ("brace_style", Checked::Word(word)) => self.brace_style = BraceStyle::from_name(word),
            ("blank_lines", Checked::Switch(switch)) => self.blank_lines = Some(switch),
            ("space_around_operators", Checked::Switch(switch)) => {
                self.space_around_operators = Some(switch)
            }
            ("final_newline", Checked::Switch(switch)) => self.final_newline = Some(switch),
            ("trim_trailing_whitespace", Checked::Switch(switch)) => {
                self.trim_trailing_whitespace = Some(switch)
            }
            ("continuation_indent", Checked::Switch(switch)) => {
                self.continuation_indent = Some(switch)
            }
            // A test makes sure every setting in the list has a line above.
            _ => {}
        }
    }
}

/// The message for a syntax error. The TOML reader's own words are used, except for the
/// two unfinished lines everybody passes through while typing: `name =` and `name`.
fn syntax_message(text: &str, position: usize, reader_message: &str) -> String {
    let position = position.min(text.len());
    let line_start = match text[..position].rfind(['\n', '\r']) {
        Some(index) => index + 1,
        None => 0,
    };
    let line = text[line_start..].lines().next().unwrap_or("").trim();
    if let Some(name) = line.strip_suffix('=') {
        let name = name.trim();
        return match find_setting(name) {
            Some(setting) => format!("`{}` needs a value: {}", name, setting.allowed()),
            None => format!("`{}` needs a value", name),
        };
    }
    let is_one_word = line
        .chars()
        .all(|current| current.is_alphanumeric() || current == '_' || current == '-');
    if is_one_word && !line.is_empty() {
        return format!(
            "`{}` is not finished. A setting is written as `name = value`",
            line
        );
    }
    reader_message.to_string()
}

/// A mistake without any width (a missing value, for example) gets the word before it,
/// or the character after it, so that an editor has something to underline.
fn widen(text: &str, span: Range<usize>) -> (usize, usize) {
    let start = span.start.min(text.len());
    let end = span.end.min(text.len()).max(start);
    if start < end {
        return (start, end);
    }
    let line_start = match text[..start].rfind(['\n', '\r']) {
        Some(index) => index + 1,
        None => 0,
    };
    let before = &text[line_start..start];
    let word_end = before.trim_end();
    let word_start = match word_end.rfind([' ', '\t']) {
        Some(index) => index + 1,
        None => 0,
    };
    if !word_end.is_empty() {
        return (line_start + word_start, start);
    }
    match text[start..].chars().next() {
        Some(next) if next != '\n' && next != '\r' => (start, start + next.len_utf8()),
        _ => (start, end),
    }
}

/// The first line of `text`, cut off when it is long, for use in a message.
fn shorten(text: &str) -> String {
    let line = text.lines().next().unwrap_or("");
    if line.chars().count() <= 40 && line.len() == text.len() {
        return line.to_string();
    }
    let start: String = line.chars().take(40).collect();
    format!("{}...", start)
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
        let error = parse_config("[formatting.python]\nbrace_style = \"allman\"\n").unwrap_err();
        assert!(error.message.contains("python"), "{}", error.message);
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

    /// The field names of a struct, read from its debug print. This always lists every
    /// field, so a new field cannot be forgotten.
    fn field_names(debug_print: &str) -> Vec<String> {
        let mut names = Vec::new();
        for line in debug_print.lines() {
            // Fields are indented by exactly four spaces: `    indent_size: 4,`
            let is_field = line.starts_with("    ") && !line.starts_with("     ");
            if let (true, Some((name, _))) = (is_field, line.trim().split_once(':')) {
                names.push(name.to_string());
            }
        }
        names
    }

    #[test]
    fn the_list_has_every_setting_of_config_and_nothing_else() {
        let listed: Vec<&str> = SETTINGS.iter().map(|setting| setting.name).collect();

        let mut in_config = field_names(&format!("{:#?}", Config::default()));
        assert_eq!(in_config.pop().as_deref(), Some("language_overrides"));
        assert_eq!(
            in_config, listed,
            "Config and SETTINGS must name the same settings"
        );

        let in_overrides = field_names(&format!("{:#?}", Overrides::default()));
        assert_eq!(
            in_overrides, listed,
            "Overrides and SETTINGS must name the same settings"
        );
    }

    #[test]
    fn the_defaults_in_the_list_are_the_real_defaults() {
        let printed = Config::default().to_string();
        let mut lines = printed.lines();
        assert_eq!(lines.next(), Some("[formatting]"));
        for setting in SETTINGS {
            let expected = format!("{} = {}", setting.name, setting.default);
            assert_eq!(lines.next(), Some(expected.as_str()));
        }
        assert_eq!(lines.next(), None);
    }

    #[test]
    fn every_value_in_the_list_can_be_read() {
        for setting in SETTINGS {
            let mut values = setting.values();
            if let SettingKind::Number { min, max } = setting.kind {
                values.push(min.to_string());
                values.push(max.to_string());
            }
            for value in values {
                let line = format!("{} = {}", setting.name, value);
                // In [formatting] and in a language table: the value must be stored, which
                // shows when the configuration is printed again.
                let config = parse_config(&format!("[formatting]\n{}\n", line)).unwrap();
                assert!(config.to_string().contains(&line), "{}", line);
                let config = parse_config(&format!("[formatting.rust]\n{}\n", line)).unwrap();
                let printed = config.to_string();
                let (_, rust_table) = printed.split_once("[formatting.rust]\n").unwrap();
                assert_eq!(rust_table.trim(), line);
            }
        }
    }

    #[test]
    fn the_words_in_the_list_are_the_words_the_code_knows() {
        let words = |name: &str| match find_setting(name).unwrap().kind {
            SettingKind::Choice(words) => words.to_vec(),
            _ => panic!("{} is not a choice", name),
        };
        for word in words("indent_style") {
            assert_eq!(IndentStyle::from_name(word).unwrap().as_str(), word);
        }
        for word in words("line_ending") {
            assert_eq!(LineEnding::from_name(word).unwrap().name(), word);
        }
        for word in words("brace_style") {
            assert_eq!(BraceStyle::from_name(word).unwrap().as_str(), word);
        }
        assert_eq!(IndentStyle::from_name("tab"), None);
        assert_eq!(LineEnding::from_name("cr"), None);
        assert_eq!(BraceStyle::from_name("Allman"), None);
    }

    #[test]
    fn help_texts_come_from_the_list() {
        let brace_style = find_setting("brace_style").unwrap();
        assert_eq!(
            brace_style.allowed(),
            "\"same_line\", \"allman\" or \"gnu\""
        );
        assert_eq!(
            brace_style.values(),
            ["\"same_line\"", "\"allman\"", "\"gnu\""]
        );
        assert!(brace_style
            .help()
            .contains("Where the `{` of a code block goes"));
        assert!(brace_style
            .help()
            .ends_with("Allowed: \"same_line\", \"allman\" or \"gnu\". Default: \"same_line\"."));

        let indent_size = find_setting("indent_size").unwrap();
        assert_eq!(indent_size.allowed(), "a whole number from 1 to 16");
        assert_eq!(indent_size.values(), ["4"]);
        assert_eq!(
            find_setting("final_newline").unwrap().allowed(),
            "true or false"
        );
        assert!(find_setting("quote_style").is_none());
        for setting in SETTINGS {
            assert!(!setting.description.is_empty(), "{}", setting.name);
        }
    }

    /// The mistakes in `text`, as (the marked text, the message).
    fn mistakes_in(text: &str) -> Vec<(String, String)> {
        let (_, mistakes) = check_config(text);
        mistakes
            .iter()
            .map(|mistake| {
                (
                    text[mistake.start..mistake.end].to_string(),
                    mistake.message.clone(),
                )
            })
            .collect()
    }

    #[test]
    fn a_correct_file_has_no_mistakes() {
        assert!(mistakes_in("").is_empty());
        assert!(mistakes_in(&Config::default().to_string()).is_empty());
        assert!(
            mistakes_in("# note\n[formatting.csharp]\nbrace_style = 'allman' # C#\n").is_empty()
        );
    }

    #[test]
    fn every_mistake_is_found_with_its_exact_place() {
        let text = "[formatting]\nindent_size = 99\nindent_style = \"tab\"\nfinal_newline = \"yes\"\nquote_style = \"double\"\n\n[formatting.python]\nindent_size = 2\n\n[formatting.csharp]\ncolumn_width = 5\nwidth = 1\n\n[format]\nindent_size = 2\n";
        let found = mistakes_in(text);
        let marked: Vec<&str> = found.iter().map(|(marked, _)| marked.as_str()).collect();
        assert_eq!(
            marked,
            [
                "99",
                "\"tab\"",
                "\"yes\"",
                "quote_style",
                "python",
                "5",
                "width",
                "format"
            ]
        );
        assert_eq!(
            found[0].1,
            "`indent_size` must be a whole number from 1 to 16, found 99"
        );
        assert_eq!(
            found[1].1,
            "`indent_style` must be \"spaces\" or \"tabs\", found \"tab\""
        );
        assert_eq!(
            found[2].1,
            "`final_newline` must be true or false, found \"yes\""
        );
        assert!(found[3]
            .1
            .starts_with("unknown setting `quote_style`. The settings are: indent_size, "));
        assert!(found[4]
            .1
            .starts_with("unknown language `python`. The languages are: rust, toml, "));
        assert_eq!(
            found[5].1,
            "`formatting.csharp.column_width` must be a whole number from 20 to 1000, found 5"
        );
        assert!(found[6].1.starts_with("unknown setting `width`"));
        assert!(found[7].1.starts_with("unknown name `format`"));
    }

    #[test]
    fn a_setting_written_twice_is_a_mistake_on_the_second_one() {
        let text = "[formatting]\nindent_size = 2\nindent_size = 3\n";
        let (_, mistakes) = check_config(text);
        assert_eq!(mistakes.len(), 1);
        // The second `indent_size`, on the third line.
        assert_eq!(mistakes[0].start, 29);
        assert_eq!(&text[mistakes[0].start..mistakes[0].end], "indent_size");
        assert!(mistakes[0]
            .message
            .starts_with("`indent_size` is written twice"));

        let found = mistakes_in("[formatting.rust]\n[formatting.rust]\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "rust");
    }

    #[test]
    fn a_half_written_file_marks_the_unfinished_place_and_the_mistakes_before_it() {
        // `brace_style` has no value yet, and `inde` is the start of a name.
        let text = "[formatting]\nindent_size = 99\nbrace_style = \ninde\n[formatting.python]\n";
        let found = mistakes_in(text);
        let marked: Vec<&str> = found.iter().map(|(marked, _)| marked.as_str()).collect();
        assert_eq!(marked, ["99", "= ", "inde"]);
        assert_eq!(
            found[1].1,
            "`brace_style` needs a value: \"same_line\", \"allman\" or \"gnu\""
        );
        assert_eq!(
            found[2].1,
            "`inde` is not finished. A setting is written as `name = value`"
        );
        // Nothing after the unfinished place is judged, so `python` gets no mark yet.

        // An unknown name without a value gets both messages.
        let found = mistakes_in("[formatting]\nquote =");
        assert!(found[0].1.starts_with("unknown setting `quote`"));
        assert_eq!(found[1].1, "`quote` needs a value");

        // An unfinished table name must not blame the settings below it.
        let found = mistakes_in("[formatting\nindent_size = 2\n");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "[formatting");

        // An unfinished file never panics, wherever it is cut off.
        let whole =
            "[formatting]\nbrace_style = \"allman\"\n[formatting.csharp]\nindent_size = 2 # å\n";
        for (end, _) in whole.char_indices() {
            let (_, mistakes) = check_config(&whole[..end]);
            for mistake in mistakes {
                assert!(
                    mistake.start < mistake.end && mistake.end <= end,
                    "{:?}",
                    &whole[..end]
                );
            }
        }
    }

    #[test]
    fn other_ways_to_write_toml_are_read_the_same() {
        let config = parse_config(
            "formatting.indent_size = 0x10\nformatting.csharp = { brace_style = 'allman' }\n",
        )
        .unwrap();
        assert_eq!(config.indent_size, 16);
        assert_eq!(
            config.for_language("csharp").brace_style,
            BraceStyle::Allman
        );
        assert_eq!(
            parse_config("[formatting]\ncolumn_width = 1_00\n")
                .unwrap()
                .column_width,
            100
        );

        let found = mistakes_in("formatting = 1\n");
        assert_eq!(found[0].0, "1");
        let found =
            mistakes_in("[formatting]\nrust = \"x\"\nindent_size = 2.5\ncolumn_width = [1,\n 2]\n");
        assert_eq!(found[0].0, "\"x\"");
        assert!(found[0].1.contains("[formatting.rust]"));
        assert_eq!(
            found[1].1,
            "`indent_size` must be a whole number from 1 to 16, found 2.5"
        );
        assert_eq!(
            found[2].1,
            "`column_width` must be a whole number from 20 to 1000, found [1,..."
        );
    }

    #[test]
    fn errors_say_which_line() {
        let error = parse_config("[formatting]\n\nindent_size = 99\n").unwrap_err();
        assert_eq!(
            error.message,
            "`indent_size` must be a whole number from 1 to 16, found 99 (line 3)"
        );
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
