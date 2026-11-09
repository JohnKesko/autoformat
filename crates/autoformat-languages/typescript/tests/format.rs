use std::path::Path;

use autoformat_core::config::Config;
use autoformat_core::testing::{check_fixtures, format_or_panic};
use autoformat_lang_typescript::{TsxLanguage, TypeScriptLanguage};

#[test]
fn typescript_fixtures() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let count = check_fixtures(&TypeScriptLanguage::new(), &directory);
    assert!(count >= 2);
}

#[test]
fn tsx_fixtures() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let count = check_fixtures(&TsxLanguage::new(), &directory);
    assert!(count >= 1);
}

#[test]
fn generics_and_optional_members() {
    let source = "class A<T=string>{x?:T;y!:number}\nlet f=<T,>(v:T):T=>v;\n";
    let expected = "class A<T = string> { x?: T; y!: number }\nlet f = <T,>(v: T): T => v;\n";
    let formatted = format_or_panic(&TypeScriptLanguage::new(), source, &Config::default());
    assert_eq!(formatted, expected);
}

#[test]
fn template_literal_types_are_untouched() {
    let source = "type Route = `/api/${ string }`;\n";
    assert_eq!(
        format_or_panic(&TypeScriptLanguage::new(), source, &Config::default()),
        source
    );
}
