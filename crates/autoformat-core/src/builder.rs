//! Builds a [`LayoutDocument`] from a syntax tree with the help of a language adapter.
//!
//! This is where the syntax tree is turned into layout facts: which tokens are brackets,
//! where statements start (for continuation indentation), and how tokens are spaced.

use std::collections::HashMap;

use autoformat_parser::tree_sitter::Node;
use autoformat_parser::{ParentMap, RawToken, SyntaxTree};

use crate::adapter::{LanguageAdapter, LanguageRules};
use crate::context::{
    is_conditional_colon, is_operator_text, is_word_text, operator_position, Role, TokenContext,
    TIGHT_TOKENS,
};
use crate::layout::{DeclarationGap, Group, LayoutDocument, LayoutToken, MultiLine, Spacing};
use crate::spacing::common_spacing;

/// Settings for building the layout.
#[derive(Clone, Debug)]
pub struct BuildOptions {
    /// Keep the original spacing everywhere (used as a safe fallback).
    pub keep_spacing: bool,
    /// Used to measure tab characters in the original source.
    pub indent_size: usize,
    /// When false, operators between two values get no spaces around them.
    pub space_around_operators: bool,
}

/// A pair of matching brackets found in the token list.
struct BracketPair<'tree> {
    open: usize,
    close: usize,
    /// The node that directly contains both brackets.
    list_node: Node<'tree>,
}

pub fn build_layout(
    tree: &SyntaxTree,
    adapter: &dyn LanguageAdapter,
    options: &BuildOptions,
) -> LayoutDocument {
    let rules = adapter.rules();
    let source = tree.source();
    let is_atomic = |node: Node| rules.is_atomic(node);
    let raw_tokens = tree.tokens(&is_atomic);
    let count = raw_tokens.len();
    let token_starts: Vec<usize> = raw_tokens.iter().map(|token| token.start).collect();
    let root_id = tree.root().id();
    let parents = ParentMap::new(tree);

    let pairs = pair_brackets(source, &raw_tokens, rules, &parents);
    let mut opens: Vec<Option<usize>> = vec![None; count];
    let mut closes: Vec<Option<usize>> = vec![None; count];
    for (group_index, pair) in pairs.iter().enumerate() {
        opens[pair.open] = Some(group_index);
        closes[pair.close] = Some(group_index);
    }
    let enclosing = enclosing_groups(&opens, &closes);

    let mut contexts: Vec<TokenContext> = Vec::with_capacity(count);
    for (index, raw) in raw_tokens.iter().enumerate() {
        let is_open = opens[index].is_some();
        let is_close = closes[index].is_some();
        contexts.push(make_context(
            index, raw, source, adapter, is_open, is_close, &parents,
        ));
    }

    // Filled in by `find_anchors`, so that each syntax node is only looked at once.
    let mut owner_anchors: HashMap<usize, Option<usize>> = HashMap::new();
    let mut tokens: Vec<LayoutToken> = Vec::with_capacity(count);
    for index in 0..count {
        let context = &contexts[index];
        let raw = &raw_tokens[index];
        let mut token = LayoutToken::new(context.text);
        token.is_comment = context.role == Role::Comment;
        token.line_breaks_before = raw.line_breaks_before;
        // A line break counts as whitespace: if brace placement joins this token to the
        // previous line, "keep the spacing" then means one space.
        token.had_space_before = raw.space_before || raw.line_breaks_before > 0;
        token.spacing = if index == 0 || options.keep_spacing {
            Spacing::Keep
        } else {
            let previous = &contexts[index - 1];
            let spacing = match adapter.spacing(previous, context) {
                Some(spacing) => spacing,
                None => common_spacing(previous, context, rules),
            };
            if options.space_around_operators {
                spacing
            } else {
                spacing_without_operator_spaces(spacing, &contexts, index)
            }
        };
        token.trailing_whitespace = trailing_whitespace(source, &raw_tokens, index);
        token.describes_next = token.is_comment && !context.kind.starts_with("preproc");
        token.opens_group = opens[index];
        token.closes_group = closes[index];
        token.column_zero = adapter.column_zero(context);
        token.outdent = adapter.outdent(context);
        token.original_line_indent = line_indent_width(source, raw.start, options.indent_size);

        if let Some(node) = raw.node {
            if !token.is_comment {
                let stop = enclosing[index].map(|group| pairs[group].list_node.id());
                token.anchors = find_anchors(
                    node,
                    index,
                    stop,
                    root_id,
                    adapter,
                    &token_starts,
                    &parents,
                    &mut owner_anchors,
                );
            }
            if token.is_multi_line() {
                if token.is_comment {
                    token.multi_line = if is_star_comment(&token.text) {
                        MultiLine::StarAligned
                    } else {
                        MultiLine::Shift
                    };
                } else if let Some(protected_lines) = adapter.shift_protected_lines(tree, node) {
                    token.multi_line = MultiLine::Shift;
                    token.protected_lines = protected_lines;
                }
            }
        }
        tokens.push(token);
    }

    // Blocks in the middle of a statement that goes on after the block's closing brace.
    for pair in &pairs {
        let statement = match adapter.statement_around_block(&contexts[pair.open]) {
            Some(statement) => statement,
            None => continue,
        };
        // The statement goes on when the next piece of code after `}` is still part of it.
        let mut next = pair.close + 1;
        while next < count && tokens[next].is_comment {
            next += 1;
        }
        if next >= count || raw_tokens[next].start >= statement.end_byte() {
            continue;
        }
        let start = statement.start_byte();
        let first = token_starts.partition_point(|&token_start| token_start < start);
        // Only when no other bracket sits between the statement's start and the block.
        if first < pair.open && enclosing[first] == enclosing[pair.open] {
            tokens[pair.open].statement_continues_from = Some(first);
        }
    }

    // Code blocks whose braces follow `brace_style`, and the `else` / `catch` keywords
    // that follow their closing brace.
    for pair in &pairs {
        if adapter.is_block_brace(&contexts[pair.open]) {
            tokens[pair.open].block_brace = true;
        }
    }
    for index in 1..count {
        let follows_block = match tokens[index - 1].closes_group {
            Some(group) => tokens[pairs[group].open].block_brace,
            None => false,
        };
        if follows_block && adapter.is_brace_continuation(&contexts[index]) {
            tokens[index].brace_continuation = true;
            // On its own line it lines up with the `}` before it.
            tokens[index].outdent = true;
        }
    }

    let mut groups = Vec::with_capacity(pairs.len());
    for (group_index, pair) in pairs.iter().enumerate() {
        let open_text = contexts[pair.open].text;
        groups.push(Group {
            open: pair.open,
            close: pair.close,
            separators: find_separators(pair, group_index, &contexts, &enclosing, rules, &parents),
            wrappable: is_wrappable_group(adapter, pair.list_node, open_text),
        });
    }

    let declaration_gaps = find_declaration_gaps(tree, adapter, &token_starts);
    LayoutDocument {
        tokens,
        groups,
        declaration_gaps,
    }
}

/// Whether a bracket group follows `column_width`: lists in `( )` and `[ ]` that the
/// language allows, and arrays in `{ }`. Nothing else is ever wrapped or joined.
fn is_wrappable_group(adapter: &dyn LanguageAdapter, list_node: Node, open_text: &str) -> bool {
    match open_text {
        "(" | "[" => adapter.is_wrappable(list_node, open_text),
        "{" => adapter.is_array_in_braces(list_node),
        _ => false,
    }
}

/// With `space_around_operators = false`: no space on either side of an operator such
/// as `=`, `+` or `&&` where `true` would put one. An operator that would run together
/// with a neighbour (`a - -b` would become `a--b`) keeps the spaces on both sides.
///
/// The result never depends on how the code was spaced before, so switching the setting
/// back and forth always gives the same code for the same setting.
fn spacing_without_operator_spaces(
    spacing: Spacing,
    contexts: &[TokenContext],
    index: usize,
) -> Spacing {
    if spacing != Spacing::Space {
        return spacing;
    }
    // The space after a comma or semicolon is not an operator space: `Map<?, ?>`.
    if matches!(contexts[index - 1].role, Role::Comma | Role::Semicolon) {
        return spacing;
    }
    let previous_is_operator = is_spaced_operator(&contexts[index - 1]);
    let next_is_operator = is_spaced_operator(&contexts[index]);
    if !previous_is_operator && !next_is_operator {
        return spacing;
    }
    if previous_is_operator && operator_needs_spaces(contexts, index - 1) {
        return Spacing::Space;
    }
    if next_is_operator && operator_needs_spaces(contexts, index) {
        return Spacing::Space;
    }
    Spacing::Never
}

/// Operators between two values: `=`, `+=`, `+`, `==`, `&&`, `|`, `? :` and so on.
/// Not operators that belong to one value (`-x`, `!done`, `i++`), not the arrows
/// `->` and `=>`, and not the `...` of Java's `String... names`.
fn is_spaced_operator(context: &TokenContext) -> bool {
    if context.text == "->" || context.text == "=>" || context.text == "..." {
        return false;
    }
    match context.role {
        Role::Infix => true,
        Role::Other => match context.node {
            Some(node) => !node.is_named() && is_operator_text(context.text),
            None => false,
        },
        _ => false,
    }
}

/// Whether the operator would run together with the token before or after it.
fn operator_needs_spaces(contexts: &[TokenContext], operator: usize) -> bool {
    let text = contexts[operator].text;
    if operator > 0 && could_run_together(contexts[operator - 1].text, text) {
        return true;
    }
    if operator + 1 < contexts.len() && could_run_together(text, contexts[operator + 1].text) {
        return true;
    }
    false
}

/// Two characters that, written next to each other, start a longer operator or a
/// comment. `a - -b` must not become `a--b`, `Vec<u8> = x` must not become `Vec<u8>= x`,
/// and `a / /re/` must not start a `//` comment.
const RUN_TOGETHER_PAIRS: &[&str] = &[
    "++", "--", "**", "//", "/*", "*/", "==", "!=", "<=", ">=", "=>", "->", "<-", "&&", "||", "??",
    "?.", "?[", "::", "..", "<<", ">>", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "!!", "|>",
    "<>", "?:",
];

/// Whether `left` followed directly by `right` could be read as a different token.
fn could_run_together(left: &str, right: &str) -> bool {
    let last = match left.chars().last() {
        Some(character) => character,
        None => return false,
    };
    let first = match right.chars().next() {
        Some(character) => character,
        None => return false,
    };
    let pair = format!("{}{}", last, first);
    RUN_TOGETHER_PAIRS.contains(&pair.as_str())
}

/// The spaces and tabs between a token and the end of its line in the source, if a line
/// break or the end of the file follows the token.
fn trailing_whitespace(source: &str, tokens: &[RawToken], index: usize) -> String {
    let end = tokens[index].end;
    let next_start = match tokens.get(index + 1) {
        Some(next) => next.start,
        None => source.len(),
    };
    let gap = &source[end..next_start];
    match gap.find('\n') {
        Some(newline) => gap[..newline].trim_end_matches('\r').to_string(),
        // After the last token of a file that does not end with a line break, the rest
        // of the file is the end of its line.
        None if index + 1 == tokens.len() => gap.trim_end_matches('\r').to_string(),
        None => String::new(),
    }
}

/// Finds neighbouring declarations inside the language's declaration containers.
fn find_declaration_gaps(
    tree: &SyntaxTree,
    adapter: &dyn LanguageAdapter,
    token_starts: &[usize],
) -> Vec<DeclarationGap> {
    let rules = adapter.rules();
    let mut gaps = Vec::new();
    if rules.declaration_kinds.is_empty() {
        return gaps;
    }
    let mut stack = vec![tree.root()];
    while let Some(node) = stack.pop() {
        let mut cursor = node.walk();
        let mut children = Vec::new();
        if cursor.goto_first_child() {
            loop {
                children.push(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        if rules.declaration_containers.contains(&node.kind()) {
            add_gaps_between_children(node.kind(), &children, adapter, token_starts, &mut gaps);
        }
        for child in children {
            if child.child_count() > 0 && !rules.is_atomic(child) {
                stack.push(child);
            }
        }
    }
    gaps.sort_by_key(|gap| gap.first_after);
    gaps
}

fn add_gaps_between_children(
    container_kind: &str,
    children: &[Node],
    adapter: &dyn LanguageAdapter,
    token_starts: &[usize],
    gaps: &mut Vec<DeclarationGap>,
) {
    let rules = adapter.rules();
    // (node, first token, last token) of each child. Attributes such as Rust's
    // `#[test]` belong to the declaration after them.
    let mut items: Vec<(Node, usize, usize)> = Vec::new();
    let mut pending_attribute_start: Option<usize> = None;
    for child in children {
        if !child.is_named() || child.is_extra() || child.end_byte() <= child.start_byte() {
            continue;
        }
        let first = token_starts.partition_point(|&start| start < child.start_byte());
        let after_last = token_starts.partition_point(|&start| start < child.end_byte());
        if after_last == 0 || first >= after_last {
            continue;
        }
        let last = after_last - 1;
        // Kotlin's `get()` / `set(value)` are part of the property before them.
        if adapter.joins_previous_sibling(*child, container_kind) {
            if let Some(previous_item) = items.last_mut() {
                previous_item.2 = last;
                continue;
            }
        }
        if rules.prefix_kinds.contains(&child.kind()) {
            if pending_attribute_start.is_none() {
                pending_attribute_start = Some(first);
            }
            continue;
        }
        let first = pending_attribute_start.take().unwrap_or(first);
        items.push((*child, first, last));
    }
    for pair in items.windows(2) {
        let (before, first_before, last_before) = pair[0];
        let (after, first_after, last_after) = pair[1];
        let is_declaration = rules.declaration_kinds.contains(&before.kind())
            || rules.declaration_kinds.contains(&after.kind());
        let always = adapter.always_separated(before, after);
        if is_declaration || always {
            gaps.push(DeclarationGap {
                first_before,
                last_before,
                first_after,
                last_after,
                always,
            });
        }
    }
}

/// Finds matching bracket tokens. Two brackets match when they are children of the same
/// syntax node, so brackets inside strings or comments are never involved.
fn pair_brackets<'tree>(
    source: &str,
    tokens: &[RawToken<'tree>],
    rules: &LanguageRules,
    parents: &ParentMap<'tree>,
) -> Vec<BracketPair<'tree>> {
    struct OpenBracket<'tree> {
        index: usize,
        closer: &'static str,
        parent: Node<'tree>,
    }
    let mut stack: Vec<OpenBracket<'tree>> = Vec::new();
    let mut pairs = Vec::new();

    for (index, token) in tokens.iter().enumerate() {
        let node = match token.node {
            Some(node) if !node.is_named() => node,
            _ => continue,
        };
        let parent = match parents.parent(node) {
            Some(parent) => parent,
            None => continue,
        };
        let text = &source[token.start..token.end];
        if let Some(closer) = closer_for(text, rules) {
            stack.push(OpenBracket {
                index,
                closer,
                parent,
            });
            continue;
        }
        if !is_closer(text, rules) {
            continue;
        }
        let mut found = None;
        for position in (0..stack.len()).rev() {
            if stack[position].closer == text && stack[position].parent.id() == parent.id() {
                found = Some(position);
                break;
            }
        }
        if let Some(position) = found {
            let open = stack[position].index;
            stack.truncate(position);
            pairs.push(BracketPair {
                open,
                close: index,
                list_node: parent,
            });
        }
    }
    pairs.sort_by_key(|pair| pair.open);
    pairs
}

fn closer_for(text: &str, rules: &LanguageRules) -> Option<&'static str> {
    match text {
        "(" => return Some(")"),
        "[" => return Some("]"),
        "{" => return Some("}"),
        _ => {}
    }
    for (open, close) in rules.extra_brackets {
        if *open == text {
            return Some(close);
        }
    }
    None
}

fn is_closer(text: &str, rules: &LanguageRules) -> bool {
    if text == ")" || text == "]" || text == "}" {
        return true;
    }
    rules.extra_brackets.iter().any(|(_, close)| *close == text)
}

/// For every token, the innermost group that contains it (between its brackets).
fn enclosing_groups(opens: &[Option<usize>], closes: &[Option<usize>]) -> Vec<Option<usize>> {
    let mut stack: Vec<usize> = Vec::new();
    let mut result = Vec::with_capacity(opens.len());
    for index in 0..opens.len() {
        if closes[index].is_some() {
            stack.pop();
        }
        result.push(stack.last().copied());
        if let Some(group) = opens[index] {
            stack.push(group);
        }
    }
    result
}

fn make_context<'tree>(
    index: usize,
    raw: &RawToken<'tree>,
    source: &'tree str,
    adapter: &dyn LanguageAdapter,
    opens: bool,
    closes: bool,
    parents: &'tree ParentMap<'tree>,
) -> TokenContext<'tree> {
    let rules = adapter.rules();
    let text = &source[raw.start..raw.end];
    let node = match raw.node {
        Some(node) => node,
        None => {
            return TokenContext {
                index,
                text,
                node: None,
                kind: "",
                parent_kind: "",
                role: Role::Gap,
                had_space_before: raw.space_before,
                parents,
            }
        }
    };
    let parent_kind = match parents.parent(node) {
        Some(parent) => parent.kind(),
        None => "",
    };
    let role = if rules.is_comment(node) {
        Role::Comment
    } else if opens {
        Role::Open
    } else if closes {
        Role::Close
    } else {
        match adapter.role(node, text, parent_kind) {
            Some(role) => role,
            None => classify(node, text, parents),
        }
    };
    TokenContext {
        index,
        text,
        node: Some(node),
        kind: node.kind(),
        parent_kind,
        role,
        had_space_before: raw.space_before,
        parents,
    }
}

fn classify(node: Node, text: &str, parents: &ParentMap) -> Role {
    if text == "," {
        return Role::Comma;
    }
    if text == ";" {
        return Role::Semicolon;
    }
    if text == "_" {
        // The wildcard pattern is an operand, even though the grammar leaves it unnamed.
        return Role::Word;
    }
    if TIGHT_TOKENS.contains(&text) {
        return Role::Tight;
    }
    if node.is_named() {
        if is_word_text(text) {
            return Role::Word;
        }
        return Role::Other;
    }
    if text == ":" {
        if is_conditional_colon(node, parents) {
            return Role::Infix;
        }
        return Role::Colon;
    }
    if is_operator_text(text) {
        return operator_position(node, parents);
    }
    if is_word_text(text) {
        return Role::Keyword;
    }
    Role::Other
}

/// The tokens where the syntax nodes around `node` start, from the innermost node out,
/// stopping at the node that holds the innermost enclosing brackets.
#[allow(clippy::too_many_arguments)]
fn find_anchors(
    node: Node,
    index: usize,
    stop: Option<usize>,
    root_id: usize,
    adapter: &dyn LanguageAdapter,
    token_starts: &[usize],
    parents: &ParentMap,
    owner_anchors: &mut HashMap<usize, Option<usize>>,
) -> Vec<usize> {
    let rules = adapter.rules();
    let mut anchors: Vec<usize> = Vec::new();
    let mut current = parents.parent(node);
    while let Some(ancestor) = current {
        if Some(ancestor.id()) == stop || ancestor.id() == root_id {
            break;
        }
        if !rules.transparent_kinds.contains(&ancestor.kind()) {
            let start = effective_start(ancestor, rules);
            let anchor = token_starts.partition_point(|&token_start| token_start < start);
            if anchor < index && anchors.last() != Some(&anchor) {
                anchors.push(anchor);
            }
        }
        // Kotlin's `get()` / `set(value)` are indented under the property before them.
        let owner_anchor = match owner_anchors.get(&ancestor.id()) {
            Some(known) => *known,
            None => {
                let found = find_owner_anchor(ancestor, adapter, token_starts, parents);
                owner_anchors.insert(ancestor.id(), found);
                found
            }
        };
        if let Some(anchor) = owner_anchor {
            if anchor < index && anchors.last() != Some(&anchor) {
                anchors.push(anchor);
            }
        }
        current = parents.parent(ancestor);
    }
    anchors
}

/// For a node that belongs to the node written before it (see
/// [`LanguageAdapter::joins_previous_sibling`]): the token where that earlier node starts.
fn find_owner_anchor(
    node: Node,
    adapter: &dyn LanguageAdapter,
    token_starts: &[usize],
    parents: &ParentMap,
) -> Option<usize> {
    let parent_kind = match parents.parent(node) {
        Some(parent) => parent.kind(),
        None => "",
    };
    if !adapter.joins_previous_sibling(node, parent_kind) {
        return None;
    }
    let owner = previous_code_sibling(node)?;
    let start = effective_start(owner, adapter.rules());
    Some(token_starts.partition_point(|&token_start| token_start < start))
}

/// The named node written before `node` under the same parent, skipping comments.
fn previous_code_sibling(node: Node) -> Option<Node> {
    let mut current = node.prev_sibling();
    while let Some(sibling) = current {
        if sibling.is_named() && !sibling.is_extra() {
            return Some(sibling);
        }
        current = sibling.prev_sibling();
    }
    None
}

/// Where a node really starts, ignoring leading attributes, decorators and comments.
fn effective_start(node: Node, rules: &LanguageRules) -> usize {
    match start_after_prefixes(node, rules) {
        Some(start) => start,
        None => node.start_byte(),
    }
}

/// The start of the first child that is not an attribute, decorator or comment.
/// `None` when the node has no such child.
///
/// A transparent child is searched the same way. Java keeps annotations and words like
/// `public` together in one `modifiers` node: `@Override public void run()` really
/// starts at `public`, and `@Override void run()` really starts at `void`.
fn start_after_prefixes(node: Node, rules: &LanguageRules) -> Option<usize> {
    let mut cursor = node.walk();
    if !cursor.goto_first_child() {
        return None;
    }
    loop {
        let child = cursor.node();
        let is_prefix = child.is_extra() || rules.prefix_kinds.contains(&child.kind());
        if !is_prefix {
            let look_inside =
                rules.transparent_kinds.contains(&child.kind()) && child.child_count() > 0;
            if !look_inside {
                return Some(child.start_byte());
            }
            if let Some(start) = start_after_prefixes(child, rules) {
                return Some(start);
            }
        }
        if !cursor.goto_next_sibling() {
            return None;
        }
    }
}

/// Separators directly inside a group (not inside nested brackets).
fn find_separators(
    pair: &BracketPair,
    group_index: usize,
    contexts: &[TokenContext],
    enclosing: &[Option<usize>],
    rules: &LanguageRules,
    parents: &ParentMap,
) -> Vec<usize> {
    let mut separators = Vec::new();
    for index in pair.open + 1..pair.close {
        if enclosing[index] != Some(group_index)
            || !rules.separators.contains(&contexts[index].text)
        {
            continue;
        }
        let parent = match contexts[index].node.and_then(|node| parents.parent(node)) {
            Some(parent) => parent,
            None => continue,
        };
        let direct_child = parent.id() == pair.list_node.id();
        let child_of_transparent_list = rules.transparent_kinds.contains(&parent.kind())
            && parents.parent(parent).map(|grandparent| grandparent.id())
                == Some(pair.list_node.id());
        if direct_child || child_of_transparent_list {
            separators.push(index);
        }
    }
    separators
}

/// The width of the indentation of the source line that contains `position`.
fn line_indent_width(source: &str, position: usize, indent_size: usize) -> usize {
    let line_start = match source[..position].rfind('\n') {
        Some(newline) => newline + 1,
        None => 0,
    };
    let mut width = 0;
    for character in source[line_start..].chars() {
        match character {
            ' ' => width += 1,
            '\t' => width += indent_size,
            _ => break,
        }
    }
    width
}

/// A `/* ... */` comment whose inner lines all start with `*`.
fn is_star_comment(text: &str) -> bool {
    if !text.starts_with("/*") {
        return false;
    }
    text.split('\n').skip(1).all(|line| {
        let content = line.trim();
        content.is_empty() || content.starts_with('*')
    })
}
