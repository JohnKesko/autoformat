use std::collections::HashMap;

use tree_sitter::{Node, Tree};

/// A parsed source document: the source text and its concrete syntax tree.
pub struct SyntaxTree {
    source: String,
    tree: Tree,
}

/// Where the parser found the first syntax error. Lines and columns start at 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl std::fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}, column {}: {}", self.line, self.column, self.message)
    }
}

impl SyntaxTree {
    pub(crate) fn new(source: String, tree: Tree) -> SyntaxTree {
        SyntaxTree { source, tree }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn root(&self) -> Node<'_> {
        self.tree.root_node()
    }

    /// The exact source text covered by `node`.
    pub fn text(&self, node: Node) -> &str {
        &self.source[node.start_byte()..node.end_byte()]
    }

    /// Returns the first syntax error in the document, if there is one.
    pub fn first_error(&self) -> Option<SyntaxError> {
        let root = self.root();
        if !root.has_error() {
            return None;
        }
        let node = find_error_node(root).unwrap_or(root);
        let position = node.start_position();
        let message = if node.is_missing() {
            format!("missing `{}`", node.kind())
        } else {
            let text = self.text(node);
            let first_line = text.lines().next().unwrap_or("");
            let short: String = first_line.chars().take(40).collect();
            if short.is_empty() {
                "syntax error".to_string()
            } else {
                format!("syntax error near `{}`", short)
            }
        };
        Some(SyntaxError {
            line: position.row + 1,
            column: position.column + 1,
            message,
        })
    }

    /// Prints the tree as an indented outline. Used by `autoformat tree` to help when
    /// writing language rules.
    pub fn dump(&self) -> String {
        let mut output = String::new();
        self.dump_node(self.root(), 0, None, &mut output);
        output
    }

    fn dump_node(&self, node: Node, depth: usize, field: Option<&str>, output: &mut String) {
        let indent = "  ".repeat(depth);
        let field_text = match field {
            Some(name) => format!("{}: ", name),
            None => String::new(),
        };
        let position = node.start_position();
        let kind = if node.is_named() {
            node.kind().to_string()
        } else {
            format!("\"{}\"", node.kind())
        };
        output.push_str(&format!(
            "{}{}{} [{}:{}]",
            indent,
            field_text,
            kind,
            position.row + 1,
            position.column + 1
        ));
        if node.child_count() == 0 && node.is_named() {
            let text = self.text(node).replace('\n', "\\n");
            let short: String = text.chars().take(60).collect();
            output.push_str(&format!(" `{}`", short));
        }
        output.push('\n');
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                let child_field = cursor.field_name();
                self.dump_node(child, depth + 1, child_field, output);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
}

/// A table from each node to its parent.
///
/// Tree-sitter's own `Node::parent` searches down from the root every time, which is
/// slow when it is called for every token. This table is built once per document.
pub struct ParentMap<'tree> {
    parents: HashMap<usize, Node<'tree>>,
}

impl<'tree> ParentMap<'tree> {
    pub fn new(tree: &'tree SyntaxTree) -> ParentMap<'tree> {
        let mut parents = HashMap::new();
        let mut cursor = tree.root().walk();
        // The nodes from the root down to the current node.
        let mut path: Vec<Node<'tree>> = Vec::new();
        loop {
            let node = cursor.node();
            if let Some(&parent) = path.last() {
                parents.insert(node.id(), parent);
            }
            if cursor.goto_first_child() {
                path.push(node);
                continue;
            }
            // Move on to the next sibling, climbing up as needed.
            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return ParentMap { parents };
                }
                path.pop();
            }
        }
    }

    pub fn parent(&self, node: Node<'tree>) -> Option<Node<'tree>> {
        self.parents.get(&node.id()).copied()
    }
}

/// Finds the first ERROR or MISSING node in document order.
fn find_error_node(node: Node) -> Option<Node> {
    if node.is_error() || node.is_missing() {
        return Some(node);
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.has_error() || child.is_missing() {
                if let Some(found) = find_error_node(child) {
                    return Some(found);
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    None
}
