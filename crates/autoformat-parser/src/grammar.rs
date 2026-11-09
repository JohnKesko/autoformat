use crate::tree::SyntaxTree;

/// A Tree-sitter grammar together with a human readable name.
///
/// Each language adapter creates one `Grammar` from its grammar crate, for example
/// `Grammar::new("rust", tree_sitter_rust::LANGUAGE.into())`.
pub struct Grammar {
    name: &'static str,
    language: tree_sitter::Language,
}

impl Grammar {
    pub fn new(name: &'static str, language: tree_sitter::Language) -> Grammar {
        Grammar { name, language }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn language(&self) -> &tree_sitter::Language {
        &self.language
    }

    /// Parses `source` into a syntax tree.
    ///
    /// A tree is returned even when the source contains syntax errors. Callers must check
    /// [`SyntaxTree::first_error`] before trusting the tree.
    pub fn parse(&self, source: &str) -> Result<SyntaxTree, String> {
        let mut parser = tree_sitter::Parser::new();
        if let Err(error) = parser.set_language(&self.language) {
            return Err(format!(
                "the {} grammar is not compatible with this Tree-sitter version: {}",
                self.name, error
            ));
        }
        match parser.parse(source, None) {
            Some(tree) => Ok(SyntaxTree::new(source.to_string(), tree)),
            None => Err(format!("the {} parser did not produce a syntax tree", self.name)),
        }
    }
}
