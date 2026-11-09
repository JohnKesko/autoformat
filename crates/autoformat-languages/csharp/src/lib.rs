//! C# support for AutoFormat, based on the `tree-sitter-c-sharp` grammar.
//!
//! Special cases handled here:
//! - Preprocessor directives such as `#if` and `#endif` start at column 0, the way Visual
//!   Studio formats them. `#region` and `#endregion` are indented like the code around
//!   them.
//! - `class A : B` and `where T : class` keep a space before the colon.
//! - Null-conditional access `a?.b` has no spaces.
//! - Interpolated, verbatim and raw strings are never changed.

use autoformat_core::{LanguageAdapter, LanguageRules, Spacing, TokenContext};
use autoformat_parser::Grammar;

pub struct CSharpLanguage {
    grammar: Grammar,
}

impl CSharpLanguage {
    pub fn new() -> CSharpLanguage {
        CSharpLanguage {
            grammar: Grammar::new("csharp", tree_sitter_c_sharp::LANGUAGE.into()),
        }
    }
}

impl Default for CSharpLanguage {
    fn default() -> CSharpLanguage {
        CSharpLanguage::new()
    }
}

/// Directive lines that are printed exactly as written.
const DIRECTIVE_KINDS: &[&str] = &[
    "preproc_region",
    "preproc_endregion",
    "preproc_define",
    "preproc_undef",
    "preproc_line",
    "preproc_error",
    "preproc_warning",
    "preproc_pragma",
    "preproc_nullable",
];

static RULES: LanguageRules = LanguageRules {
    // `#region` lines behave like comments: they are indented like the code after them.
    comment_kinds: &["comment", "preproc_region", "preproc_endregion"],
    atomic_kinds: &[
        "string_literal",
        "verbatim_string_literal",
        "raw_string_literal",
        "interpolated_string_expression",
        "character_literal",
        "preproc_region",
        "preproc_endregion",
        "preproc_define",
        "preproc_undef",
        "preproc_line",
        "preproc_error",
        "preproc_warning",
        "preproc_pragma",
        "preproc_nullable",
    ],
    transparent_kinds: &[
        "file_scoped_namespace_declaration",
        "preproc_if",
        "preproc_elif",
        "preproc_else",
    ],
    prefix_kinds: &["attribute_list"],
    wrappable_kinds: &[
        "argument_list",
        "parameter_list",
        "attribute_argument_list",
        "initializer_expression",
        "collection_expression",
        "tuple_expression",
        "enum_member_declaration_list",
        "switch_expression",
        "anonymous_object_creation_expression",
    ],
    outdent_tokens: &["{", "else", "catch", "finally"],
    space_before_paren_keywords: &[
        "if", "for", "foreach", "while", "switch", "catch", "using", "lock", "fixed", "when",
    ],
    index_kinds: &["bracketed_argument_list", "array_rank_specifier"],
    extra_brackets: &[],
    separators: &[","],
};

impl LanguageAdapter for CSharpLanguage {
    fn name(&self) -> &'static str {
        "csharp"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["cs", "csx"]
    }

    fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    fn rules(&self) -> &LanguageRules {
        &RULES
    }

    fn column_zero(&self, token: &TokenContext) -> bool {
        // `#if`, `#elif`, `#else`, `#endif`, `#define`, `#pragma` ... but not `#region`.
        if !token.text.starts_with('#') {
            return false;
        }
        let is_region = token.kind == "preproc_region" || token.kind == "preproc_endregion";
        let is_directive = token.kind.starts_with('#') || DIRECTIVE_KINDS.contains(&token.kind);
        is_directive && !is_region
    }

    fn spacing(&self, previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
        // Directive lines keep their own spacing: `#if DEBUG`.
        if previous.text.starts_with('#') || previous.parent_kind.starts_with("preproc") {
            return Some(Spacing::Keep);
        }

        // `a?.b` and `a?[0]`
        if next.is("?") && next.parent_kind == "conditional_access_expression" {
            return Some(Spacing::NoSpace);
        }
        if previous.is("?") && previous.parent_kind == "conditional_access_expression" {
            return Some(Spacing::NoSpace);
        }

        // Relational patterns: `x is < 18`, `{ Age: >= 65 }`.
        if previous.parent_kind == "relational_pattern" && previous.role == autoformat_core::Role::Prefix {
            return Some(Spacing::Space);
        }

        // `class A : B`, `where T : class`, `Foo() : base()`
        let spaced_colon_parents = [
            "base_list",
            "type_parameter_constraints_clause",
            "constructor_initializer",
        ];
        if next.is(":") && spaced_colon_parents.contains(&next.parent_kind) {
            return Some(Spacing::Space);
        }
        if previous.is(":") && spaced_colon_parents.contains(&previous.parent_kind) {
            return Some(Spacing::Space);
        }
        None
    }
}
