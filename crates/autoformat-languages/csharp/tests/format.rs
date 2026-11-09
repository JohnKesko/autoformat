use std::path::Path;

use autoformat_core::config::{BraceStyle, Config};
use autoformat_core::testing::{check_fixtures, format_or_panic};
use autoformat_lang_csharp::CSharpLanguage;

#[test]
fn fixtures() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let count = check_fixtures(&CSharpLanguage::new(), &directory);
    assert!(count >= 2);
}

/// These tests are written with braces on their own lines, so they use that style.
fn allman() -> Config {
    Config {
        brace_style: BraceStyle::Allman,
        ..Config::default()
    }
}

#[test]
fn special_strings_are_untouched() {
    let source = "class A\n{\n    string a = $\"{x,5}  {y:N2}\";\n    string b = @\"C:\\path  \"\"q\"\"\";\n    string c = \"\"\"\n      raw  text\n      \"\"\";\n}\n";
    assert_eq!(
        format_or_panic(&CSharpLanguage::new(), source, &allman()),
        source
    );
}

#[test]
fn preprocessor_directives_stay_at_column_zero() {
    let source = "class A\n{\n    void M()\n    {\n        #if DEBUG\n        Log();\n        #endif\n    }\n}\n";
    let expected = "class A\n{\n    void M()\n    {\n#if DEBUG\n        Log();\n#endif\n    }\n}\n";
    assert_eq!(
        format_or_panic(&CSharpLanguage::new(), source, &allman()),
        expected
    );
}

#[test]
fn regions_are_indented_like_code() {
    let source = "class A\n{\n#region Fields\nint x;\n#endregion\n}\n";
    let expected = "class A\n{\n    #region Fields\n    int x;\n    #endregion\n}\n";
    assert_eq!(
        format_or_panic(&CSharpLanguage::new(), source, &allman()),
        expected
    );
}
