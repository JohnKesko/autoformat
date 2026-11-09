//! Coordinates parsing, layout, printing and the safety check.

use std::fmt;

use autoformat_config::{Config, IndentStyle};
use autoformat_parser::tree_sitter::Node;
use autoformat_parser::{compare_trees, SyntaxError, SyntaxTree};

use crate::adapter::LanguageAdapter;
use crate::builder::{build_layout, BuildOptions};
use crate::printer::{print, PrintOptions};

/// Why a document could not be formatted. The original text is never changed when an
/// error is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// The source has syntax errors.
    Syntax(SyntaxError),
    /// The grammar could not be loaded.
    Grammar(String),
    /// AutoFormat could not produce output that provably means the same as the input.
    Unsafe(String),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Syntax(error) => write!(f, "syntax error at {}; the file was not changed", error),
            FormatError::Grammar(message) => write!(f, "{}", message),
            FormatError::Unsafe(message) => write!(
                f,
                "formatting was stopped because the result would change the code ({}); the file was not changed",
                message
            ),
        }
    }
}

impl std::error::Error for FormatError {}

/// The result of formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formatted {
    pub text: String,
    /// Notes about steps that were skipped to keep the code safe, for example when long
    /// lines could not be wrapped.
    pub notes: Vec<String>,
}

/// The formatting attempts, from most to least ambitious. The first one whose result
/// passes the safety check wins.
struct Attempt {
    wrap_lines: bool,
    keep_spacing: bool,
    place_braces: bool,
    note: &'static str,
}

const ATTEMPTS: [Attempt; 4] = [
    Attempt {
        wrap_lines: true,
        keep_spacing: false,
        place_braces: true,
        note: "",
    },
    Attempt {
        wrap_lines: false,
        keep_spacing: false,
        place_braces: true,
        note: "long lines were not wrapped because wrapping would have changed the code",
    },
    Attempt {
        wrap_lines: false,
        keep_spacing: false,
        place_braces: false,
        note: "braces were left where they were because moving them would have changed the code",
    },
    Attempt {
        wrap_lines: false,
        keep_spacing: true,
        place_braces: false,
        note: "only indentation was changed because spacing changes would have changed the code",
    },
];

/// Formats `source` with the given adapter and configuration.
///
/// Steps:
/// 1. parse the source (syntax errors stop here),
/// 2. build the layout and print it,
/// 3. parse the result and compare it with the original tree,
/// 4. if they differ, retry with fewer changes; if nothing is safe, return an error.
pub fn format_source(
    adapter: &dyn LanguageAdapter,
    source: &str,
    config: &Config,
) -> Result<Formatted, FormatError> {
    let has_bom = source.starts_with('\u{feff}');
    let without_bom = source.strip_prefix('\u{feff}').unwrap_or(source);
    let normalized = without_bom.replace("\r\n", "\n");
    if normalized.trim().is_empty() {
        return Ok(Formatted {
            text: String::new(),
            notes: Vec::new(),
        });
    }

    let tree = adapter
        .grammar()
        .parse(&normalized)
        .map_err(FormatError::Grammar)?;
    if let Some(error) = tree.first_error() {
        return Err(FormatError::Syntax(error));
    }

    let brace_style = config.brace_style_for(adapter.config_name());
    let mut problem = String::new();
    let mut notes = Vec::new();
    for attempt in &ATTEMPTS {
        let build_options = BuildOptions {
            keep_spacing: attempt.keep_spacing,
            indent_size: config.indent_size,
        };
        let print_options = PrintOptions {
            indent_size: config.indent_size,
            use_tabs: config.indent_style == IndentStyle::Tabs,
            column_width: config.column_width,
            align_comments: config.comment_alignment,
            wrap_lines: attempt.wrap_lines,
            brace_style: if attempt.place_braces {
                Some(brace_style)
            } else {
                None
            },
        };
        let document = build_layout(&tree, adapter, &build_options);
        let printed = print(&document, &print_options);
        match check_same_program(adapter, &tree, &printed) {
            Ok(()) => {
                if !attempt.note.is_empty() {
                    notes.push(format!("{} ({})", attempt.note, problem));
                }
                return Ok(Formatted {
                    text: finish(printed, config, has_bom),
                    notes,
                });
            }
            Err(message) => problem = message,
        }
    }
    Err(FormatError::Unsafe(problem))
}

/// Parses the printed text and compares it with the original syntax tree.
fn check_same_program(
    adapter: &dyn LanguageAdapter,
    original: &SyntaxTree,
    printed: &str,
) -> Result<(), String> {
    // Formatting only changes whitespace, so every other character must be unchanged
    // and in the same order. This catches lost or duplicated text anywhere in the file.
    if without_whitespace(original.source()) != without_whitespace(printed) {
        return Err("the text other than whitespace changed".to_string());
    }
    let formatted = adapter.grammar().parse(printed)?;
    if let Some(error) = formatted.first_error() {
        return Err(format!("the result would not parse: {}", error));
    }
    let is_atomic = |node: Node| adapter.compare_as_atomic(node);
    let text_matches =
        |node: Node, before: &str, after: &str| leaf_text_matches(adapter, node, before, after);
    compare_trees(original, &formatted, &is_atomic, &text_matches)
}

/// The shared rules for comparing token text before and after formatting.
fn leaf_text_matches(adapter: &dyn LanguageAdapter, node: Node, before: &str, after: &str) -> bool {
    if let Some(answer) = adapter.text_matches(node, before, after) {
        return answer;
    }
    if before.trim().is_empty() {
        // Whitespace that the grammar turned into a node only has to stay whitespace.
        return after.trim().is_empty() && !after.is_empty();
    }
    if adapter.rules().is_comment(node) {
        // Comments may be re-indented, so only compare their visible characters.
        return without_whitespace(before) == without_whitespace(after);
    }
    before == after
}

fn without_whitespace(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Applies the configured line ending and restores a byte order mark.
fn finish(printed: String, config: &Config, has_bom: bool) -> String {
    let mut text = if config.line_ending.as_str() == "\n" {
        printed
    } else {
        printed.replace('\n', config.line_ending.as_str())
    };
    if has_bom {
        text.insert(0, '\u{feff}');
    }
    text
}
