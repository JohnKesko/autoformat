//! The list of languages AutoFormat supports, and file type detection.

use std::path::Path;

use autoformat_core::LanguageAdapter;

/// All language adapters, created once and looked up by name or file extension.
pub struct Languages {
    adapters: Vec<Box<dyn LanguageAdapter>>,
}

impl Languages {
    pub fn new() -> Languages {
        let adapters: Vec<Box<dyn LanguageAdapter>> = vec![
            Box::new(autoformat_lang_rust::RustLanguage::new()),
            Box::new(autoformat_lang_toml::TomlLanguage::new()),
            Box::new(autoformat_lang_javascript::JavaScriptLanguage::new()),
            Box::new(autoformat_lang_typescript::TypeScriptLanguage::new()),
            Box::new(autoformat_lang_typescript::TsxLanguage::new()),
            Box::new(autoformat_lang_csharp::CSharpLanguage::new()),
            Box::new(autoformat_lang_powershell::PowerShellLanguage::new()),
        ];
        Languages { adapters }
    }

    pub fn all(&self) -> &[Box<dyn LanguageAdapter>] {
        &self.adapters
    }

    /// Finds a language by its name, for example `"rust"`.
    pub fn by_name(&self, name: &str) -> Option<&dyn LanguageAdapter> {
        let wanted = name.to_lowercase();
        for adapter in &self.adapters {
            if adapter.name() == wanted {
                return Some(adapter.as_ref());
            }
        }
        None
    }

    /// Finds a language by file extension (without the dot).
    pub fn by_extension(&self, extension: &str) -> Option<&dyn LanguageAdapter> {
        let wanted = extension.to_lowercase();
        for adapter in &self.adapters {
            if adapter.extensions().contains(&wanted.as_str()) {
                return Some(adapter.as_ref());
            }
        }
        None
    }

    /// Finds the language for a file path, based on its extension.
    pub fn for_path(&self, path: &Path) -> Option<&dyn LanguageAdapter> {
        let extension = path.extension()?.to_str()?;
        self.by_extension(extension)
    }
}

impl Default for Languages {
    fn default() -> Languages {
        Languages::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_languages_from_paths() {
        let languages = Languages::new();
        assert_eq!(languages.for_path(Path::new("src/main.rs")).unwrap().name(), "rust");
        assert_eq!(languages.for_path(Path::new("Cargo.toml")).unwrap().name(), "toml");
        assert_eq!(languages.for_path(Path::new("MAIN.RS")).unwrap().name(), "rust");
        assert!(languages.for_path(Path::new("README.md")).is_none());
        assert!(languages.for_path(Path::new("Makefile")).is_none());
    }

    #[test]
    fn finds_languages_by_name() {
        let languages = Languages::new();
        assert!(languages.by_name("Rust").is_some());
        assert!(languages.by_name("cobol").is_none());
    }
}
