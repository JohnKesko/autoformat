//! Java support for AutoFormat, based on the `tree-sitter-java` grammar.
//!
//! Special cases handled here:
//! - Generics have no spaces inside the angle brackets: `Map<String, List<Integer>>`.
//! - Annotations (`@Override`) belong to the declaration after them, so the line after an
//!   annotation is not indented.
//! - Array initializers have no spaces inside the braces: `{1, 2, 3}`. They follow
//!   `column_width` like lists in `( )`.
//! - A cast is followed by one space: `(int) value`.
//! - `for (String name : names)` keeps a space before the colon.
//! - With `blank_lines` on, one empty line follows the `package` line and the imports.
//! - Strings, text blocks (`"""`) and characters are never changed.

use autoformat_core::{LanguageAdapter, LanguageRules, Role, Spacing, TokenContext};
use autoformat_parser::tree_sitter::Node;
use autoformat_parser::Grammar;

pub struct JavaLanguage {
    grammar: Grammar,
}

impl JavaLanguage {
    pub fn new() -> JavaLanguage {
        JavaLanguage {
            grammar: Grammar::new("java", tree_sitter_java::LANGUAGE.into()),
        }
    }
}

impl Default for JavaLanguage {
    fn default() -> JavaLanguage {
        JavaLanguage::new()
    }
}

/// The two kinds of annotation: `@Override` and `@SuppressWarnings("unchecked")`.
const ANNOTATION_KINDS: &[&str] = &["marker_annotation", "annotation"];

/// Nodes that hold the angle brackets of generics: `List<String>` and `class Box<T>`.
const ANGLE_BRACKET_KINDS: &[&str] = &["type_arguments", "type_parameters"];

/// Nodes that hold the braces of an array initializer: `{1, 2, 3}`.
const ARRAY_INITIALIZER_KINDS: &[&str] = &["array_initializer", "element_value_array_initializer"];

static RULES: LanguageRules = LanguageRules {
    comment_kinds: &["line_comment", "block_comment"],
    // A text block is a `string_literal` too, so it is printed exactly as written.
    atomic_kinds: &["string_literal", "character_literal"],
    // `modifiers` holds the annotations and words like `public` in front of a
    // declaration. A label (`outer:`) does not indent the loop after it. The part of an
    // enum after its values (`enum_body_declarations`) is indented like the values.
    transparent_kinds: &[
        "modifiers",
        "catch_clause",
        "finally_clause",
        "enum_body_declarations",
        "labeled_statement",
    ],
    prefix_kinds: &["marker_annotation", "annotation"],
    wrappable_kinds: &[
        "argument_list",
        "formal_parameters",
        "inferred_parameters",
        "annotation_argument_list",
    ],
    outdent_tokens: &["{", "else", "catch", "finally"],
    space_before_paren_keywords: &[
        "if",
        "for",
        "while",
        "switch",
        "catch",
        "try",
        "synchronized",
    ],
    index_kinds: &["array_access", "dimensions", "dimensions_expr"],
    extra_brackets: &[],
    separators: &[","],
    // Lambdas, anonymous classes, the bodies of enum values, array initializers and
    // the block of a `case 1: { }` are not listed: their braces stay where they are.
    block_braces: &[
        ("class_body", "class_declaration"),
        ("class_body", "record_declaration"),
        ("interface_body", "interface_declaration"),
        ("enum_body", "enum_declaration"),
        ("annotation_type_body", "annotation_type_declaration"),
        ("block", "method_declaration"),
        ("constructor_body", "constructor_declaration"),
        ("block", "compact_constructor_declaration"),
        ("block", "static_initializer"),
        ("block", "if_statement"),
        ("block", "for_statement"),
        ("block", "enhanced_for_statement"),
        ("block", "while_statement"),
        ("block", "do_statement"),
        ("block", "try_statement"),
        ("block", "try_with_resources_statement"),
        ("block", "catch_clause"),
        ("block", "finally_clause"),
        ("block", "synchronized_statement"),
        ("switch_block", "switch_expression"),
        ("block", "switch_rule"),
    ],
    brace_continuations: &[
        ("else", "if_statement"),
        ("catch", "catch_clause"),
        ("finally", "finally_clause"),
        ("while", "do_statement"),
    ],
    declaration_containers: &[
        "program",
        "class_body",
        "interface_body",
        "annotation_type_body",
        "enum_body_declarations",
    ],
    declaration_kinds: &[
        "class_declaration",
        "interface_declaration",
        "enum_declaration",
        "record_declaration",
        "annotation_type_declaration",
        "method_declaration",
        "constructor_declaration",
        "compact_constructor_declaration",
        "static_initializer",
        "field_declaration",
        "constant_declaration",
        "annotation_type_element_declaration",
    ],
};

impl LanguageAdapter for JavaLanguage {
    fn name(&self) -> &'static str {
        "java"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["java"]
    }

    fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    fn rules(&self) -> &LanguageRules {
        &RULES
    }

    fn always_separated(&self, before: Node, after: Node) -> bool {
        // One empty line after the `package` line and after the last import.
        match before.kind() {
            "package_declaration" => true,
            "import_declaration" => after.kind() != "import_declaration",
            _ => false,
        }
    }

    fn is_array_in_braces(&self, list_node: Node) -> bool {
        ARRAY_INITIALIZER_KINDS.contains(&list_node.kind())
    }

    fn spacing(&self, previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
        let angle_spacing = angle_bracket_spacing(previous, next);
        if angle_spacing.is_some() {
            return angle_spacing;
        }

        // `String... names`, `Class<?>... types`. `Never` is safe here: `...` cannot
        // run together with the type before it.
        if next.is("...") {
            return Some(Spacing::Never);
        }
        if previous.is("...") {
            return Some(Spacing::Space);
        }

        // `? extends Number` and `? super Integer` inside generics.
        let after_wildcard = previous.is("?") && previous.parent_kind == "wildcard";
        if after_wildcard && (next.is("extends") || next.is("super")) {
            return Some(Spacing::Space);
        }

        // `for (String name : names)` and `assert ok : "message"`
        let spaced_colon_parents = ["enhanced_for_statement", "assert_statement"];
        if next.is(":") && spaced_colon_parents.contains(&next.parent_kind) {
            return Some(Spacing::Space);
        }
        if previous.is(":") && spaced_colon_parents.contains(&previous.parent_kind) {
            return Some(Spacing::Space);
        }

        // `(int) value`
        if previous.is(")") && previous.parent_kind == "cast_expression" {
            return Some(Spacing::Space);
        }

        // `if (done) return;`, `void run() throws IOException`, `@Size(max = 5) String`
        if previous.is(")") && matches!(next.role, Role::Word | Role::Keyword) {
            return Some(Spacing::Space);
        }

        // `{1, 2, 3}`
        if previous.is("{") && ARRAY_INITIALIZER_KINDS.contains(&previous.parent_kind) {
            return Some(Spacing::NoSpace);
        }
        if next.is("}") && ARRAY_INITIALIZER_KINDS.contains(&next.parent_kind) {
            return Some(Spacing::NoSpace);
        }

        // `public @Nullable String name`, `@A @B void run()`
        let before_annotation = next.is("@") && ANNOTATION_KINDS.contains(&next.parent_kind);
        if before_annotation && matches!(previous.role, Role::Word | Role::Keyword) {
            return Some(Spacing::Space);
        }
        None
    }
}

/// Spacing around the `<` and `>` of generics. `None` for every other pair of tokens.
fn angle_bracket_spacing(previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
    let previous_is_angle = ANGLE_BRACKET_KINDS.contains(&previous.parent_kind);
    let next_is_angle = ANGLE_BRACKET_KINDS.contains(&next.parent_kind);

    // Nothing after `<` and nothing before `>`: `<String>`, `<>`, `<?>`, `<List<T>>`.
    // `Never` is safe here: Java reads `<>` and `>>` in generics as separate brackets.
    if previous.is("<") && previous_is_angle {
        return Some(Spacing::Never);
    }
    if next.is(">") && next_is_angle {
        return Some(Spacing::Never);
    }

    if next.is("<") && next_is_angle {
        // `public <T> T first()`: the brackets come before the method, after a word.
        let owner = next.grandparent_kind();
        let before_method = owner == "method_declaration" || owner == "constructor_declaration";
        if before_method || previous.role == Role::Keyword {
            return Some(Spacing::Space);
        }
        // `List<String>`, `class Box<T>`, `Collections.<String>emptyList()`
        return Some(Spacing::NoSpace);
    }

    if previous.is(">") && previous_is_angle {
        // `new ArrayList<>()`
        if next.is("(") {
            return Some(Spacing::NoSpace);
        }
        if matches!(next.role, Role::Word | Role::Keyword) {
            // `Collections.<String>emptyList()`, `Lists::<String>of`
            let owner = previous.grandparent_kind();
            if owner == "method_invocation" || owner == "method_reference" {
                return Some(Spacing::NoSpace);
            }
            // `List<String> names`, `class Box<T> extends Base`
            return Some(Spacing::Space);
        }
    }
    None
}
