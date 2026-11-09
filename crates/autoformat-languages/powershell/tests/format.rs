use std::path::Path;

use autoformat_core::config::Config;
use autoformat_core::testing::{check_fixtures, format_or_panic};
use autoformat_lang_powershell::PowerShellLanguage;

#[test]
fn fixtures() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let count = check_fixtures(&PowerShellLanguage::new(), &directory);
    assert!(count >= 2);
}

#[test]
fn here_strings_are_untouched() {
    let source = "function F {\n$a = @'\n  literal  $text\n'@\n$b = @\"\n    expanded $x\n\"@\n}\n";
    let expected =
        "function F {\n    $a = @'\n  literal  $text\n'@\n    $b = @\"\n    expanded $x\n\"@\n}\n";
    assert_eq!(
        format_or_panic(&PowerShellLanguage::new(), source, &Config::default()),
        expected
    );
}

#[test]
fn command_arguments_keep_their_spacing() {
    // `-Path` must stay separated from its value, and `.` stays an argument.
    let source = "Get-ChildItem -Path . -Recurse\nGet-Item $a[0]\n";
    assert_eq!(
        format_or_panic(&PowerShellLanguage::new(), source, &Config::default()),
        source
    );
}

#[test]
fn backtick_continuation_is_kept() {
    let source = "Get-Item -Path x `\n-Force\n";
    let expected = "Get-Item -Path x `\n    -Force\n";
    assert_eq!(
        format_or_panic(&PowerShellLanguage::new(), source, &Config::default()),
        expected
    );
}
