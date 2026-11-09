//! The interface every language adapter implements.

use autoformat_parser::tree_sitter::Node;
use autoformat_parser::{Grammar, SyntaxTree};

use crate::context::TokenContext;
use crate::layout::Spacing;

/// Tables of node kinds that describe a language to the shared formatting rules.
///
/// Node kinds are the names Tree-sitter uses for syntax nodes (`"string_literal"`,
/// `"block"`...). Run `autoformat tree <file>` to see the kinds in a file.
pub struct LanguageRules {
    /// Comment nodes. Comments are printed exactly as written (they may be re-indented)
    /// and are indented like the code that follows them.
    pub comment_kinds: &'static [&'static str],
    /// Nodes printed exactly as written, never split into smaller tokens: strings,
    /// characters, raw strings, here-strings and similar.
    pub atomic_kinds: &'static [&'static str],
    /// Container nodes that never add indentation by themselves. A statement list inside
    /// braces, a TOML table, or a Rust `where` clause are examples. The root node always
    /// behaves like this.
    pub transparent_kinds: &'static [&'static str],
    /// Children that are skipped when deciding on which line a node starts. Attributes and
    /// decorators belong here so that the line after `[Test]` is not treated as a
    /// continuation.
    pub prefix_kinds: &'static [&'static str],
    /// Nodes whose brackets may be wrapped when a line is too long (argument lists,
    /// arrays...). The kind is the kind of the node that directly contains the brackets.
    pub wrappable_kinds: &'static [&'static str],
    /// Tokens that use one level less continuation indentation when they start a line:
    /// `else`, `catch`, `{` and similar.
    pub outdent_tokens: &'static [&'static str],
    /// Keywords followed by a space before `(`, like `if (`.
    pub space_before_paren_keywords: &'static [&'static str],
    /// Nodes whose `[` is an index or array-type bracket: no space before it.
    pub index_kinds: &'static [&'static str],
    /// Bracket pairs in addition to `()`, `[]` and `{}`, for example PowerShell `@(` `)`.
    pub extra_brackets: &'static [(&'static str, &'static str)],
    /// Tokens after which a wrapped group may break. Usually just `,`.
    pub separators: &'static [&'static str],
}

impl LanguageRules {
    pub fn is_comment(&self, node: Node) -> bool {
        self.comment_kinds.contains(&node.kind())
    }

    /// Atomic nodes are kept whole. Comments are always atomic.
    pub fn is_atomic(&self, node: Node) -> bool {
        self.atomic_kinds.contains(&node.kind()) || self.is_comment(node)
    }
}

/// A language adapter connects one Tree-sitter grammar to the shared formatting engine.
///
/// Most behavior comes from the tables in [`LanguageRules`]. The methods with default
/// implementations are hooks for rules that tables cannot express.
pub trait LanguageAdapter: Send + Sync {
    /// Short identifier, for example `"rust"`.
    fn name(&self) -> &'static str;

    /// File extensions handled by this adapter, without the dot.
    fn extensions(&self) -> &'static [&'static str];

    fn grammar(&self) -> &Grammar;

    fn rules(&self) -> &LanguageRules;

    /// Language-specific spacing between two tokens on the same line.
    /// Return `None` to use the shared rules.
    fn spacing(&self, _previous: &TokenContext, _next: &TokenContext) -> Option<Spacing> {
        None
    }

    /// Whether the brackets directly inside `list_node` may be wrapped.
    fn is_wrappable(&self, list_node: Node, _open_text: &str) -> bool {
        self.rules().wrappable_kinds.contains(&list_node.kind())
    }

    /// Whether the token must start at column 0 when it begins a line.
    fn column_zero(&self, _token: &TokenContext) -> bool {
        false
    }

    /// Whether the token uses one level less continuation indentation when it begins a line.
    fn outdent(&self, token: &TokenContext) -> bool {
        self.rules().outdent_tokens.contains(&token.text)
    }

    /// For a multi-line atomic node that is not a comment: return `Some(lines)` to allow
    /// re-indenting its lines (all except the listed ones, counted from 0), or `None` to
    /// print it exactly as written.
    fn shift_protected_lines(&self, _tree: &SyntaxTree, _node: Node) -> Option<Vec<usize>> {
        None
    }

    /// Whether the safety check compares this node as a whole instead of child by child.
    fn compare_as_atomic(&self, node: Node) -> bool {
        self.rules().is_atomic(node)
    }

    /// Language-specific text comparison for the safety check. Return `None` to use the
    /// shared rules.
    fn text_matches(&self, _node: Node, _original: &str, _formatted: &str) -> Option<bool> {
        None
    }
}
