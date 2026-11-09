use tree_sitter::Node;

use crate::tree::SyntaxTree;

/// Checks that two syntax trees describe the same program.
///
/// The trees must have the same shape (same node kinds in the same places). Whitespace
/// between nodes is allowed to differ, because changing it is the whole point of
/// formatting. Some grammars (PowerShell) turn whitespace into nodes; those nodes are
/// ignored. The text of every leaf must match according to `text_matches`.
///
/// - `is_atomic(node)`: compare the whole text of this node and do not look inside it.
/// - `text_matches(node, original_text, formatted_text)`: decides whether the text of a
///   leaf (or atomic node) is unchanged. The formatting engine uses this to allow
///   re-indented comments, for example.
///
/// Returns a description of the first difference when the trees differ.
pub fn compare_trees(
    original: &SyntaxTree,
    formatted: &SyntaxTree,
    is_atomic: &dyn Fn(Node) -> bool,
    text_matches: &dyn Fn(Node, &str, &str) -> bool,
) -> Result<(), String> {
    let checker = Checker {
        original,
        formatted,
        is_atomic,
        text_matches,
    };
    checker.compare(original.root(), formatted.root())
}

struct Checker<'a> {
    original: &'a SyntaxTree,
    formatted: &'a SyntaxTree,
    is_atomic: &'a dyn Fn(Node) -> bool,
    text_matches: &'a dyn Fn(Node, &str, &str) -> bool,
}

impl Checker<'_> {
    fn compare(&self, left: Node, right: Node) -> Result<(), String> {
        let line = left.start_position().row + 1;
        if left.kind_id() != right.kind_id() || left.is_named() != right.is_named() {
            return Err(format!(
                "expected `{}` but the formatted code has `{}` (line {})",
                left.kind(),
                right.kind(),
                line
            ));
        }

        let is_leaf = left.child_count() == 0 || (self.is_atomic)(left);
        if is_leaf {
            let left_text = self.original.text(left);
            let right_text = self.formatted.text(right);
            if (self.text_matches)(left, left_text, right_text) {
                return Ok(());
            }
            return Err(format!("the text of `{}` on line {} changed", left.kind(), line));
        }

        let left_children = children(self.original, left);
        let right_children = children(self.formatted, right);
        if left_children.len() != right_children.len() {
            return Err(format!("`{}` on line {} changed its structure", left.kind(), line));
        }
        for (left_child, right_child) in left_children.into_iter().zip(right_children) {
            self.compare(left_child, right_child)?;
        }
        Ok(())
    }
}

/// The children of a node, without nodes that contain only whitespace.
fn children<'tree>(tree: &'tree SyntaxTree, node: Node<'tree>) -> Vec<Node<'tree>> {
    let mut result = Vec::new();
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if !is_whitespace_node(tree, child) {
                result.push(child);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    result
}

/// A node made only of whitespace: a whitespace leaf, or a node whose children are all
/// whitespace nodes (PowerShell's argument separators, for example).
fn is_whitespace_node(tree: &SyntaxTree, node: Node) -> bool {
    let text = tree.text(node);
    if text.is_empty() {
        return false;
    }
    if node.child_count() == 0 {
        return text.trim().is_empty();
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if !is_whitespace_node(tree, cursor.node()) {
                return false;
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    true
}
