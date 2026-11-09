//! Tree-sitter integration for AutoFormat.
//!
//! This crate knows how to:
//! - wrap a Tree-sitter grammar ([`Grammar`]),
//! - parse source text into a [`SyntaxTree`],
//! - report syntax errors safely ([`SyntaxError`]),
//! - split a tree into the tokens the formatter works with ([`SyntaxTree::tokens`]),
//! - compare two trees to prove that formatting changed only whitespace ([`compare_trees`]).
//!
//! It contains no formatting rules and no knowledge of any specific language.

mod compare;
mod grammar;
mod tokens;
mod tree;

pub use compare::compare_trees;
pub use grammar::Grammar;
pub use tokens::RawToken;
pub use tree::{ParentMap, SyntaxError, SyntaxTree};

/// Re-exported so language crates use exactly the same Tree-sitter version.
pub use tree_sitter;
