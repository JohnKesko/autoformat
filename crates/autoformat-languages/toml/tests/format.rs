use std::path::Path;

use autoformat_core::config::Config;
use autoformat_core::testing::{assert_idempotent, check_fixtures, format_or_panic};
use autoformat_core::{format_source, FormatError};
use autoformat_lang_toml::TomlLanguage;

#[test]
fn fixtures() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let count = check_fixtures(&TomlLanguage::new(), &directory);
    assert!(count >= 2);
}

#[test]
fn multi_line_strings_are_untouched() {
    let source = "text = \"\"\"\n   keep   this\n\"\"\"\nraw = '''\n  and this  \n'''\n";
    assert_eq!(
        format_or_panic(&TomlLanguage::new(), source, &Config::default()),
        source
    );
}

#[test]
fn long_arrays_are_wrapped() {
    let config = Config {
        column_width: 30,
        ..Config::default()
    };
    let source = "values = [\"alpha\", \"beta\", \"gamma\", \"delta\"]\n";
    let expected = "values = [\n    \"alpha\",\n    \"beta\",\n    \"gamma\",\n    \"delta\"\n]\n";
    let formatted = format_or_panic(&TomlLanguage::new(), source, &config);
    assert_eq!(formatted, expected);
    assert_idempotent(&TomlLanguage::new(), &formatted, &config, "array");
}

#[test]
fn inline_tables_are_never_wrapped() {
    let config = Config {
        column_width: 20,
        ..Config::default()
    };
    let source = "point = { x = 1000000, y = 2000000, z = 3000000 }\n";
    assert_eq!(format_or_panic(&TomlLanguage::new(), source, &config), source);
}

#[test]
fn invalid_toml_is_rejected() {
    let result = format_source(&TomlLanguage::new(), "[table\nkey = 1\n", &Config::default());
    assert!(matches!(result, Err(FormatError::Syntax(_))), "{:?}", result);
}
