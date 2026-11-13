//! PowerShell support for AutoFormat, based on the `tree-sitter-powershell` grammar.
//!
//! PowerShell is sensitive to whitespace: `Get-Item -Path x` and `$a [0]` mean something
//! different without their spaces. So this adapter keeps the original spacing everywhere
//! except in a few places where changing it is known to be safe:
//! - after commas, around `=` in assignments, hashtables and parameter defaults,
//! - around the pipe `|`,
//! - between keywords and `(` or `{`, like `if (...) {`,
//! - inside braces and parentheses.
//!
//! Indentation is recomputed from the structure. Here-strings are never changed.

use autoformat_core::{LanguageAdapter, LanguageRules, Role, Spacing, TokenContext};
use autoformat_parser::tree_sitter::Node;
use autoformat_parser::Grammar;

pub struct PowerShellLanguage {
    grammar: Grammar,
}

impl PowerShellLanguage {
    pub fn new() -> PowerShellLanguage {
        PowerShellLanguage {
            grammar: Grammar::new("powershell", tree_sitter_powershell::LANGUAGE.into()),
        }
    }
}

impl Default for PowerShellLanguage {
    fn default() -> PowerShellLanguage {
        PowerShellLanguage::new()
    }
}

static RULES: LanguageRules = LanguageRules {
    comment_kinds: &["comment"],
    atomic_kinds: &["string_literal", "variable"],
    transparent_kinds: &[
        "statement_list",
        "script_block",
        "script_block_body",
        "named_block_list",
        "parameter_list",
        "argument_expression_list",
        "hash_literal_body",
        "switch_clauses",
        "elseif_clauses",
        "catch_clauses",
        "class_property_definition_list",
        "attribute_list",
        "elseif_clause",
        "else_clause",
        "catch_clause",
        "finally_clause",
    ],
    prefix_kinds: &["attribute_list"],
    // `param( ... )` blocks are left as written: one parameter per line is the usual style.
    wrappable_kinds: &["argument_list", "hash_literal_expression"],
    outdent_tokens: &["{", "else", "elseif", "catch", "finally"],
    space_before_paren_keywords: &["if", "elseif", "foreach", "for", "while", "switch", "until"],
    index_kinds: &[],
    extra_brackets: &[("@(", ")"), ("@{", "}"), ("$(", ")")],
    separators: &[",", ";"],
    // Script blocks passed to commands (`ForEach-Object { }`) and hashtables are not
    // listed: moving their braces would change or break the command.
    block_braces: &[
        ("function_statement", "*"),
        ("class_statement", "*"),
        ("statement_block", "if_statement"),
        ("statement_block", "elseif_clause"),
        ("statement_block", "else_clause"),
        ("statement_block", "foreach_statement"),
        ("statement_block", "for_statement"),
        ("statement_block", "while_statement"),
        ("statement_block", "do_statement"),
        ("statement_block", "try_statement"),
        ("statement_block", "catch_clause"),
        ("statement_block", "finally_clause"),
        ("statement_block", "trap_statement"),
        ("statement_block", "named_block"),
        ("switch_body", "switch_statement"),
    ],
    brace_continuations: &[
        ("else", "else_clause"),
        ("elseif", "elseif_clause"),
        ("catch", "catch_clause"),
        ("finally", "finally_clause"),
        ("while", "do_statement"),
        ("until", "do_statement"),
    ],
    declaration_containers: &["statement_list", "class_statement"],
    declaration_kinds: &[
        "function_statement",
        "class_statement",
        "enum_statement",
        "class_method_definition",
        "class_property_definition",
    ],
};

/// Keywords followed by a space before `{`.
const BLOCK_KEYWORDS: &[&str] = &[
    "else",
    "try",
    "finally",
    "do",
    "begin",
    "process",
    "end",
    "dynamicparam",
    "clean",
];

/// Parents of an `=` that is safe to surround with spaces.
const ASSIGNMENT_PARENTS: &[&str] = &[
    "assignement_operator",
    "hash_entry",
    "script_parameter_default",
];

/// Keywords that follow a closing `}` on the same line.
const AFTER_BRACE_KEYWORDS: &[&str] = &["else", "elseif", "catch", "finally", "while", "until"];

impl LanguageAdapter for PowerShellLanguage {
    fn name(&self) -> &'static str {
        "powershell"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ps1", "psm1", "psd1"]
    }

    fn grammar(&self) -> &Grammar {
        &self.grammar
    }

    fn rules(&self) -> &LanguageRules {
        &RULES
    }

    fn spacing(&self, previous: &TokenContext, next: &TokenContext) -> Option<Spacing> {
        Some(powershell_spacing(previous, next))
    }

    /// PowerShell keywords are case-insensitive: `Else` works like `else`.
    /// A script block given to a command, like `ForEach-Object { ... }`, can sit in the
    /// middle of a pipeline. The statement around it is the nearest pipeline.
    fn statement_around_block<'tree>(&self, open: &TokenContext<'tree>) -> Option<Node<'tree>> {
        if !open.is("{") || open.parent_kind != "script_block_expression" {
            return None;
        }
        let mut current = open.parent();
        while let Some(node) = current {
            if node.kind() == "pipeline" {
                return Some(node);
            }
            current = open.parents.parent(node);
        }
        None
    }

    /// The grammar reads `ForEach-Object{ }` as a command followed by a script block, and
    /// `ForEach-Object { }` as a command followed by an expression that holds only that
    /// script block. PowerShell runs both the same way, so the safety check looks through
    /// the two expression nodes when a script block is all they contain.
    fn is_plain_wrapper(&self, node: Node) -> bool {
        match node.kind() {
            "unary_expression" => holds_only_a_script_block(node),
            "array_literal_expression" => match only_child(node) {
                Some(child) => {
                    child.kind() == "unary_expression" && holds_only_a_script_block(child)
                }
                None => false,
            },
            _ => false,
        }
    }

    fn outdent(&self, token: &TokenContext) -> bool {
        RULES
            .outdent_tokens
            .contains(&token.text.to_lowercase().as_str())
    }

    fn is_brace_continuation(&self, token: &TokenContext) -> bool {
        let keyword = token.text.to_lowercase();
        for (wanted, parent_kind) in RULES.brace_continuations {
            if *wanted == keyword && *parent_kind == token.parent_kind {
                return true;
            }
        }
        false
    }
}

/// The only child of a node, or `None` when it has no children or more than one.
fn only_child(node: Node) -> Option<Node> {
    if node.child_count() != 1 {
        return None;
    }
    node.child(0)
}

fn holds_only_a_script_block(node: Node) -> bool {
    match only_child(node) {
        Some(child) => child.kind() == "script_block_expression",
        None => false,
    }
}

/// PowerShell spacing: keep the source spacing unless a rule below is known to be safe.
fn powershell_spacing(previous: &TokenContext, next: &TokenContext) -> Spacing {
    if matches!(previous.role, Role::Comment | Role::Gap)
        || matches!(next.role, Role::Comment | Role::Gap)
    {
        return Spacing::Keep;
    }

    // Commas: `@(1, 2, 3)`, `$x.Method(a, b)`.
    if next.is(",") {
        return Spacing::NoSpace;
    }
    if previous.is(",") {
        if next.role == Role::Close {
            return Spacing::NoSpace;
        }
        return Spacing::Space;
    }

    // `$a = 1`, `@{ Key = 'value' }`, `param($Days = 7)`
    if (next.is("=") && ASSIGNMENT_PARENTS.contains(&next.parent_kind))
        || (previous.is("=") && ASSIGNMENT_PARENTS.contains(&previous.parent_kind))
    {
        return Spacing::Space;
    }
    if next.parent_kind == "assignement_operator" || previous.parent_kind == "assignement_operator"
    {
        // `+=`, `-=` and friends.
        return Spacing::Space;
    }

    // `Get-Item | Select-Object`
    if (next.is("|") || previous.is("|")) && (next.kind == "|" || previous.kind == "|") {
        let pipe = if next.is("|") { next } else { previous };
        if pipe.parent_kind == "pipeline_chain" || pipe.parent_kind == "pipeline" {
            return Spacing::Space;
        }
    }

    // Brackets: `(a)`, `@(a)`, `$(a)`, `{ a }`, `@{ a }`, `{}`.
    if previous.role == Role::Open {
        if next.role == Role::Close {
            // `{ }` and `{}` keep their spacing; `()` and `@()` lose inner spaces.
            if next.is("}") {
                return Spacing::Keep;
            }
            return Spacing::NoSpace;
        }
        if previous.text.ends_with('{') {
            return Spacing::Space;
        }
        return Spacing::NoSpace;
    }
    if next.role == Role::Close {
        if next.is("}") {
            return Spacing::Space;
        }
        return Spacing::NoSpace;
    }

    // `if (`, `foreach (`, `switch (`
    if next.role == Role::Open
        && next.is("(")
        && previous.role == Role::Keyword
        && RULES
            .space_before_paren_keywords
            .contains(&previous.text.to_lowercase().as_str())
    {
        return Spacing::Space;
    }

    // `) {`, `else {`, `function Name {`, `try {`
    if next.role == Role::Open && next.is("{") {
        let after_keyword = previous.role == Role::Keyword
            && BLOCK_KEYWORDS.contains(&previous.text.to_lowercase().as_str());
        if previous.is(")") || after_keyword || previous.kind == "function_name" {
            return Spacing::Space;
        }
    }

    // `ForEach-Object {`, `Invoke-Command -ScriptBlock {`: a script block given to a
    // command. A parameter that ends with `:` keeps its value attached.
    if next.role == Role::Open && next.is("{") && next.parent_kind == "script_block_expression" {
        let after_parameter = previous.kind == "command_parameter" && !previous.text.ends_with(':');
        if previous.kind == "command_name" || after_parameter {
            return Spacing::Space;
        }
    }

    // `} else {`, `} catch {`
    if previous.is("}") && AFTER_BRACE_KEYWORDS.contains(&next.text.to_lowercase().as_str()) {
        return Spacing::Space;
    }

    // `[Math]::Round(1)`, `$x.Name`
    let is_member_operator =
        (next.is("::") || next.is(".")) && next.node.map(|node| !node.is_named()).unwrap_or(false);
    let after_value =
        previous.role == Role::Close || previous.kind == "variable" || previous.role == Role::Word;
    if is_member_operator && after_value {
        return Spacing::NoSpace;
    }

    Spacing::Keep
}
