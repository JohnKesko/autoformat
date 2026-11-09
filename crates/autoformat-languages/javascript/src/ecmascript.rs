//! Rules shared by JavaScript and TypeScript.
//!
//! JSX markup (`<div>...</div>`) is treated as one block: its text is never respaced.
//! When the line it starts on moves left or right, the following lines of the markup move
//! by the same amount. This is safe because JSX ignores the indentation of text lines.
//! Lines that start inside a multi-line string are never moved.

use autoformat_core::{LanguageRules, Role, Spacing, TokenContext};
use autoformat_parser::tree_sitter::Node;
use autoformat_parser::SyntaxTree;

pub const COMMENT_KINDS: &[&str] = &["comment", "html_comment"];

pub const JSX_KINDS: &[&str] = &["jsx_element", "jsx_self_closing_element"];

pub const ATOMIC_KINDS: &[&str] = &[
    "string",
    "template_string",
    "regex",
    "hash_bang_line",
    "jsx_element",
    "jsx_self_closing_element",
];

pub const WRAPPABLE_KINDS: &[&str] = &[
    "arguments",
    "formal_parameters",
    "array",
    "object",
    "object_pattern",
    "array_pattern",
    "named_imports",
    "export_clause",
];

/// Tokens that do not get continuation indentation when they start a line. A leading `;`
/// (`;(a || b).run()`, common in code without semicolons) belongs to the statement level.
pub const OUTDENT_TOKENS: &[&str] = &["{", "else", "catch", "finally", ";"];

pub const SPACE_BEFORE_PAREN_KEYWORDS: &[&str] = &["if", "for", "while", "switch", "catch", "with"];

/// Containers that never add indentation. Making the `else`, `catch` and `finally`
/// clauses transparent keeps a `{` on its own line after them at the statement level.
pub const TRANSPARENT_KINDS: &[&str] = &["else_clause", "catch_clause", "finally_clause"];

/// Code blocks whose braces follow `brace_style`. Arrow functions, function expressions
/// and object literals are not listed: their braces stay where they are.
pub const BLOCK_BRACES: &[(&str, &str)] = &[
    ("statement_block", "function_declaration"),
    ("statement_block", "generator_function_declaration"),
    ("statement_block", "method_definition"),
    ("class_body", "class_declaration"),
    ("statement_block", "if_statement"),
    ("statement_block", "else_clause"),
    ("statement_block", "for_statement"),
    ("statement_block", "for_in_statement"),
    ("statement_block", "while_statement"),
    ("statement_block", "do_statement"),
    ("statement_block", "try_statement"),
    ("statement_block", "catch_clause"),
    ("statement_block", "finally_clause"),
    ("statement_block", "with_statement"),
    ("switch_body", "switch_statement"),
];

/// Declarations that get a blank line around them when they span several lines.
/// `const` and `let` are not listed: in scripts they are ordinary code.
pub const DECLARATION_KINDS: &[&str] = &[
    "function_declaration",
    "generator_function_declaration",
    "class_declaration",
    "method_definition",
    "field_definition",
    "export_statement",
];

/// Keywords that continue a statement after a block's closing brace.
pub const BRACE_CONTINUATIONS: &[(&str, &str)] = &[
    ("else", "else_clause"),
    ("catch", "catch_clause"),
    ("finally", "finally_clause"),
    ("while", "do_statement"),
];

const GENERATOR_KINDS: &[&str] = &["generator_function_declaration", "generator_function"];

/// Spacing rules shared by JavaScript and TypeScript.
pub fn spacing(previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
    // Generators: `function* name()`, `yield* other()`.
    if next.is("*")
        && (GENERATOR_KINDS.contains(&next.parent_kind) || next.parent_kind == "yield_expression")
    {
        return Some(Spacing::NoSpace);
    }
    if previous.is("*")
        && (GENERATOR_KINDS.contains(&previous.parent_kind)
            || previous.parent_kind == "yield_expression")
    {
        return Some(Spacing::Space);
    }
    // Generator methods: `*items() {}`.
    if previous.is("*") && previous.parent_kind == "method_definition" {
        return Some(Spacing::NoSpace);
    }
    None
}

/// For a JSX block: the lines (counted from 0 within the block) that start inside a
/// multi-line string or template and must not be moved. Returns `None` for other nodes.
pub fn jsx_protected_lines(_tree: &SyntaxTree, node: Node) -> Option<Vec<usize>> {
    if !JSX_KINDS.contains(&node.kind()) {
        return None;
    }
    let first_row = node.start_position().row;
    let mut protected = Vec::new();
    collect_protected_lines(node, first_row, &mut protected);
    Some(protected)
}

fn collect_protected_lines(node: Node, first_row: usize, protected: &mut Vec<usize>) {
    let kind = node.kind();
    if kind == "string" || kind == "template_string" {
        let start_row = node.start_position().row;
        let end_row = node.end_position().row;
        for row in start_row + 1..=end_row {
            protected.push(row - first_row);
        }
        return;
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            collect_protected_lines(cursor.node(), first_row, protected);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// JSX blocks are compared child by child so that their text can be compared with JSX
/// whitespace rules.
pub fn compare_as_atomic(rules: &LanguageRules, node: Node) -> bool {
    if JSX_KINDS.contains(&node.kind()) {
        return false;
    }
    rules.is_atomic(node)
}

pub fn text_matches(node: Node, original: &str, formatted: &str) -> Option<bool> {
    if node.kind() == "jsx_text" {
        return Some(jsx_visible_text(original) == jsx_visible_text(formatted));
    }
    None
}

/// The text that JSX actually produces from a piece of JSX text, following the rules
/// used by Babel and TypeScript: lines are trimmed (except the outer edges of the first
/// and last line), empty lines are dropped, and lines are joined with a single space.
pub fn jsx_visible_text(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut last_non_empty = 0;
    for (index, line) in lines.iter().enumerate() {
        if line.chars().any(|c| c != ' ' && c != '\t') {
            last_non_empty = index;
        }
    }
    let mut result = String::new();
    for (index, line) in lines.iter().enumerate() {
        let is_first = index == 0;
        let is_last = index == lines.len() - 1;
        let mut trimmed = line.replace('\t', " ");
        if !is_first {
            trimmed = trimmed.trim_start_matches(' ').to_string();
        }
        if !is_last {
            trimmed = trimmed.trim_end_matches(' ').to_string();
        }
        if !trimmed.is_empty() {
            if index != last_non_empty {
                trimmed.push(' ');
            }
            result.push_str(&trimmed);
        }
    }
    result
}

/// Whether a token is a `?` or `!` that marks an optional or definite member in
/// TypeScript (`name?: string`, `name!: string`), as opposed to an operator.
pub fn is_type_marker(token: &TokenContext) -> bool {
    if !(token.is("?") || token.is("!")) {
        return false;
    }
    let operator_parents = [
        "ternary_expression",
        "conditional_type",
        "unary_expression",
        "non_null_expression",
        "optional_type",
    ];
    !operator_parents.contains(&token.parent_kind) && token.role != Role::Postfix
}

#[cfg(test)]
mod tests {
    use super::jsx_visible_text;

    #[test]
    fn jsx_text_ignores_indentation() {
        assert_eq!(
            jsx_visible_text("\n    Hello "),
            jsx_visible_text("\n        Hello ")
        );
        assert_eq!(
            jsx_visible_text(" world\n    "),
            jsx_visible_text(" world\n")
        );
        assert_ne!(jsx_visible_text("a b"), jsx_visible_text("ab"));
        assert_ne!(jsx_visible_text("a  b"), jsx_visible_text("a b"));
        assert_eq!(
            jsx_visible_text("\n  line one\n  line two\n"),
            "line one line two"
        );
    }
}
