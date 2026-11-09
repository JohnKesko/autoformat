//! Rust support for AutoFormat, based on the `tree-sitter-rust` grammar.
//!
//! Special cases handled here:
//! - Macro invocations (`println!(...)`, `vec![...]`): the grammar does not know what is
//!   inside a macro, so only commas, semicolons and brackets are spaced there; all other
//!   spacing is kept as written.
//! - `macro_rules!` definitions keep their spacing completely.
//! - `use a::{b, c}` has no spaces inside the braces.

use autoformat_core::{LanguageAdapter, LanguageRules, Role, Spacing, TokenContext};
use autoformat_parser::tree_sitter::Node;
use autoformat_parser::Grammar;

pub struct RustLanguage {
    grammar: Grammar,
}

impl RustLanguage {
    pub fn new() -> RustLanguage {
        RustLanguage {
            grammar: Grammar::new("rust", tree_sitter_rust::LANGUAGE.into()),
        }
    }
}

impl Default for RustLanguage {
    fn default() -> RustLanguage {
        RustLanguage::new()
    }
}

static RULES: LanguageRules = LanguageRules {
    comment_kinds: &["line_comment", "block_comment"],
    atomic_kinds: &[
        "string_literal",
        "raw_string_literal",
        "char_literal",
        "lifetime",
        "label",
        "shebang",
    ],
    transparent_kinds: &["where_clause"],
    prefix_kinds: &["attribute_item", "inner_attribute_item"],
    wrappable_kinds: &[
        "arguments",
        "parameters",
        "array_expression",
        "tuple_expression",
        "tuple_type",
        "tuple_pattern",
        "tuple_struct_pattern",
        "slice_pattern",
        "struct_pattern",
        "use_list",
        "field_initializer_list",
        "field_declaration_list",
        "ordered_field_declaration_list",
        "enum_variant_list",
        "token_tree",
    ],
    outdent_tokens: &["{", "else", "where"],
    space_before_paren_keywords: &["if", "while", "match", "for", "in"],
    index_kinds: &["index_expression"],
    extra_brackets: &[],
    separators: &[","],
};

impl LanguageAdapter for RustLanguage {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    fn rules(&self) -> &LanguageRules {
        &RULES
    }

    fn is_wrappable(&self, list_node: Node, open_text: &str) -> bool {
        // Brace-delimited macro bodies (`html! { ... }`) are left alone.
        if list_node.kind() == "token_tree" && open_text == "{" {
            return false;
        }
        RULES.wrappable_kinds.contains(&list_node.kind())
    }

    fn spacing(&self, previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
        // `macro_rules!` bodies are written in their own little language: keep them as is.
        if previous.has_ancestor(&["macro_rule"]) || next.has_ancestor(&["macro_rule"]) {
            return Some(Spacing::Keep);
        }

        // The `!` of a macro call: `println!(...)`, `vec![...]`, but `name! { ... }`.
        if next.is("!") && next.parent_kind == "macro_invocation" {
            return Some(Spacing::NoSpace);
        }
        if previous.is("!") && previous.parent_kind == "macro_invocation" {
            if next.is("{") {
                return Some(Spacing::Space);
            }
            return Some(Spacing::NoSpace);
        }

        // Closures: `|item| item.name()`, `move || run()`.
        if previous.is("|") && next.is("|") && next.parent_kind == "closure_parameters" {
            return Some(Spacing::NoSpace);
        }
        if previous.is("|") && previous.parent_kind == "closure_parameters" && is_last_child(previous) {
            return Some(Spacing::Space);
        }
        if next.is("|") && next.parent_kind == "closure_parameters" && previous.role == Role::Keyword {
            return Some(Spacing::Space);
        }

        // `use std::{io, fmt};`
        if previous.is("{") && previous.parent_kind == "use_list" {
            return Some(Spacing::NoSpace);
        }
        if next.is("}") && next.parent_kind == "use_list" {
            return Some(Spacing::NoSpace);
        }

        // Inside macro arguments only commas, semicolons and brackets are spaced.
        if previous.parent_kind == "token_tree" || next.parent_kind == "token_tree" {
            return Some(token_tree_spacing(previous, next));
        }
        None
    }
}

fn is_last_child(token: &TokenContext) -> bool {
    match token.node {
        Some(node) => node.next_sibling().is_none(),
        None => false,
    }
}

/// Spacing inside macro arguments, where the grammar only sees a flat list of tokens.
fn token_tree_spacing(previous: &TokenContext, next: &TokenContext) -> Spacing {
    if previous.role == Role::Comment || next.role == Role::Comment {
        return Spacing::Keep;
    }
    if next.is(",") || next.is(";") {
        return Spacing::NoSpace;
    }
    if next.role == Role::Close && (next.is(")") || next.is("]")) {
        return Spacing::NoSpace;
    }
    if previous.role == Role::Open && (previous.is("(") || previous.is("[")) {
        return Spacing::NoSpace;
    }
    if (previous.is(",") || previous.is(";")) && next.role != Role::Close {
        return Spacing::Space;
    }
    Spacing::Keep
}
