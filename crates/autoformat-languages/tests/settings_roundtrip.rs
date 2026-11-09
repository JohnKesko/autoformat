//! Changing a setting and then changing it back must give the same result as before:
//! format with B, then with A, then with B again == format with B.
//! This is what lets people try other settings without being stuck with the old ones.

use std::path::{Path, PathBuf};

use autoformat_core::config::{BraceStyle, Config, IndentStyle, LineEnding};
use autoformat_core::format_source;
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

/// Pairs of settings that differ in one option each.
fn setting_pairs() -> Vec<(&'static str, Config, Config)> {
    let base = Config::default();
    vec![
        (
            "column_width 140 <-> 40",
            base.clone(),
            Config {
                column_width: 40,
                ..base.clone()
            },
        ),
        (
            "column_width 140 <-> 80",
            base.clone(),
            Config {
                column_width: 80,
                ..base.clone()
            },
        ),
        (
            "indent_size 4 <-> 2",
            base.clone(),
            Config {
                indent_size: 2,
                ..base.clone()
            },
        ),
        (
            "indent_size 4 <-> 8",
            base.clone(),
            Config {
                indent_size: 8,
                ..base.clone()
            },
        ),
        (
            "indent_style spaces <-> tabs",
            base.clone(),
            Config {
                indent_style: IndentStyle::Tabs,
                ..base.clone()
            },
        ),
        (
            "comment_alignment on <-> off",
            base.clone(),
            Config {
                comment_alignment: false,
                ..base.clone()
            },
        ),
        (
            "line_ending lf <-> crlf",
            base.clone(),
            Config {
                line_ending: LineEnding::Crlf,
                ..base.clone()
            },
        ),
        (
            "brace_style same_line <-> allman",
            base.clone(),
            Config {
                brace_style: BraceStyle::Allman,
                ..base.clone()
            },
        ),
        (
            "brace_style same_line <-> gnu",
            base.clone(),
            Config {
                brace_style: BraceStyle::Gnu,
                ..base.clone()
            },
        ),
        (
            "brace_style allman <-> gnu",
            Config {
                brace_style: BraceStyle::Allman,
                ..base.clone()
            },
            Config {
                brace_style: BraceStyle::Gnu,
                ..base.clone()
            },
        ),
        (
            "final_newline true <-> false",
            base.clone(),
            Config {
                final_newline: false,
                ..base.clone()
            },
        ),
        (
            "blank_lines true <-> false",
            base.clone(),
            Config {
                blank_lines: false,
                ..base.clone()
            },
        ),
        (
            "space_around_operators true <-> false",
            base.clone(),
            Config {
                space_around_operators: false,
                ..base.clone()
            },
        ),
        (
            "blank_lines and space_around_operators false, allman <-> defaults",
            base.clone(),
            Config {
                blank_lines: false,
                space_around_operators: false,
                brace_style: BraceStyle::Allman,
                ..base.clone()
            },
        ),
    ]
}

fn format(path: &Path, source: &str, config: &Config) -> String {
    let languages = Languages::new();
    let adapter = languages.for_path(path).unwrap();
    match format_source(adapter, source, config) {
        Ok(formatted) => formatted.text,
        Err(error) => panic!("{}: {}", path.display(), error),
    }
}

/// `trim_trailing_whitespace = false` means "never remove spaces at the end of a line".
/// There is nothing for it to put back, so it is checked on its own: turning it off
/// changes nothing, and turning it on again gives the same result as before.
fn trim_pair() -> (Config, Config) {
    let on = Config::default();
    let off = Config {
        trim_trailing_whitespace: false,
        ..Config::default()
    };
    (on, off)
}

#[test]
fn every_setting_can_be_changed_back() {
    let mut failures = Vec::new();
    for path in fixture_files() {
        let source = std::fs::read_to_string(&path).unwrap();
        for (name, first, second) in setting_pairs() {
            // Both directions: first -> second -> first, and second -> first -> second.
            for (b, a) in [(&first, &second), (&second, &first)] {
                let with_b = format(&path, &source, b);
                let with_a = format(&path, &with_b, a);
                let back = format(&path, &with_a, b);
                if back != with_b {
                    failures.push(format!(
                        "{}  {}",
                        name,
                        path.file_name().unwrap().to_string_lossy()
                    ));
                }
            }
        }
    }
    failures.dedup();
    assert!(
        failures.is_empty(),
        "settings that could not be changed back:\n{}",
        failures.join("\n")
    );
}

#[test]
fn trimming_can_be_turned_off_and_on_again() {
    let (on, off) = trim_pair();
    let mut failures = Vec::new();
    let mut inputs: Vec<(PathBuf, String)> = Vec::new();
    for path in fixture_files() {
        let source = std::fs::read_to_string(&path).unwrap();
        inputs.push((path, source));
    }
    for (name, source) in TRICKY_SOURCES {
        inputs.push((PathBuf::from(name), source.to_string()));
    }
    for (path, source) in &inputs {
        let with_on = format(path, source, &on);
        let with_off = format(path, &with_on, &off);
        let back = format(path, &with_off, &on);
        if with_off != with_on || back != with_on {
            failures.push(path.file_name().unwrap().to_string_lossy().to_string());
        }
    }
    assert!(
        failures.is_empty(),
        "trim_trailing_whitespace changed code when turned off, or could not be turned on again:\n{}",
        failures.join("\n")
    );
}

/// Small inputs with layouts that are easy to get wrong.
const TRICKY_SOURCES: &[(&str, &str)] = &[
    ("a.rs", "pub async fn incoming(stream: TcpStream, tls_acceptor: TlsAcceptor, context: Arc<Context>) {}\n"),
    ("b.rs", "fn f() {\n    call_something(first_argument, second_argument,\n        third_argument);\n}\n"),
    ("c.rs", "fn f() {\n    let list = vec![\n        1,\n        2,\n    ];\n    let other = (alpha, beta, gamma, delta, epsilon, zeta, eta, theta, iota, kappa, lambda,);\n}\n"),
    ("d.rs", "fn f() {\n    call(\n        a, // first\n        b\n    );\n}\n"),
    ("e.rs", "fn f() { run();\n    stop(); }\n"),
    ("f.ts", "const settings = { name: 'demo', size: 3, enabled: true, tags: ['alpha', 'beta', 'gamma', 'delta'] };\n"),
    ("g.cs", "class A\n{\n    void Run(int first, int second, int third, int fourth, int fifth, int sixth) { Go(first, second); }\n}\n"),
    ("h.toml", "values = [\"alpha\", \"beta\", \"gamma\", \"delta\", \"epsilon\", \"zeta\", \"eta\", \"theta\"]\n"),
    ("i.ps1", "$result = $client.Invoke($firstArgument, $secondArgument, $thirdArgument, $fourth)\n"),
];

#[test]
fn tricky_layouts_can_be_changed_back() {
    let mut failures = Vec::new();
    for (name, source) in TRICKY_SOURCES {
        let path = Path::new(name);
        for (setting, first, second) in setting_pairs() {
            for (b, a) in [(&first, &second), (&second, &first)] {
                let with_b = format(path, source, b);
                let back = format(path, &format(path, &with_b, a), b);
                if back != with_b {
                    failures.push(format!(
                        "{}  {}\n--- expected ---\n{}--- got ---\n{}",
                        setting, name, with_b, back
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_wrapped_list_goes_back_on_one_line_when_it_fits() {
    let source = "pub async fn incoming(stream: TcpStream, tls_acceptor: TlsAcceptor, context: Arc<Context>) {}\n";
    let narrow = Config {
        column_width: 80,
        ..Config::default()
    };
    let wrapped = format(Path::new("a.rs"), source, &narrow);
    assert_eq!(
        wrapped,
        "pub async fn incoming(\n    stream: TcpStream,\n    tls_acceptor: TlsAcceptor,\n    context: Arc<Context>\n) {}\n"
    );
    let wide = format(Path::new("a.rs"), &wrapped, &Config::default());
    assert_eq!(wide, source);
}

#[test]
fn lists_with_a_final_comma_or_a_comment_keep_their_layout() {
    let wide = Config::default();
    let narrow = Config {
        column_width: 40,
        ..Config::default()
    };
    let kept = [
        "fn f() {\n    call(\n        a,\n        b,\n    );\n}\n",
        "fn f() {\n    call(\n        a,  // first\n        b\n    );\n}\n",
        "fn f() {\n    let t = (one_long_name, another_long_name, a_third_long_name,);\n}\n",
    ];
    for source in kept {
        assert_eq!(format(Path::new("k.rs"), source, &wide), source);
        assert_eq!(format(Path::new("k.rs"), source, &narrow), source);
    }
}
