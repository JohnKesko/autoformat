//! Helpers for language adapter tests.
//!
//! Each adapter keeps fixture files in `tests/fixtures`:
//! - `name.input.<ext>`: source as a person might write it,
//! - `name.expected.<ext>`: the expected output with the default configuration.
//!
//! [`check_fixtures`] formats every input, compares it with the expected file, and checks
//! that formatting is idempotent (formatting the expected output changes nothing).
//! Set `AUTOFORMAT_BLESS=1` to write missing or changed expected files.

use std::path::Path;

use autoformat_config::Config;

use crate::adapter::LanguageAdapter;
use crate::engine::format_source;

/// Formats `source` with the default configuration and panics on errors.
pub fn format_or_panic(adapter: &dyn LanguageAdapter, source: &str, config: &Config) -> String {
    match format_source(adapter, source, config) {
        Ok(formatted) => {
            assert!(
                formatted.notes.is_empty(),
                "formatting used a fallback: {:?}\n--- source ---\n{}",
                formatted.notes,
                source
            );
            formatted.text
        }
        Err(error) => panic!("formatting failed: {}\n--- source ---\n{}", error, source),
    }
}

/// Asserts that formatting the text again changes nothing.
pub fn assert_idempotent(adapter: &dyn LanguageAdapter, formatted: &str, config: &Config, name: &str) {
    let again = format_or_panic(adapter, formatted, config);
    if again != formatted {
        panic!(
            "formatting is not idempotent for {}\n--- first ---\n{}\n--- second ---\n{}",
            name, formatted, again
        );
    }
}

/// Checks every fixture pair in `directory`. Returns the number of fixtures checked.
pub fn check_fixtures(adapter: &dyn LanguageAdapter, directory: &Path) -> usize {
    let bless = std::env::var("AUTOFORMAT_BLESS").is_ok();
    let config = Config::default();
    let mut entries: Vec<_> = std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {}", directory.display(), error))
        .map(|entry| entry.unwrap().path())
        .collect();
    entries.sort();

    let mut checked = 0;
    let mut failures = Vec::new();
    for input_path in entries {
        let file_name = input_path.file_name().unwrap().to_string_lossy().to_string();
        if !file_name.contains(".input.") {
            continue;
        }
        let extension = input_path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        if !adapter.extensions().contains(&extension.as_str()) {
            continue;
        }
        let expected_name = file_name.replace(".input.", ".expected.");
        let expected_path = directory.join(&expected_name);
        let source = std::fs::read_to_string(&input_path).unwrap();
        let formatted = format_or_panic(adapter, &source, &config);
        assert_idempotent(adapter, &formatted, &config, &file_name);

        let expected = std::fs::read_to_string(&expected_path).ok();
        if expected.as_deref() != Some(formatted.as_str()) {
            if bless {
                std::fs::write(&expected_path, &formatted).unwrap();
            } else {
                failures.push(format!(
                    "{} does not match {}\n--- actual ---\n{}",
                    file_name, expected_name, formatted
                ));
            }
        }
        checked += 1;
    }
    if !failures.is_empty() {
        panic!("{}", failures.join("\n"));
    }
    checked
}
