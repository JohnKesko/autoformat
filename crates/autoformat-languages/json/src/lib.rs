//! JSON and JSONC support for AutoFormat, based on the `tree-sitter-json` grammar.
//!
//! JSONC is JSON with comments (`// ...` and `/* ... */`). The grammar reads comments in
//! every file, so one adapter handles both `.json` and `.jsonc` files.
//!
//! Only whitespace changes. Strings, numbers and comments are printed exactly as written,
//! and no comma is ever added or removed.
//!
//! Arrays and objects are put on one line when they fit and one item per line when they
//! do not, like `( )` and `[ ]` lists in every other language. One with a comment inside
//! keeps its layout as written.
//!
//! Two things the grammar does not accept, so a file with one of them is reported as a
//! syntax error and left unchanged:
//! - a comma after the last item (`[1, 2,]`),
//! - a number with a plus sign in its exponent (`1e+5`; `1e5` and `1e-5` are fine).

use autoformat_core::{LanguageAdapter, LanguageRules, Spacing, TokenContext};
use autoformat_parser::tree_sitter::Node;
use autoformat_parser::Grammar;

pub struct JsonLanguage {
    grammar: Grammar,
}

impl JsonLanguage {
    pub fn new() -> JsonLanguage {
        JsonLanguage {
            grammar: Grammar::new("json", tree_sitter_json::LANGUAGE.into()),
        }
    }
}

impl Default for JsonLanguage {
    fn default() -> JsonLanguage {
        JsonLanguage::new()
    }
}

static RULES: LanguageRules = LanguageRules {
    comment_kinds: &["comment"],
    atomic_kinds: &["string"],
    transparent_kinds: &[],
    prefix_kinds: &[],
    wrappable_kinds: &["array"],
    outdent_tokens: &[],
    space_before_paren_keywords: &[],
    index_kinds: &[],
    extra_brackets: &[],
    separators: &[","],
    // JSON has no code blocks: braces are objects, which stay where they are.
    block_braces: &[],
    brace_continuations: &[],
    // JSON has no declarations, so `blank_lines` never adds a blank line.
    declaration_containers: &[],
    declaration_kinds: &[],
};

impl LanguageAdapter for JsonLanguage {
    fn name(&self) -> &'static str {
        "json"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["json", "jsonc"]
    }

    fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    fn rules(&self) -> &LanguageRules {
        &RULES
    }

    fn is_array_in_braces(&self, list_node: Node) -> bool {
        // In JSON an object is a plain list of `"name": value` pairs, so it follows
        // `column_width` the same way an array does.
        list_node.kind() == "object"
    }

    fn spacing(&self, previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
        // An empty object is always written `{}`.
        if previous.is("{") && next.is("}") {
            return Some(Spacing::NoSpace);
        }
        None
    }
}
