//! Builds a [`LayoutDocument`] from a syntax tree with the help of a language adapter.
//!
//! This is where the syntax tree is turned into layout facts: which tokens are brackets,
//! where statements start (for continuation indentation), and how tokens are spaced.

use autoformat_parser::tree_sitter::Node;
use autoformat_parser::{ParentMap, RawToken, SyntaxTree};

use crate::adapter::{LanguageAdapter, LanguageRules};
use crate::context::{
    is_conditional_colon, is_operator_text, is_word_text, operator_position, Role, TokenContext,
    TIGHT_TOKENS,
};
use crate::layout::{Group, LayoutDocument, LayoutToken, MultiLine, Spacing};
use crate::spacing::common_spacing;

/// Settings for building the layout.
#[derive(Clone, Debug)]
pub struct BuildOptions {
    /// Keep the original spacing everywhere (used as a safe fallback).
    pub keep_spacing: bool,
    /// Used to measure tab characters in the original source.
    pub indent_size: usize,
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
            index, raw, source, rules, is_open, is_close, &parents,
        ));
    }

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
            match adapter.spacing(previous, context) {
                Some(spacing) => spacing,
                None => common_spacing(previous, context, rules),
            }
        };
        token.opens_group = opens[index];
        token.closes_group = closes[index];
        token.column_zero = adapter.column_zero(context);
        token.outdent = adapter.outdent(context);
        token.original_line_indent = line_indent_width(source, raw.start, options.indent_size);

        if let Some(node) = raw.node {
            if !token.is_comment {
                let stop = enclosing[index].map(|group| pairs[group].list_node.id());
                token.anchors =
                    find_anchors(node, index, stop, root_id, rules, &token_starts, &parents);
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
            wrappable: adapter.is_wrappable(pair.list_node, open_text),
        });
    }

    LayoutDocument { tokens, groups }
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
    rules: &LanguageRules,
    opens: bool,
    closes: bool,
    parents: &'tree ParentMap<'tree>,
) -> TokenContext<'tree> {
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
        classify(node, text, parents)
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
fn find_anchors(
    node: Node,
    index: usize,
    stop: Option<usize>,
    root_id: usize,
    rules: &LanguageRules,
    token_starts: &[usize],
    parents: &ParentMap,
) -> Vec<usize> {
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
        current = parents.parent(ancestor);
    }
    anchors
}

/// Where a node really starts, ignoring leading attributes, decorators and comments.
fn effective_start(node: Node, rules: &LanguageRules) -> usize {
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if !child.is_extra() && !rules.prefix_kinds.contains(&child.kind()) {
                return child.start_byte();
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    node.start_byte()
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
