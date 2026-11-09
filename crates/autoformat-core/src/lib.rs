//! The AutoFormat engine.
//!
//! The engine is language independent. Language adapters (in `autoformat-languages`)
//! describe their syntax through [`LanguageAdapter`] and [`LanguageRules`]; the engine
//! does everything else:
//!
//! 1. parse the source with Tree-sitter ([`autoformat_parser`]),
//! 2. turn the syntax tree into a [`LayoutDocument`] ([`build_layout`]),
//! 3. print the layout: indentation, spacing, wrapping, comment alignment ([`print`]),
//! 4. parse the result again and prove it is the same program ([`format_source`]).

// The printer walks tokens by index on purpose: it often looks at neighbours and line
// boundaries, and index loops keep that easy to follow.
#![allow(clippy::needless_range_loop)]

mod adapter;
mod builder;
mod context;
mod engine;
mod layout;
mod printer;
mod spacing;
pub mod testing;

pub use adapter::{LanguageAdapter, LanguageRules};
pub use builder::{build_layout, BuildOptions};
pub use context::{Role, TokenContext};
pub use engine::{format_source, FormatError, Formatted};
pub use layout::{Group, LayoutDocument, LayoutToken, MultiLine, Spacing};
pub use printer::{print, PrintOptions};
pub use spacing::common_spacing;

pub use autoformat_config as config;
pub use autoformat_parser as parser;
