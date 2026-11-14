use std::path::Path;

use autoformat_core::config::{Config, IndentStyle};
use autoformat_core::testing::{assert_idempotent, check_fixtures, format_or_panic};
use autoformat_core::{format_source, FormatError};
use autoformat_lang_json::JsonLanguage;

fn format(source: &str) -> String {
    format_or_panic(&JsonLanguage::new(), source, &Config::default())
}

#[test]
fn fixtures() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let count = check_fixtures(&JsonLanguage::new(), &directory);
    assert!(count >= 2);
}

#[test]
fn spacing_is_tidied() {
    assert_eq!(
        format("{\"a\":1,\"b\" : [1,2 , 3],\"c\":{ }}\n"),
        "{ \"a\": 1, \"b\": [1, 2, 3], \"c\": {} }\n"
    );
}

#[test]
fn objects_are_indented() {
    // The blank line keeps both objects on several lines.
    let source = "{\n\"a\": 1,\n        \"b\": {\n\"c\": [ ],\n\n  \"d\": null\n}\n}\n";
    let expected =
        "{\n    \"a\": 1,\n    \"b\": {\n        \"c\": [],\n\n        \"d\": null\n    }\n}\n";
    assert_eq!(format(source), expected);

    let tabs = Config {
        indent_style: IndentStyle::Tabs,
        ..Config::default()
    };
    let with_tabs = format_or_panic(&JsonLanguage::new(), source, &tabs);
    assert_eq!(
        with_tabs,
        "{\n\t\"a\": 1,\n\t\"b\": {\n\t\t\"c\": [],\n\n\t\t\"d\": null\n\t}\n}\n"
    );

    let two = Config {
        indent_size: 2,
        ..Config::default()
    };
    let with_two = format_or_panic(&JsonLanguage::new(), source, &two);
    assert_eq!(
        with_two,
        "{\n  \"a\": 1,\n  \"b\": {\n    \"c\": [],\n\n    \"d\": null\n  }\n}\n"
    );
}

#[test]
fn strings_and_numbers_are_untouched() {
    let source =
        "{ \"a  b\": \"x ,  y:z  [ ] { } // no /* comment */\", \"n\": [-0.50, 1E2, 1e-07] }\n";
    assert_eq!(format(source), source);
}

#[test]
fn long_arrays_are_wrapped_and_short_ones_are_joined() {
    let narrow = Config {
        column_width: 30,
        ..Config::default()
    };
    let source = "[\"alpha\", \"beta\", \"gamma\", \"delta\"]\n";
    let wrapped = "[\n    \"alpha\",\n    \"beta\",\n    \"gamma\",\n    \"delta\"\n]\n";
    let formatted = format_or_panic(&JsonLanguage::new(), source, &narrow);
    assert_eq!(formatted, wrapped);
    assert_idempotent(&JsonLanguage::new(), &formatted, &narrow, "array");
    // With room again, the array goes back on one line.
    assert_eq!(format(wrapped), source);
}

#[test]
fn long_objects_are_wrapped_and_short_ones_are_joined() {
    let narrow = Config {
        column_width: 30,
        ..Config::default()
    };
    let one_line = "{ \"x\": 1000000, \"y\": 2000000, \"z\": 3000000 }\n";
    let wrapped = "{\n    \"x\": 1000000,\n    \"y\": 2000000,\n    \"z\": 3000000\n}\n";
    let formatted = format_or_panic(&JsonLanguage::new(), one_line, &narrow);
    assert_eq!(formatted, wrapped);
    assert_idempotent(&JsonLanguage::new(), &formatted, &narrow, "object");
    // With room again, the object goes back on one line.
    assert_eq!(format(wrapped), one_line);
}

#[test]
fn only_the_outer_lists_are_wrapped_when_that_is_enough() {
    let narrow = Config {
        column_width: 40,
        ..Config::default()
    };
    let source =
        "{\"name\":\"demo\",\"point\":{\"x\":1,\"y\":2},\"tags\":[\"a\",\"b\"],\"empty\":{}}\n";
    let expected = "{\n    \"name\": \"demo\",\n    \"point\": { \"x\": 1, \"y\": 2 },\n    \"tags\": [\"a\", \"b\"],\n    \"empty\": {}\n}\n";
    assert_eq!(
        format_or_panic(&JsonLanguage::new(), source, &narrow),
        expected
    );
}

#[test]
fn an_object_with_a_comment_keeps_its_line_breaks() {
    let source = "{\n    \"x\": 1,  // first\n    \"y\": 2\n}\n";
    assert_eq!(format(source), source);
}

#[test]
fn comments_are_kept_in_jsonc() {
    let source = "// top\n{\n/* before */ \"a\":1, // after\n  // alone\n\"b\":[1, /* inside */ 2]\n}\n// bottom\n";
    let expected = "// top\n{\n    /* before */ \"a\": 1,  // after\n    // alone\n    \"b\": [1, /* inside */ 2]\n}\n// bottom\n";
    let formatted = format(source);
    assert_eq!(formatted, expected);
    assert_idempotent(
        &JsonLanguage::new(),
        &formatted,
        &Config::default(),
        "comments",
    );
}

#[test]
fn the_final_newline_and_trailing_spaces_follow_the_settings() {
    // The blank line keeps the object on several lines.
    let source = "{\n    \"a\": 1,   \n\n    \"b\": 2\n}\n\n\n";
    assert_eq!(format(source), "{\n    \"a\": 1,\n\n    \"b\": 2\n}\n");

    let no_newline = Config {
        final_newline: false,
        ..Config::default()
    };
    assert_eq!(
        format_or_panic(&JsonLanguage::new(), "{ \"a\": 1 }", &no_newline),
        "{ \"a\": 1 }"
    );
    assert_eq!(format("{ \"a\": 1 }"), "{ \"a\": 1 }\n");

    let keep_spaces = Config {
        trim_trailing_whitespace: false,
        ..Config::default()
    };
    assert_eq!(
        format_or_panic(&JsonLanguage::new(), source, &keep_spaces),
        "{\n    \"a\": 1,   \n\n    \"b\": 2\n}\n"
    );
}

#[test]
fn an_empty_file_stays_empty() {
    assert_eq!(format(""), "");
}

#[test]
fn a_file_with_a_syntax_error_is_rejected() {
    let broken = [
        "{ \"a\": }\n",
        "{ \"a\": 1\n",
        "{ 'a': 1 }\n",
        "{ a: 1 }\n",
        "[1 2]\n",
    ];
    for source in broken {
        let result = format_source(&JsonLanguage::new(), source, &Config::default());
        assert!(
            matches!(result, Err(FormatError::Syntax(_))),
            "{:?}",
            result
        );
    }
}

/// The grammar does not accept a comma after the last item. Such a file is left alone
/// instead of having the comma removed.
#[test]
fn a_comma_after_the_last_item_is_a_syntax_error() {
    for source in ["[1, 2,]\n", "{ \"a\": 1, }\n"] {
        let result = format_source(&JsonLanguage::new(), source, &Config::default());
        assert!(
            matches!(result, Err(FormatError::Syntax(_))),
            "{:?}",
            result
        );
    }
}
