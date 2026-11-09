//! Information about a token and its place in the syntax tree, used by spacing rules.

use autoformat_parser::tree_sitter::Node;
use autoformat_parser::ParentMap;

/// What a token does, as far as spacing is concerned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// An opening bracket of a group.
    Open,
    /// A closing bracket of a group.
    Close,
    Comma,
    Semicolon,
    /// A `:` that separates a name from a type or value (`x: i32`, `key: value`).
    Colon,
    /// Tokens printed without spaces around them: `.`, `::`, `?.`, `..`.
    Tight,
    /// An operator between two operands (`a + b`, `x = 1`, `a ? b : c`).
    Infix,
    /// An operator in front of its operand (`-x`, `!x`, `&x`).
    Prefix,
    /// An operator after its operand (`x?`, `x++`, `x!`).
    Postfix,
    /// A named leaf made of letters or digits: identifiers, numbers, type names.
    Word,
    /// An anonymous leaf made of letters: keywords like `if`, `fn`, `return`.
    Keyword,
    Comment,
    /// Text the grammar left outside of every node (for example a PowerShell backtick).
    Gap,
    Other,
}

/// A token as seen by the spacing rules.
#[derive(Clone, Copy)]
pub struct TokenContext<'tree> {
    pub index: usize,
    pub text: &'tree str,
    /// The syntax node of the token. `None` for gap text.
    pub node: Option<Node<'tree>>,
    /// The node kind, or `""` for gap text.
    pub kind: &'tree str,
    /// The kind of the parent node, or `""`.
    pub parent_kind: &'tree str,
    pub role: Role,
    pub had_space_before: bool,
    /// Used to find parents and ancestors quickly.
    pub parents: &'tree ParentMap<'tree>,
}

impl<'tree> TokenContext<'tree> {
    pub fn is(&self, text: &str) -> bool {
        self.text == text
    }

    pub fn parent(&self) -> Option<Node<'tree>> {
        match self.node {
            Some(node) => self.parents.parent(node),
            None => None,
        }
    }

    /// The kind of the grandparent node, or `""`.
    pub fn grandparent_kind(&self) -> &'tree str {
        match self.parent().and_then(|parent| self.parents.parent(parent)) {
            Some(node) => node.kind(),
            None => "",
        }
    }

    /// Whether any ancestor (not the token itself) has one of the given kinds.
    pub fn has_ancestor(&self, kinds: &[&str]) -> bool {
        let mut current = self.parent();
        while let Some(node) = current {
            if kinds.contains(&node.kind()) {
                return true;
            }
            current = self.parents.parent(node);
        }
        false
    }
}

/// Characters that make up operators.
pub(crate) fn is_operator_text(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| "=<>!+-*/%&|^~?:.@#".contains(c))
}

pub(crate) fn is_word_text(text: &str) -> bool {
    match text.chars().next() {
        Some(first) => first.is_alphanumeric() || first == '_' || first == '$' || first == '@',
        None => false,
    }
}

/// Tokens that never have spaces around them.
pub(crate) const TIGHT_TOKENS: &[&str] = &[".", "?.", "::", "..", "..="];

/// Finds whether an anonymous operator token sits between operands, before one, or after
/// one, by looking at its named siblings (and `_`, which is an anonymous operand).
pub(crate) fn operator_position(node: Node, parents: &ParentMap) -> Role {
    let parent = match parents.parent(node) {
        Some(parent) => parent,
        None => return Role::Other,
    };
    let mut named_before = false;
    let mut named_after = false;
    let mut seen_self = false;
    let mut cursor = parent.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.id() == node.id() {
                seen_self = true;
            } else if (child.is_named() || child.kind() == "_") && !child.is_extra() {
                if seen_self {
                    named_after = true;
                } else {
                    named_before = true;
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    match (named_before, named_after) {
        (true, true) => Role::Infix,
        (false, true) => Role::Prefix,
        (true, false) => Role::Postfix,
        (false, false) => Role::Other,
    }
}

/// Whether a `:` belongs to a conditional expression (`a ? b : c`).
pub(crate) fn is_conditional_colon(node: Node, parents: &ParentMap) -> bool {
    let parent = match parents.parent(node) {
        Some(parent) => parent,
        None => return false,
    };
    let mut cursor = parent.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if !child.is_named() && child.kind() == "?" {
                return true;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    false
}
