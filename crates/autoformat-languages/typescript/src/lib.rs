//! TypeScript support for AutoFormat, based on the `tree-sitter-typescript` grammars.
//!
//! There are two adapters: [`TypeScriptLanguage`] for `.ts` files and [`TsxLanguage`] for
//! `.tsx` files (TypeScript with JSX markup). They share all rules; only the grammar
//! differs. Rules shared with JavaScript come from `autoformat_lang_javascript::ecmascript`.

use autoformat_core::{LanguageAdapter, LanguageRules, Spacing, TokenContext};
use autoformat_lang_javascript::ecmascript;
use autoformat_parser::tree_sitter::Node;
use autoformat_parser::{Grammar, SyntaxTree};

static RULES: LanguageRules = LanguageRules {
    comment_kinds: ecmascript::COMMENT_KINDS,
    atomic_kinds: &[
        "string",
        "template_string",
        "template_literal_type",
        "regex",
        "hash_bang_line",
        "jsx_element",
        "jsx_self_closing_element",
    ],
    // A union written one member per line keeps all members at the same level.
    transparent_kinds: &[
        "union_type",
        "intersection_type",
        "else_clause",
        "catch_clause",
        "finally_clause",
    ],
    prefix_kinds: &["decorator"],
    wrappable_kinds: &[
        "arguments",
        "formal_parameters",
        "array",
        "object",
        "object_pattern",
        "array_pattern",
        "named_imports",
        "export_clause",
        "object_type",
        "interface_body",
        "enum_body",
        "tuple_type",
    ],
    outdent_tokens: ecmascript::OUTDENT_TOKENS,
    space_before_paren_keywords: ecmascript::SPACE_BEFORE_PAREN_KEYWORDS,
    index_kinds: &["subscript_expression", "array_type", "lookup_type"],
    extra_brackets: &[],
    separators: &[",", ";"],
    // Namespaces (`namespace X {`) are not listed: the grammar reads `namespace X` with
    // the `{` on the next line as different code, so their braces stay where they are.
    block_braces: &[
        ("statement_block", "function_declaration"),
        ("statement_block", "generator_function_declaration"),
        ("statement_block", "method_definition"),
        ("class_body", "class_declaration"),
        ("class_body", "abstract_class_declaration"),
        ("interface_body", "interface_declaration"),
        ("enum_body", "enum_declaration"),
        ("statement_block", "if_statement"),
        ("statement_block", "else_clause"),
        ("statement_block", "for_statement"),
        ("statement_block", "for_in_statement"),
        ("statement_block", "while_statement"),
        ("statement_block", "do_statement"),
        ("statement_block", "try_statement"),
        ("statement_block", "catch_clause"),
        ("statement_block", "finally_clause"),
        ("switch_body", "switch_statement"),
    ],
    brace_continuations: ecmascript::BRACE_CONTINUATIONS,
};

/// Spacing rules for TypeScript, falling back to the shared ECMAScript rules.
fn typescript_spacing(previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
    // Optional and definite members: `name?: string`, `name!: string`, `method?()`.
    if ecmascript::is_type_marker(next) || ecmascript::is_type_marker(previous) {
        return Some(Spacing::NoSpace);
    }
    // Mapped type modifiers: `{ [K in Keys]?: T }`, `-?:`, `+?:`.
    let mapped_markers = ["?:", "-?:", "+?:"];
    if mapped_markers.contains(&next.text) {
        return Some(Spacing::NoSpace);
    }
    if mapped_markers.contains(&previous.text) {
        return Some(Spacing::Space);
    }
    // Type parameter defaults: `<T = string>`.
    if (next.is("=") && next.parent_kind == "default_type")
        || (previous.is("=") && previous.parent_kind == "default_type")
    {
        return Some(Spacing::Space);
    }
    // A leading `|` or `&` in a type: `type A =\n    | B\n    | C`.
    if (previous.is("|") && previous.parent_kind == "union_type")
        || (previous.is("&") && previous.parent_kind == "intersection_type")
    {
        return Some(Spacing::Space);
    }
    ecmascript::spacing(previous, next)
}

pub struct TypeScriptLanguage {
    grammar: Grammar,
}

impl TypeScriptLanguage {
    pub fn new() -> TypeScriptLanguage {
        TypeScriptLanguage {
            grammar: Grammar::new(
                "typescript",
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            ),
        }
    }
}

impl Default for TypeScriptLanguage {
    fn default() -> TypeScriptLanguage {
        TypeScriptLanguage::new()
    }
}

impl LanguageAdapter for TypeScriptLanguage {
    fn name(&self) -> &'static str {
        "typescript"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ts", "mts", "cts"]
    }

    fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    fn rules(&self) -> &LanguageRules {
        &RULES
    }

    fn spacing(&self, previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
        typescript_spacing(previous, next)
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

pub struct TsxLanguage {
    grammar: Grammar,
}

impl TsxLanguage {
    pub fn new() -> TsxLanguage {
        TsxLanguage {
            grammar: Grammar::new("tsx", tree_sitter_typescript::LANGUAGE_TSX.into()),
        }
    }
}

impl Default for TsxLanguage {
    fn default() -> TsxLanguage {
        TsxLanguage::new()
    }
}

impl LanguageAdapter for TsxLanguage {
    fn name(&self) -> &'static str {
        "tsx"
    }

    /// `.tsx` files use the `[formatting.typescript]` settings.
    fn config_name(&self) -> &'static str {
        "typescript"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["tsx"]
    }

    fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    fn rules(&self) -> &LanguageRules {
        &RULES
    }

    fn spacing(&self, previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
        typescript_spacing(previous, next)
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
