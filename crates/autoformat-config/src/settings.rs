//! The one list of settings.
//!
//! Reading `autoformat.toml`, checking it, and the help editors show while the file is
//! edited (suggestions and explanations) all use this list, so they always agree.

use crate::{MAX_COLUMN_WIDTH, MAX_INDENT_SIZE, MIN_COLUMN_WIDTH, MIN_INDENT_SIZE};

/// What kind of value a setting takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingKind {
    /// A whole number from `min` to `max`.
    Number { min: usize, max: usize },
    /// One of these words, written in quotes.
    Choice(&'static [&'static str]),
    /// `true` or `false`.
    Switch,
}

/// One setting of `autoformat.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Setting {
    pub name: &'static str,
    pub kind: SettingKind,
    /// The default value, as it is written in `autoformat.toml`.
    pub default: &'static str,
    /// One or two plain sentences about what the setting does.
    pub description: &'static str,
}

/// Every setting, in the order of the fields of [`crate::Config`].
pub const SETTINGS: &[Setting] = &[
    Setting {
        name: "indent_size",
        kind: SettingKind::Number {
            min: MIN_INDENT_SIZE,
            max: MAX_INDENT_SIZE,
        },
        default: "4",
        description: "How deep one indentation step is.",
    },
    Setting {
        name: "indent_style",
        kind: SettingKind::Choice(&["spaces", "tabs"]),
        default: "\"spaces\"",
        description: "Indent with spaces or with tabs.",
    },
    Setting {
        name: "column_width",
        kind: SettingKind::Number {
            min: MIN_COLUMN_WIDTH,
            max: MAX_COLUMN_WIDTH,
        },
        default: "140",
        description: "How long a line may be before lists are wrapped onto several lines.",
    },
    Setting {
        name: "comment_alignment",
        kind: SettingKind::Switch,
        default: "true",
        description: "Lines up comments at the end of neighbouring lines, so they start in the same column.",
    },
    Setting {
        name: "line_ending",
        kind: SettingKind::Choice(&["lf", "crlf"]),
        default: "\"lf\"",
        description: "How every line ends: \"lf\" for macOS and Linux, \"crlf\" for Windows.",
    },
    Setting {
        name: "brace_style",
        kind: SettingKind::Choice(&["same_line", "allman", "gnu"]),
        default: "\"same_line\"",
        description: "Where the `{` of a code block goes. \"same_line\" keeps it at the end of the line. \"allman\" puts it on its own line. \"gnu\" puts it on its own line and indents it one step.",
    },
    Setting {
        name: "blank_lines",
        kind: SettingKind::Switch,
        default: "true",
        description: "Puts one empty line between functions, classes and other declarations when one of them spans several lines.",
    },
    Setting {
        name: "space_around_operators",
        kind: SettingKind::Switch,
        default: "true",
        description: "Puts one space on each side of operators such as `=`, `+` and `==`. When false, the spacing is kept as written.",
    },
    Setting {
        name: "final_newline",
        kind: SettingKind::Switch,
        default: "true",
        description: "Ends the file with exactly one line break. When false, the file ends right after its last character.",
    },
    Setting {
        name: "trim_trailing_whitespace",
        kind: SettingKind::Switch,
        default: "true",
        description: "Removes spaces and tabs at the end of lines.",
    },
    Setting {
        name: "continuation_indent",
        kind: SettingKind::Switch,
        default: "false",
        description: "Pushes in a block in the middle of a statement, and the rest of that statement, one extra step (PowerShell only).",
    },
];

/// The setting with this name, if there is one.
pub fn find_setting(name: &str) -> Option<&'static Setting> {
    SETTINGS.iter().find(|setting| setting.name == name)
}

/// The names of all settings, for messages: `indent_size, indent_style, ...`.
pub fn setting_names() -> String {
    let names: Vec<&str> = SETTINGS.iter().map(|setting| setting.name).collect();
    names.join(", ")
}

impl Setting {
    /// The values to offer while typing, as they are written in the file. A number
    /// setting offers its default.
    pub fn values(&self) -> Vec<String> {
        match self.kind {
            SettingKind::Number { .. } => vec![self.default.to_string()],
            SettingKind::Choice(words) => {
                words.iter().map(|word| format!("\"{}\"", word)).collect()
            }
            SettingKind::Switch => vec!["true".to_string(), "false".to_string()],
        }
    }

    /// The allowed values in words, for example `"spaces" or "tabs"`.
    pub fn allowed(&self) -> String {
        match self.kind {
            SettingKind::Number { min, max } => {
                format!("a whole number from {} to {}", min, max)
            }
            SettingKind::Choice(_) => {
                let mut words = self.values();
                let last = words.pop().unwrap_or_default();
                if words.is_empty() {
                    return last;
                }
                format!("{} or {}", words.join(", "), last)
            }
            SettingKind::Switch => "true or false".to_string(),
        }
    }

    /// The explanation an editor shows for this setting: what it does, the allowed
    /// values and the default.
    pub fn help(&self) -> String {
        format!(
            "{}\n\nAllowed: {}. Default: {}.",
            self.description,
            self.allowed(),
            self.default
        )
    }
}
