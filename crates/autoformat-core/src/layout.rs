//! The layout model: a language-independent description of a document that the printer
//! turns into text.
//!
//! A [`LayoutDocument`] is a list of tokens plus bracket groups. Each token carries the
//! layout decisions the language adapter made for it:
//!
//! - **Text**: the token text, printed exactly as written.
//! - **Line**: hard line breaks taken from the source (`line_breaks_before`).
//! - **Soft lines / break opportunities**: places inside a wrappable [`Group`] where the
//!   printer may add a line break when a line is longer than `column_width`.
//! - **Indent**: bracket groups indent their contents; `anchors` give extra
//!   "continuation" indentation for statements that span several lines.
//! - **Spacing**: whether a space separates the token from the previous one.

/// How the space between two tokens on the same line is decided.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spacing {
    /// Always one space.
    Space,
    /// No space, unless removing the space would merge the two tokens into one.
    NoSpace,
    /// Keep what the source had: one space if there was any space, otherwise none.
    Keep,
}

/// How a token that spans several lines (a block comment, a multi-line string...) is
/// printed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiLine {
    /// Every line is printed exactly as written. Used for strings.
    Verbatim,
    /// The lines after the first move left or right by the same amount as the line the
    /// token starts on. Used for block comments and JSX markup.
    Shift,
    /// Lines starting with `*` are lined up under the first line, like this:
    /// ```text
    /// /**
    ///  * Text
    ///  */
    /// ```
    StarAligned,
}

/// One token and the layout decisions for it.
#[derive(Clone, Debug)]
pub struct LayoutToken {
    /// The exact source text. It is never changed.
    pub text: String,
    /// Comments (and comment-like lines such as `#region`) are indented like the code
    /// that follows them, and trailing comments can be aligned.
    pub is_comment: bool,
    /// Line breaks between the previous token and this one in the source.
    pub line_breaks_before: usize,
    /// Whether the source had a space in front of this token (when on the same line).
    pub had_space_before: bool,
    /// How to space this token from the previous one when both are on the same line.
    pub spacing: Spacing,
    /// Indexes of tokens where the syntax nodes that contain this token start.
    /// When this token starts a line, each distinct earlier line among these anchors adds
    /// one level of continuation indentation.
    pub anchors: Vec<usize>,
    /// When this token starts a line, use one level less continuation indentation.
    /// Used for `else`, `catch`, a `{` on its own line and similar tokens.
    pub outdent: bool,
    /// Always print this token at column 0 when it starts a line (C# `#if`).
    pub column_zero: bool,
    /// This `{` opens a code block (function body, class body, `if` block...) whose
    /// placement follows `brace_style`. Decided by the language adapter from the syntax
    /// tree, never from the text alone.
    pub block_brace: bool,
    /// This token (`else`, `catch`, `finally`, the `while` of a do-while) directly
    /// follows the closing brace of a code block and continues the same statement.
    pub brace_continuation: bool,
    /// The group this token opens or closes, if it is a bracket.
    pub opens_group: Option<usize>,
    pub closes_group: Option<usize>,
    /// How to print the token if it spans several lines.
    pub multi_line: MultiLine,
    /// For [`MultiLine::Shift`]: line numbers (0 = first line of the token) that must be
    /// printed exactly as written because they start inside a string.
    pub protected_lines: Vec<usize>,
    /// The indentation width (in columns) of the source line this token starts on. Used
    /// to shift multi-line tokens.
    pub original_line_indent: usize,
}

impl LayoutToken {
    /// A plain token with neutral layout decisions.
    pub fn new(text: &str) -> LayoutToken {
        LayoutToken {
            text: text.to_string(),
            is_comment: false,
            line_breaks_before: 0,
            had_space_before: false,
            spacing: Spacing::Keep,
            anchors: Vec::new(),
            outdent: false,
            column_zero: false,
            block_brace: false,
            brace_continuation: false,
            opens_group: None,
            closes_group: None,
            multi_line: MultiLine::Verbatim,
            protected_lines: Vec::new(),
            original_line_indent: 0,
        }
    }

    pub fn is_multi_line(&self) -> bool {
        self.text.contains('\n')
    }
}

/// A pair of brackets: `(...)`, `[...]`, `{...}` and similar.
#[derive(Clone, Debug)]
pub struct Group {
    /// Index of the opening token.
    pub open: usize,
    /// Index of the closing token.
    pub close: usize,
    /// Indexes of separator tokens (usually commas) directly inside the group. A line
    /// break may be added after each of them when the group is wrapped.
    pub separators: Vec<usize>,
    /// Whether the printer may wrap this group when a line is too long.
    pub wrappable: bool,
}

/// A whole document ready for printing.
#[derive(Clone, Debug, Default)]
pub struct LayoutDocument {
    pub tokens: Vec<LayoutToken>,
    pub groups: Vec<Group>,
}
