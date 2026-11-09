//! JavaScript support for AutoFormat, based on the `tree-sitter-javascript` grammar.
//!
//! The [`ecmascript`] module holds the rules that JavaScript and TypeScript share. The
//! TypeScript adapter uses it too, because TypeScript is JavaScript with types.

pub mod ecmascript;

use autoformat_core::{LanguageAdapter, LanguageRules, Spacing, TokenContext};
use autoformat_parser::tree_sitter::Node;
use autoformat_parser::{Grammar, SyntaxTree};

pub struct JavaScriptLanguage {
    grammar: Grammar,
}

impl JavaScriptLanguage {
    pub fn new() -> JavaScriptLanguage {
        JavaScriptLanguage {
            grammar: Grammar::new("javascript", tree_sitter_javascript::LANGUAGE.into()),
        }
    }
}

impl Default for JavaScriptLanguage {
    fn default() -> JavaScriptLanguage {
        JavaScriptLanguage::new()
    }
}

static RULES: LanguageRules = LanguageRules {
    comment_kinds: ecmascript::COMMENT_KINDS,
    atomic_kinds: ecmascript::ATOMIC_KINDS,
    transparent_kinds: ecmascript::TRANSPARENT_KINDS,
    prefix_kinds: &["decorator"],
    wrappable_kinds: ecmascript::WRAPPABLE_KINDS,
    outdent_tokens: ecmascript::OUTDENT_TOKENS,
    space_before_paren_keywords: ecmascript::SPACE_BEFORE_PAREN_KEYWORDS,
    index_kinds: &["subscript_expression"],
    extra_brackets: &[],
    separators: &[","],
    block_braces: ecmascript::BLOCK_BRACES,
    brace_continuations: ecmascript::BRACE_CONTINUATIONS,
};

impl LanguageAdapter for JavaScriptLanguage {
    fn name(&self) -> &'static str {
        "javascript"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["js", "mjs", "cjs", "jsx"]
    }

    fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    fn rules(&self) -> &LanguageRules {
        &RULES
    }

    fn spacing(&self, previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
        ecmascript::spacing(previous, next)
    }

    fn shift_protected_lines(&self, tree: &SyntaxTree, node: Node) -> Option<Vec<usize>> {
        ecmascript::jsx_protected_lines(tree, node)
    }

    fn compare_as_atomic(&self, node: Node) -> bool {
        ecmascript::compare_as_atomic(&RULES, node)
    }

    fn text_matches(&self, node: Node, original: &str, formatted: &str) -> Option<bool> {
        ecmascript::text_matches(node, original, formatted)
    }
}
