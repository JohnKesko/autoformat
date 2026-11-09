use std::path::Path;

use autoformat_core::config::{Config, IndentStyle, LineEnding};
use autoformat_core::testing::{assert_idempotent, check_fixtures, format_or_panic};
use autoformat_core::{format_source, FormatError};
use autoformat_lang_rust::RustLanguage;

#[test]
fn fixtures() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let count = check_fixtures(&RustLanguage::new(), &directory);
    assert!(count >= 4);
}

#[test]
fn strings_and_raw_strings_are_untouched() {
    let source = "fn f() {\n    let a = \"x  =  y\";\n    let b = r#\"  {  }  \"#;\n    let c = b\"  \";\n}\n";
    let formatted = format_or_panic(&RustLanguage::new(), source, &Config::default());
    assert_eq!(formatted, source);
}

#[test]
fn multi_line_strings_keep_their_lines() {
    let source = "fn f() {\n        let a = \"first\n   second\n third\";\n}\n";
    let expected = "fn f() {\n    let a = \"first\n   second\n third\";\n}\n";
    assert_eq!(
        format_or_panic(&RustLanguage::new(), source, &Config::default()),
        expected
    );
}

#[test]
fn syntax_errors_are_reported_and_nothing_changes() {
    let result = format_source(&RustLanguage::new(), "fn main( {\n", &Config::default());
    match result {
        Err(FormatError::Syntax(error)) => assert_eq!(error.line, 1),
        other => panic!("expected a syntax error, got {:?}", other),
    }
}

#[test]
fn tabs_and_crlf_are_applied() {
    let config = Config {
        indent_style: IndentStyle::Tabs,
        line_ending: LineEnding::Crlf,
        ..Config::default()
    };
    let source = "fn main() {\n  if true {\n    run();\n  }\n}\n";
    let expected = "fn main() {\r\n\tif true {\r\n\t\trun();\r\n\t}\r\n}\r\n";
    let formatted = format_or_panic(&RustLanguage::new(), source, &config);
    assert_eq!(formatted, expected);
    assert_idempotent(&RustLanguage::new(), &formatted, &config, "crlf");
}

#[test]
fn indent_size_is_applied() {
    let config = Config {
        indent_size: 2,
        ..Config::default()
    };
    let source = "fn main() {\n        run();\n}\n";
    assert_eq!(
        format_or_panic(&RustLanguage::new(), source, &config),
        "fn main() {\n  run();\n}\n"
    );
}

#[test]
fn long_lines_are_wrapped_at_the_column_width() {
    let config = Config {
        column_width: 40,
        ..Config::default()
    };
    let source = "fn main() {\n    let total = add_numbers(first_value, second_value, third_value);\n}\n";
    let expected = "fn main() {\n    let total = add_numbers(\n        first_value,\n        second_value,\n        third_value\n    );\n}\n";
    let formatted = format_or_panic(&RustLanguage::new(), source, &config);
    assert_eq!(formatted, expected);
    assert_idempotent(&RustLanguage::new(), &formatted, &config, "wrap");
}

#[test]
fn lines_that_cannot_be_wrapped_are_left_alone() {
    let config = Config {
        column_width: 20,
        ..Config::default()
    };
    let source = "fn main() {\n    let message = \"this string is much longer than twenty characters\";\n}\n";
    assert_eq!(format_or_panic(&RustLanguage::new(), source, &config), source);
}

#[test]
fn comment_alignment_can_be_disabled() {
    let config = Config {
        comment_alignment: false,
        ..Config::default()
    };
    let source = "fn main() {\n    let a = 1;     // one\n    let bb = 2; // two\n}\n";
    let expected = "fn main() {\n    let a = 1; // one\n    let bb = 2; // two\n}\n";
    assert_eq!(format_or_panic(&RustLanguage::new(), source, &config), expected);
}

#[test]
fn empty_input_stays_empty() {
    assert_eq!(format_or_panic(&RustLanguage::new(), "", &Config::default()), "");
    assert_eq!(format_or_panic(&RustLanguage::new(), "\n\n", &Config::default()), "");
}
