use tree_sitter::Node;

use crate::tree::SyntaxTree;

/// One token of the source document.
///
/// A token is normally a leaf of the syntax tree. Nodes that must be printed exactly as
/// written (strings, comments, ...) are "atomic": the whole node becomes one token, no
/// matter how many children it has.
///
/// If the grammar leaves some non-whitespace text outside of every node (for example a
/// PowerShell line-continuation backtick), that text becomes a token with `node == None`.
#[derive(Clone, Copy, Debug)]
pub struct RawToken<'tree> {
    pub node: Option<Node<'tree>>,
    pub start: usize,
    pub end: usize,
    /// Number of line breaks between the previous token and this one.
    pub line_breaks_before: usize,
    /// True when spaces or tabs separated this token from the previous one on the same line.
    pub space_before: bool,
}

impl SyntaxTree {
    /// Splits the document into tokens, in source order.
    ///
    /// `is_atomic` decides which nodes are kept whole instead of being split into children.
    pub fn tokens<'tree>(&'tree self, is_atomic: &dyn Fn(Node) -> bool) -> Vec<RawToken<'tree>> {
        let mut leaves: Vec<Node<'tree>> = Vec::new();
        collect_leaves(self.root(), is_atomic, &mut leaves);

        let source = self.source();
        let mut tokens = Vec::new();
        let mut position = 0;
        for leaf in leaves {
            let start = leaf.start_byte();
            let mut end = leaf.end_byte();
            let text = &source[start..end];
            if text.trim().is_empty() {
                // Whitespace that the grammar turned into a node (PowerShell does this
                // between command arguments). It is handled like any other whitespace.
                continue;
            }
            if leaf.is_extra() {
                // Some grammars include the line break at the end of a comment in the
                // comment node. Leave it outside of the token.
                end = start + text.trim_end().len();
            }
            if start < position {
                // Overlapping nodes should not happen; skip defensively.
                continue;
            }
            push_gap_tokens(source, position, start, &mut tokens);
            let (line_breaks_before, space_before) = describe_gap(source, gap_start(&tokens, position), start);
            tokens.push(RawToken {
                node: Some(leaf),
                start,
                end,
                line_breaks_before,
                space_before,
            });
            position = end;
        }
        push_gap_tokens(source, position, source.len(), &mut tokens);
        tokens
    }
}

/// Collects leaves (and atomic nodes) in source order. Zero-width nodes are skipped
/// because they have no text to print.
fn collect_leaves<'tree>(root: Node<'tree>, is_atomic: &dyn Fn(Node) -> bool, leaves: &mut Vec<Node<'tree>>) {
    let mut cursor = root.walk();
    loop {
        let node = cursor.node();
        let is_leaf = node.child_count() == 0 || is_atomic(node);
        if is_leaf {
            if node.end_byte() > node.start_byte() {
                leaves.push(node);
            }
        } else if cursor.goto_first_child() {
            continue;
        }
        // Move to the next sibling, climbing up as needed.
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return;
            }
        }
    }
}

/// Where the whitespace in front of the next token begins: right after the previous
/// token, which may be a gap token that was just pushed.
fn gap_start(tokens: &[RawToken], position: usize) -> usize {
    match tokens.last() {
        Some(token) if token.end > position => token.end,
        _ => position,
    }
}

/// Turns any non-whitespace text between `start` and `end` into gap tokens.
fn push_gap_tokens(source: &str, start: usize, end: usize, tokens: &mut Vec<RawToken>) {
    let gap = &source[start..end];
    let mut run_start: Option<usize> = None;
    for (offset, character) in gap.char_indices() {
        if character.is_whitespace() {
            if let Some(begin) = run_start {
                push_gap_token(source, start + begin, start + offset, tokens);
                run_start = None;
            }
        } else if run_start.is_none() {
            run_start = Some(offset);
        }
    }
    if let Some(begin) = run_start {
        push_gap_token(source, start + begin, end, tokens);
    }
}

fn push_gap_token(source: &str, start: usize, end: usize, tokens: &mut Vec<RawToken>) {
    let previous_end = match tokens.last() {
        Some(token) => token.end,
        None => 0,
    };
    let (line_breaks_before, space_before) = describe_gap(source, previous_end.min(start), start);
    tokens.push(RawToken {
        node: None,
        start,
        end,
        line_breaks_before,
        space_before,
    });
}

/// Counts line breaks in the whitespace between two tokens.
fn describe_gap(source: &str, start: usize, end: usize) -> (usize, bool) {
    if start >= end {
        return (0, false);
    }
    let gap = &source[start..end];
    let line_breaks = gap.matches('\n').count();
    let space = line_breaks == 0 && gap.contains([' ', '\t']);
    (line_breaks, space)
}
