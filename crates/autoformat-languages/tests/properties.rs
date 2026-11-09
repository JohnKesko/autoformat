//! Properties that must hold for every language and every configuration:
//! - formatting succeeds on all fixtures,
//! - formatting twice gives the same result as formatting once,
//! - only whitespace changes,
//! - code that a broken rule would damage is protected by the safety check.

use std::path::{Path, PathBuf};

use autoformat_core::config::{BraceStyle, Config, IndentStyle, LineEnding};
use autoformat_core::{format_source, LanguageAdapter, LanguageRules, Spacing, TokenContext};
use autoformat_languages::Languages;

fn fixture_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for language in [
        "rust",
        "toml",
        "javascript",
        "typescript",
        "csharp",
        "powershell",
    ] {
        let directory = root.join(language).join("tests/fixtures");
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.to_string_lossy().contains(".input.") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn configurations() -> Vec<Config> {
    vec![
        Config::default(),
        Config {
            indent_size: 2,
            ..Config::default()
        },
        Config {
            indent_style: IndentStyle::Tabs,
            ..Config::default()
        },
        Config {
            column_width: 40,
            ..Config::default()
        },
        Config {
            column_width: 60,
            comment_alignment: false,
            line_ending: LineEnding::Crlf,
            ..Config::default()
        },
        Config {
            brace_style: BraceStyle::Allman,
            ..Config::default()
        },
        Config {
            brace_style: BraceStyle::Gnu,
            indent_size: 2,
            column_width: 60,
            ..Config::default()
        },
        Config {
            brace_style: BraceStyle::Allman,
            language_brace_styles: vec![("csharp".to_string(), BraceStyle::SameLine)],
            ..Config::default()
        },
    ]
}

fn without_whitespace(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn every_fixture_is_idempotent_and_only_whitespace_changes() {
    let languages = Languages::new();
    let files = fixture_files();
    assert!(files.len() >= 12, "expected fixtures for every language");
    for path in files {
        let adapter = languages.for_path(&path).unwrap();
        let source = std::fs::read_to_string(&path).unwrap();
        for config in configurations() {
            let first = match format_source(adapter, &source, &config) {
                Ok(formatted) => {
                    if config.column_width >= 60 {
                        // Brace moves and normal wrapping never need a safety fallback.
                        assert!(
                            formatted.notes.is_empty(),
                            "{} with {:?}: {:?}",
                            path.display(),
                            config,
                            formatted.notes
                        );
                    }
                    formatted.text
                }
                Err(error) => panic!("{} with {:?}: {}", path.display(), config, error),
            };
            assert_eq!(
                without_whitespace(&first),
                without_whitespace(&source),
                "{} with {:?}: non-whitespace text changed",
                path.display(),
                config
            );
            let second = format_source(adapter, &first, &config).unwrap().text;
            assert_eq!(
                first,
                second,
                "{} with {:?} is not idempotent",
                path.display(),
                config
            );
            if config.line_ending == LineEnding::Crlf {
                assert!(
                    !first.replace("\r\n", "").contains('\n'),
                    "{}: stray LF",
                    path.display()
                );
            }
        }
    }
}

/// A deliberately broken adapter: it removes every space it can. In PowerShell this
/// would glue `Get-Item` and `-Path` into one word and change the program.
struct BrokenPowerShell {
    inner: autoformat_lang_powershell::PowerShellLanguage,
}

impl LanguageAdapter for BrokenPowerShell {
    fn name(&self) -> &'static str {
        "broken"
    }
    fn extensions(&self) -> &'static [&'static str] {
        &[]
    }
    fn grammar(&self) -> &autoformat_core::parser::Grammar {
        self.inner.grammar()
    }
    fn rules(&self) -> &LanguageRules {
        self.inner.rules()
    }
    fn spacing(&self, _previous: &TokenContext, _next: &TokenContext) -> Option<Spacing> {
        Some(Spacing::NoSpace)
    }
}

#[test]
fn the_safety_check_rejects_spacing_that_changes_the_program() {
    let adapter = BrokenPowerShell {
        inner: autoformat_lang_powershell::PowerShellLanguage::new(),
    };
    let source = "Get-Item -Path $env:TEMP\n";
    let formatted = format_source(&adapter, source, &Config::default()).unwrap();
    // The broken spacing was refused, and the safe fallback kept the original spacing.
    assert_eq!(formatted.text, source);
    assert!(!formatted.notes.is_empty());
}

#[test]
fn syntax_errors_never_produce_output() {
    let languages = Languages::new();
    let broken = [
        ("rs", "fn main( {"),
        ("toml", "[a\nb = "),
        ("js", "function ( {"),
        ("ts", "let x: = 1;"),
        ("cs", "class { void }"),
        ("ps1", "function { if ( }"),
    ];
    for (extension, source) in broken {
        let adapter = languages.by_extension(extension).unwrap();
        let result = format_source(adapter, source, &Config::default());
        assert!(result.is_err(), "{} should fail: {:?}", extension, result);
    }
}

#[test]
fn comments_are_never_lost_or_duplicated() {
    let languages = Languages::new();
    let samples = [
        (
            "rs",
            "// a\nfn f() { // b\n    /* c */ let x = 1; // d\n}\n// e\n",
            "//",
        ),
        ("toml", "# a\nkey = 1 # b\n# c\n[t] # d\n", "#"),
        (
            "js",
            "// a\nfunction f() { /* b */ return 1; // c\n}\n",
            "//",
        ),
        (
            "cs",
            "// a\nclass A { // b\n    int x; /* c */ // d\n}\n",
            "//",
        ),
        (
            "ps1",
            "# a\nfunction F { # b\n    $x = 1 # c\n}\n<# d #>\n",
            "#",
        ),
    ];
    for (extension, source, marker) in samples {
        let adapter = languages.by_extension(extension).unwrap();
        let formatted = format_source(adapter, source, &Config::default())
            .unwrap()
            .text;
        assert_eq!(
            formatted.matches(marker).count(),
            source.matches(marker).count(),
            "{}: comment count changed\n{}",
            extension,
            formatted
        );
    }
}
