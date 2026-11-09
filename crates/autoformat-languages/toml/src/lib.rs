//! TOML support for AutoFormat, based on the `tree-sitter-toml-ng` grammar.
//!
//! Keys inside tables are not indented. Arrays may be wrapped; inline tables never are,
//! because TOML 1.0 requires an inline table to stay on one line.

use autoformat_core::{LanguageAdapter, LanguageRules, Spacing, TokenContext};
use autoformat_parser::Grammar;

pub struct TomlLanguage {
    grammar: Grammar,
}

impl TomlLanguage {
    pub fn new() -> TomlLanguage {
        TomlLanguage {
            grammar: Grammar::new("toml", tree_sitter_toml_ng::LANGUAGE.into()),
        }
    }
}

impl Default for TomlLanguage {
    fn default() -> TomlLanguage {
        TomlLanguage::new()
    }
}

static RULES: LanguageRules = LanguageRules {
    comment_kinds: &["comment"],
    atomic_kinds: &["string", "quoted_key"],
    transparent_kinds: &["table", "table_array_element"],
    prefix_kinds: &[],
    wrappable_kinds: &["array"],
    outdent_tokens: &[],
    space_before_paren_keywords: &[],
    index_kinds: &[],
    extra_brackets: &[],
    separators: &[","],
};

impl LanguageAdapter for TomlLanguage {
    fn name(&self) -> &'static str {
        "toml"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["toml"]
    }

    fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    fn rules(&self) -> &LanguageRules {
        &RULES
    }

    fn spacing(&self, previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
        // `[[products]]`
        if previous.is("[[") || next.is("]]") {
            return Some(Spacing::NoSpace);
        }
        None
    }
}
