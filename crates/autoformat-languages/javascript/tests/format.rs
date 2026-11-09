use std::path::Path;

use autoformat_core::config::Config;
use autoformat_core::testing::{check_fixtures, format_or_panic};
use autoformat_lang_javascript::JavaScriptLanguage;

#[test]
fn fixtures() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let count = check_fixtures(&JavaScriptLanguage::new(), &directory);
    assert!(count >= 2);
}

#[test]
fn template_strings_and_regexes_are_untouched() {
    let source = "const a = `x  ${ y  }  z`;\nconst b = /a  b/g;\nconst c = \"q  r\";\n";
    assert_eq!(
        format_or_panic(&JavaScriptLanguage::new(), source, &Config::default()),
        source
    );
}

#[test]
fn lines_without_semicolons_are_kept_apart() {
    // Joining these lines would change the program (automatic semicolon insertion).
    let source = "function f() {\nlet a = b\n;(c || d).run()\nreturn\nvalue\n}\n";
    let expected =
        "function f() {\n    let a = b\n    ; (c || d).run()\n    return\n    value\n}\n";
    assert_eq!(
        format_or_panic(&JavaScriptLanguage::new(), source, &Config::default()),
        expected
    );
}

#[test]
fn jsx_text_spacing_is_preserved() {
    let source = "const a = <p>one   two <b>three</b>four</p>;\n";
    assert_eq!(
        format_or_panic(&JavaScriptLanguage::new(), source, &Config::default()),
        source
    );
}

#[test]
fn long_calls_are_wrapped() {
    let config = Config {
        column_width: 50,
        ..Config::default()
    };
    let source = "const result = compute(firstArgument, secondArgument, thirdArgument);\n";
    let expected =
        "const result = compute(\n    firstArgument,\n    secondArgument,\n    thirdArgument\n);\n";
    assert_eq!(
        format_or_panic(&JavaScriptLanguage::new(), source, &config),
        expected
    );
}
