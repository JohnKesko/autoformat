//! Kotlin support for AutoFormat, based on the `tree-sitter-kt` grammar.
//!
//! Kotlin has no semicolons: a line break can end a statement. So this adapter is
//! careful about where line breaks may be added:
//! - Long lines are only wrapped inside `( )` and `[ ]`, after a comma. Kotlin ignores
//!   line breaks there.
//! - `brace_style` only moves the `{` of classes, functions, `if` / `else`, `when`,
//!   loops, `try` / `catch` / `finally`, `init`, constructors and `get` / `set`. Kotlin
//!   allows a line break before each of those.
//! - The `{` of a lambda is never moved. `run {` and `run` + line break + `{` do not
//!   mean the same. The `{` of `object : Type {` (an expression), of an anonymous
//!   function and of an enum value's body stay where they are too.
//!
//! Other special cases handled here:
//! - Strings (with their `$name` and `${...}` templates), raw strings (`"""`) and
//!   characters are never changed.
//! - `class A : B`, `<T : Any>` and `where T : Any` have a space on both sides of the
//!   colon. `name: Type` has a space after it only. `@get:Rule` has none.
//! - Generics have no spaces inside the angle brackets: `Map<String, List<Int>>`.
//! - `String?`, `value!!`, `return@loop`, `this@Outer` and `loop@` are kept together.
//! - Ranges (`1..10`, `0..<size`) keep the spacing they were written with.
//! - `get()` and `set(value)` are indented under their property.
//! - With `blank_lines` on, one empty line follows the `package` line and the imports.

use autoformat_core::parser::tree_sitter::Node;
use autoformat_core::{LanguageAdapter, LanguageRules, Role, Spacing, TokenContext};
use autoformat_parser::Grammar;

pub struct KotlinLanguage {
    grammar: Grammar,
}

impl KotlinLanguage {
    pub fn new() -> KotlinLanguage {
        KotlinLanguage {
            grammar: Grammar::new("kotlin", tree_sitter_kt::LANGUAGE.into()),
        }
    }
}

impl Default for KotlinLanguage {
    fn default() -> KotlinLanguage {
        KotlinLanguage::new()
    }
}

/// Nodes that hold the angle brackets of generics: `List<String>` and `class Box<T>`.
const ANGLE_BRACKET_KINDS: &[&str] = &["type_arguments", "type_parameters"];

/// Nodes where a `:` comes before a parent type or an upper bound and has a space on
/// both sides: `class A : B`, `object : B`, `constructor() : this()`, `<T : Any>`.
const SPACED_COLON_PARENTS: &[&str] = &[
    "class_declaration",
    "object_declaration",
    "object_literal",
    "companion_object",
    "secondary_constructor",
    "type_parameter",
    "type_constraint",
];

/// The two kinds of annotation: `@Test` and `@file:JvmName("Demo")`.
const ANNOTATION_KINDS: &[&str] = &["annotation", "file_annotation"];

/// Words directly followed by `(`: `constructor(name: String)`, `get()`, `this(1)`.
const WORDS_BEFORE_PAREN: &[&str] = &["constructor", "get", "set", "this", "super"];

static RULES: LanguageRules = LanguageRules {
    comment_kinds: &["line_comment", "multiline_comment"],
    // A string is kept whole, so its templates (`$name`, `${a.b}`) and the lines of a
    // raw string are printed exactly as written. `#!/usr/bin/env kotlin` is kept too.
    // Numbers with a letter at the end (`10L`, `0xFFu`) are kept whole as well.
    atomic_kinds: &[
        "string_literal",
        "character_literal",
        "long_literal",
        "unsigned_literal",
        "shebang_line",
    ],
    // `modifiers` holds the annotations and words like `private` in front of a
    // declaration. `statements` is the list of lines inside a block.
    // `value_arguments` is listed for annotations: the grammar hides the `(` of
    // `@Deprecated(`, so those brackets cannot indent their contents by themselves.
    transparent_kinds: &[
        "modifiers",
        "statements",
        "import_list",
        "catch_block",
        "finally_block",
        "value_arguments",
    ],
    // What is written in front of a declaration, often on a line of its own:
    // `@Test` and `context(scope: Scope)`.
    prefix_kinds: &["annotation", "context_parameters"],
    // Only lists inside `( )` and `[ ]`: Kotlin ignores line breaks inside those.
    wrappable_kinds: &[
        "value_arguments",
        "function_value_parameters",
        "primary_constructor",
        "function_type_parameters",
        "multi_variable_declaration",
        "collection_literal",
    ],
    outdent_tokens: &["{", "else", "catch", "finally"],
    space_before_paren_keywords: &["if", "for", "while", "when", "catch"],
    index_kinds: &["indexing_suffix"],
    extra_brackets: &[],
    separators: &[","],
    // Kotlin allows a line break before each of these braces. Lambdas, `object : Type {`,
    // anonymous functions and the bodies of enum values are left out on purpose.
    block_braces: &[
        ("class_body", "class_declaration"),
        ("enum_class_body", "class_declaration"),
        ("class_body", "object_declaration"),
        ("class_body", "companion_object"),
        ("function_body", "function_declaration"),
        ("function_body", "getter"),
        ("function_body", "setter"),
        ("secondary_constructor", "*"),
        ("anonymous_initializer", "*"),
        ("control_structure_body", "if_expression"),
        ("control_structure_body", "for_statement"),
        ("control_structure_body", "while_statement"),
        ("control_structure_body", "do_while_statement"),
        ("control_structure_body", "when_entry"),
        ("when_expression", "*"),
        ("try_expression", "*"),
        ("catch_block", "*"),
        ("finally_block", "*"),
    ],
    brace_continuations: &[
        ("else", "if_expression"),
        ("catch", "catch_block"),
        ("finally", "finally_block"),
        ("while", "do_while_statement"),
    ],
    declaration_containers: &["source_file", "class_body", "enum_class_body"],
    declaration_kinds: &[
        "class_declaration",
        "object_declaration",
        "companion_object",
        "function_declaration",
        "property_declaration",
        "secondary_constructor",
        "anonymous_initializer",
        "type_alias",
    ],
};

impl LanguageAdapter for KotlinLanguage {
    fn name(&self) -> &'static str {
        "kotlin"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["kt", "kts"]
    }

    fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    fn rules(&self) -> &LanguageRules {
        &RULES
    }

    fn always_separated(&self, before: Node, _after: Node) -> bool {
        // One empty line after the `package` line and after the imports.
        before.kind() == "package_header" || before.kind() == "import_list"
    }

    fn role(&self, node: Node, text: &str, parent_kind: &str) -> Option<Role> {
        // `fun f() = 1` and `get() = field`: the `=` sits between the function and its
        // value, like the `=` of `val x = 1`.
        if text == "=" && parent_kind == "function_body" {
            return Some(Role::Infix);
        }
        // The `?` of `String?` and the `!!` of `value!!` belong to what is before them.
        if node.kind() == "quest" || text == "!!" {
            return Some(Role::Postfix);
        }
        // A word such as `value`, `field` or `data` used as a name is a name.
        if parent_kind == "simple_identifier" {
            return Some(Role::Word);
        }
        None
    }

    fn joins_previous_sibling(&self, node: Node, parent_kind: &str) -> bool {
        if node.kind() == "getter" || node.kind() == "setter" {
            return true;
        }
        // Statements sit in a block (`statements`) or, in a script, in the file itself.
        let is_statement = parent_kind == "statements" || parent_kind == "source_file";
        is_statement && is_body_of_for_loop(node)
    }

    fn outdent(&self, token: &TokenContext) -> bool {
        // A lambda's `{` on a line of its own stays indented under its statement.
        if token.is("{") {
            return RULES.is_block_brace(token);
        }
        // The `)` that ends the arguments of an annotation written over several lines.
        // The grammar hides its `(`, so this `)` is not seen as a closing bracket.
        if token.is(")") && token.role != Role::Close {
            return true;
        }
        // `else`, `catch` and `finally` as keywords. A function may be named `catch`.
        if token.role == Role::Keyword && RULES.outdent_tokens.contains(&token.text) {
            return true;
        }
        starts_later_when_condition(token)
    }

    fn spacing(&self, previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
        // Labels: `loop@ for`, `run loop@{`, `return@loop`, `break@loop`, `this@Outer`,
        // `super@Outer`. A space here can change the meaning, so nothing is changed.
        if next.kind == "label" || previous.text.ends_with('@') {
            return Some(Spacing::Keep);
        }

        // The grammar hides the `(` of an annotation's arguments: `@Suppress("unused")`.
        if previous.role == Role::Gap && previous.is("(") {
            return Some(Spacing::NoSpace);
        }
        if next.is(")") && next.role != Role::Close {
            return Some(Spacing::NoSpace);
        }
        // A `;` between two statements on one line: `val a = 1; val b = 2`.
        if previous.role == Role::Gap && previous.is(";") {
            return Some(Spacing::Space);
        }

        // `String?`, `List<Int>?`, `(() -> Unit)?`: the `?` is part of the type.
        // `Never` is safe here: no Kotlin operator ends with the `?` of a type.
        if next.kind == "quest" {
            return Some(Spacing::Never);
        }

        let angle_spacing = angle_bracket_spacing(previous, next);
        if angle_spacing.is_some() {
            return angle_spacing;
        }

        // `!is` and `!in` are one word each. The space before them is kept as written.
        let next_is_test_word = next.is("is") || next.is("in");
        if previous.is("!") && next_is_test_word {
            return Some(Spacing::Never);
        }
        let not_test_parents = ["check_expression", "type_test", "range_test"];
        if next.is("!") && not_test_parents.contains(&next.parent_kind) {
            return Some(Spacing::Keep);
        }

        // Ranges keep the spacing they were written with: `1..10`, `0..<size`.
        let range_operators = ["..", "..<"];
        if range_operators.contains(&previous.text) || range_operators.contains(&next.text) {
            return Some(Spacing::Keep);
        }

        // `x -> x + 1`, `1 -> "one"`, `else -> 0`, `(Int) -> Unit`
        if next.is("->") || previous.is("->") {
            return Some(Spacing::Space);
        }

        // `@get:Rule`, `@file:JvmName("Demo")`
        let target_colon_parents = ["use_site_target", "file_annotation"];
        if previous.is(":") && target_colon_parents.contains(&previous.parent_kind) {
            return Some(Spacing::NoSpace);
        }

        // `class A : B`, `object : Runnable`, `constructor() : this()`, `<T : Any>`
        if next.is(":") && SPACED_COLON_PARENTS.contains(&next.parent_kind) {
            // `object : Runnable` is written with the space in every Kotlin style.
            return Some(Spacing::Space);
        }
        if previous.is(":") && SPACED_COLON_PARENTS.contains(&previous.parent_kind) {
            return Some(Spacing::Space);
        }

        if next.is("(") {
            // `@Volatile (a + b)` is not `@Volatile(a + b)`: after an annotation the
            // space before `(` is kept as written.
            let previous_in_annotation = previous.has_ancestor(ANNOTATION_KINDS);
            if previous_in_annotation && !next.has_ancestor(ANNOTATION_KINDS) {
                return Some(Spacing::Keep);
            }
            // `constructor(name: String)`, `get()`, `set(value)`, `this(name, 0)`
            if WORDS_BEFORE_PAREN.contains(&previous.text) && previous.role == Role::Keyword {
                return Some(Spacing::NoSpace);
            }
            // Brackets around a value are not a call.
            if next.parent_kind == "parenthesized_expression" {
                // `if (ready) (first as Runnable).run()`
                if previous.is(")") {
                    return Some(Spacing::Space);
                }
                // `flags or (1 shl bit)`
                if previous.role == Role::Word {
                    return Some(Spacing::Keep);
                }
            }
        }

        // `if (done) return`, `fun run() where T : Any`, `@Size(max = 5) val`
        if previous.is(")") && matches!(next.role, Role::Word | Role::Keyword) {
            return Some(Spacing::Space);
        }
        None
    }
}

/// Spacing around the `<` and `>` of generics. `None` for every other pair of tokens.
fn angle_bracket_spacing(previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
    let previous_is_angle = ANGLE_BRACKET_KINDS.contains(&previous.parent_kind);
    let next_is_angle = ANGLE_BRACKET_KINDS.contains(&next.parent_kind);

    // Nothing after `<` and nothing before `>`: `<String>`, `<*>`, `<List<T>>`.
    // `Never` is safe here: Kotlin has no `<>` or `>>` operator.
    if previous.is("<") && previous_is_angle {
        return Some(Spacing::Never);
    }
    if next.is(">") && next_is_angle {
        return Some(Spacing::Never);
    }

    if next.is("<") && next_is_angle {
        // `fun <T> first()`, `val <T> List<T>.last`: the brackets come before the name.
        if comes_before_name(next) {
            return Some(Spacing::Space);
        }
        // `List<String>`, `class Box<T>`, `listOf<String>()`
        return Some(Spacing::NoSpace);
    }

    if previous.is(">") && previous_is_angle {
        // `fun <T> first()`, `fun <T> (T).show()`
        if comes_before_name(previous) {
            return Some(Spacing::Space);
        }
        // `listOf<String>()`, `class Box<T>(val item: T)`
        if next.is("(") {
            return Some(Spacing::NoSpace);
        }
        // `List<String> names`, `class Box<T> where T : Any`, `Map<K, V> by lazy`
        if matches!(next.role, Role::Word | Role::Keyword) {
            return Some(Spacing::Space);
        }
    }
    None
}

/// Whether `node` is the body of the `for` loop written on the line before it:
///
/// ```text
/// for (item in items)
///     send(item)
/// ```
///
/// The grammar reads these as two statements side by side (the loop, then the call).
/// Kotlin reads the call as the loop's body, so it is indented under the loop.
fn is_body_of_for_loop(node: Node) -> bool {
    let mut current = node.prev_sibling();
    while let Some(sibling) = current {
        if !sibling.is_extra() {
            return sibling.kind() == "for_statement" && !has_body(sibling);
        }
        current = sibling.prev_sibling();
    }
    false
}

/// Whether a `for` loop node holds its own body.
fn has_body(for_loop: Node) -> bool {
    let mut cursor = for_loop.walk();
    let has_body = for_loop
        .children(&mut cursor)
        .any(|child| child.kind() == "control_structure_body");
    has_body
}

/// Whether the token starts the second or a later condition of one `when` branch:
///
/// ```text
/// when (color) {
///     RED,
///     GREEN -> "warm"     <- `GREEN` lines up with `RED`
/// }
/// ```
fn starts_later_when_condition(token: &TokenContext) -> bool {
    let token_node = match token.node {
        Some(node) => node,
        None => return false,
    };
    let start = token_node.start_byte();
    let mut current = Some(token_node);
    while let Some(node) = current {
        if node.start_byte() != start {
            return false;
        }
        if node.kind() == "when_condition" {
            return node.prev_sibling().is_some();
        }
        current = token.parents.parent(node);
    }
    false
}

/// Whether this `<` or `>` belongs to the generics written before the name of a
/// function or property: `fun <T> first()`.
fn comes_before_name(bracket: &TokenContext) -> bool {
    if bracket.parent_kind != "type_parameters" {
        return false;
    }
    let owner = bracket.grandparent_kind();
    owner == "function_declaration" || owner == "property_declaration"
}
