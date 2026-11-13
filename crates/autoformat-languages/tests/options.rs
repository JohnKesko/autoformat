//! Tests for blank_lines, space_around_operators, final_newline, trim_trailing_whitespace,
//! continuation_indent, line endings, per-language settings and the C# property pattern regression.
//!
//! Every check formats the input, compares it with the expected text, and formats the
//! result again to prove that nothing changes the second time.

use autoformat_core::config::{parse_config, Config, LineEnding};
use autoformat_core::format_source;
use autoformat_core::parser::tree_sitter::Node;
use autoformat_languages::Languages;

fn format(extension: &str, source: &str, config: &Config) -> String {
    let languages = Languages::new();
    let adapter = languages.by_extension(extension).unwrap();
    match format_source(adapter, source, config) {
        Ok(formatted) => {
            assert!(
                formatted.notes.is_empty(),
                "a fallback was used: {:?}",
                formatted.notes
            );
            formatted.text
        }
        Err(error) => panic!("{}\n--- source ---\n{}", error, source),
    }
}

fn check_with(extension: &str, config: &Config, source: &str, expected: &str) {
    let first = format(extension, source, config);
    assert_eq!(
        first, expected,
        "\n--- got ---\n{}\n--- expected ---\n{}",
        first, expected
    );
    let second = format(extension, &first, config);
    assert_eq!(second, first, "not idempotent:\n{}", second);
}

fn check(extension: &str, source: &str, expected: &str) {
    check_with(extension, &Config::default(), source, expected);
}

fn settings(text: &str) -> Config {
    parse_config(text).unwrap()
}

// ---------------------------------------------------------------------------------------
// blank_lines

#[test]
fn blank_lines_separate_multi_line_rust_items() {
    let source = "\
use std::fmt;
struct Point {
    x: i32,
}
impl Point {
    fn new() -> Point {
        Point { x: 0 }
    }
    fn x(&self) -> i32 {
        self.x
    }
}
const A: u8 = 1;
const B: u8 = 2;
/// Adds one.
#[inline]
fn add_one(x: i32) -> i32 {
    x + 1
}
";
    let expected = "\
use std::fmt;

struct Point {
    x: i32,
}

impl Point {
    fn new() -> Point {
        Point { x: 0 }
    }

    fn x(&self) -> i32 {
        self.x
    }
}

const A: u8 = 1;
const B: u8 = 2;

/// Adds one.
#[inline]
fn add_one(x: i32) -> i32 {
    x + 1
}
";
    check("rs", source, expected);
}

#[test]
fn blank_lines_inside_code_are_left_alone() {
    // Statements inside a function are code, not declarations: no blank lines are
    // added between them, and the ones written by hand stay.
    let source = "fn main() {\n    let a = 1;\n    if a > 0 {\n        run();\n    }\n    stop();\n\n    finish();\n}\n";
    check("rs", source, source);
}

#[test]
fn blank_lines_can_be_turned_off() {
    let off = settings("[formatting]\nblank_lines = false\n");
    // `false` means no empty line between declarations that span several lines.
    let spaced = "fn a() {\n    one();\n}\n\n\n\nfn b() {\n    two();\n}\n";
    let compact = "fn a() {\n    one();\n}\nfn b() {\n    two();\n}\n";
    check_with("rs", &off, spaced, compact);
    check_with("rs", &off, compact, compact);
    // Empty lines inside function bodies and between one-line items are never touched.
    let untouched = "const A: u8 = 1;\n\nconst B: u8 = 2;\nfn c() {\n    one();\n\n    two();\n}\n";
    check_with("rs", &off, untouched, untouched);
}

#[test]
fn blank_lines_switch_back_and_forth() {
    let on = Config::default();
    let off = settings("[formatting]\nblank_lines = false\n");
    let cases = [
        ("rs", "fn a() {\n    one();\n}\n/// About b.\n#[inline]\nfn b() {\n    two();\n}\n"),
        ("rs", "fn a() {\n    one();\n}\n// Section\n\nfn b() {\n    two();\n}\n"),
        ("rs", "fn a() {\n    one();\n}  // end of a\nfn b() {}\n"),
        ("cs", "class A\n{\n    void One()\n    {\n    }\n    #endregion\n    // Two.\n    void Two() { }\n}\n"),
        ("ts", "export class A {\n    a() {\n        one();\n    }\n    b() {}\n}\nfunction c() {\n    two();\n}\nmain();\n"),
        ("ps1", "function A {\n    'a'\n}\nfunction B {\n    'b'\n}\n"),
        ("toml", "a = 1\n[t]\nb = 2\n\n[u]\nc = 3\n"),
    ];
    for (extension, source) in cases {
        switch_back_and_forth(extension, source, &on, &off);
    }
}

/// Changing a setting and changing it back must give the same code as before, in both
/// directions.
fn switch_back_and_forth(extension: &str, source: &str, first: &Config, second: &Config) {
    for (a, b) in [(first, second), (second, first)] {
        let with_a = format(extension, source, a);
        let with_b = format(extension, &with_a, b);
        let back = format(extension, &with_b, a);
        assert_eq!(
            back, with_a,
            "{}: changing the setting and back gave different code\n--- before ---\n{}--- after ---\n{}",
            extension, with_a, back
        );
    }
}

#[test]
fn blank_lines_in_csharp_classes() {
    let source = "\
namespace Shop
{
    public class Cart
    {
        private int count;
        public string Name { get; set; }
        public int Total
        {
            get { return count; }
        }
        // Adds one item.
        public void Add()
        {
            count++;
        }
        public void Clear() { count = 0; }
    }
}
";
    let expected = "\
namespace Shop {
    public class Cart {
        private int count;
        public string Name { get; set; }

        public int Total {
            get { return count; }
        }

        // Adds one item.
        public void Add() {
            count++;
        }

        public void Clear() { count = 0; }
    }
}
";
    check("cs", source, expected);
}

#[test]
fn blank_lines_in_typescript_and_javascript() {
    let source = "\
import { readFile } from 'fs';
const limit = 10;
const names = [
    'a',
    'b',
];
export interface Shape {
    area(): number;
}
export class Square implements Shape {
    size = 1;
    area(): number {
        return this.size * this.size;
    }
    grow(): void {
        this.size++;
    }
}
";
    // `const` statements are ordinary code in scripts, so only declarations get space.
    let expected = "\
import { readFile } from 'fs';
const limit = 10;
const names = [
    'a',
    'b',
];

export interface Shape {
    area(): number;
}

export class Square implements Shape {
    size = 1;

    area(): number {
        return this.size * this.size;
    }

    grow(): void {
        this.size++;
    }
}
";
    check("ts", source, expected);
    let javascript = "function a() {\n    one();\n}\nfunction b() {\n    two();\n}\nmain();\n";
    let expected = "function a() {\n    one();\n}\n\nfunction b() {\n    two();\n}\n\nmain();\n";
    check("js", javascript, expected);
}

#[test]
fn blank_lines_in_powershell_and_toml() {
    let powershell = "function A {\n    'a'\n}\nfunction B {\n    'b'\n}\nA\nB\n";
    let expected = "function A {\n    'a'\n}\n\nfunction B {\n    'b'\n}\n\nA\nB\n";
    check("ps1", powershell, expected);

    let toml = "name = \"demo\"\n[dependencies]\nserde = \"1\"\n[profile.release]\nlto = true\n";
    let expected =
        "name = \"demo\"\n\n[dependencies]\nserde = \"1\"\n\n[profile.release]\nlto = true\n";
    check("toml", toml, expected);
}

#[test]
fn blank_lines_in_nested_declarations() {
    let source = "mod outer {\n    mod inner {\n        fn a() {\n            one();\n        }\n        fn b() {\n            two();\n        }\n    }\n    fn c() {}\n}\n";
    let expected = "mod outer {\n    mod inner {\n        fn a() {\n            one();\n        }\n\n        fn b() {\n            two();\n        }\n    }\n\n    fn c() {}\n}\n";
    check("rs", source, expected);
}

// ---------------------------------------------------------------------------------------
// space_around_operators

#[test]
fn operators_get_spaces_in_every_language() {
    check(
        "js",
        "const result=a+b*10;\n",
        "const result = a + b * 10;\n",
    );
    check(
        "ts",
        "let ok:boolean=a&&b||!c;\n",
        "let ok: boolean = a && b || !c;\n",
    );
    check(
        "cs",
        "var total=price*count-discount;\n",
        "var total = price * count - discount;\n",
    );
    check(
        "rs",
        "fn f() {\n    let x=a<<2|b;\n}\n",
        "fn f() {\n    let x = a << 2 | b;\n}\n",
    );
    check("ps1", "$total=$a+1\n", "$total = $a+1\n");
}

#[test]
fn unary_operators_stay_attached() {
    check(
        "js",
        "let y = - x;\nlet z = ! done;\ni ++;\n",
        "let y = -x;\nlet z = !done;\ni++;\n",
    );
    check(
        "rs",
        "fn f() {\n    let y = - x;\n    let r = & value;\n}\n",
        "fn f() {\n    let y = -x;\n    let r = &value;\n}\n",
    );
}

#[test]
fn operators_inside_strings_and_comments_are_never_touched() {
    let source = "const s = \"a+b=c\";\nconst t = `x*y ${a+b}`;\n// a+b=c\nconst r = /a+b/;\n";
    check("js", source, source);
    // Comments at the end of a line get two spaces in front (comment_alignment = "align").
    let rust = "fn f() {\n    let s = \"1+1=2\";  // a+b\n    println!(\"{}\", 1+1);\n}\n";
    check("rs", rust, rust);
}

#[test]
fn operator_spacing_can_be_turned_off() {
    let off = settings("[formatting]\nspace_around_operators = false\n");
    // `false` means no spaces around operators between two values.
    check_with(
        "js",
        &off,
        "const result = a + b * 10;\nfoo(a,b);\n",
        "const result=a+b*10;\nfoo(a, b);\n",
    );
    check_with("cs", &off, "var x = a + b;\n", "var x=a+b;\n");
    check_with(
        "rs",
        &off,
        "fn f() {\n    let x = a << 2 | b;\n}\n",
        "fn f() {\n    let x=a<<2|b;\n}\n",
    );
    check_with("ps1", &off, "$total = $a + 1\n", "$total=$a + 1\n");
    // Arrows, `-x` and keywords are not affected.
    check_with(
        "ts",
        &off,
        "const f = (a) => -a;\nreturn -x;\n",
        "const f=(a) => -a;\nreturn -x;\n",
    );
}

#[test]
fn operators_that_would_run_together_keep_their_spaces() {
    let off = settings("[formatting]\nspace_around_operators = false\n");
    // Without the spaces these would read as `--`, a `//` comment, `>=` or `&&`.
    check_with(
        "js",
        &off,
        "y = a - -b;\nz = a / /re/.source;\nw = a + +b;\n",
        "y=a - -b;\nz=a / /re/.source;\nw=a + +b;\n",
    );
    check_with(
        "ts",
        &off,
        "let t: Map<string, number> = new Map();\n",
        "let t: Map<string, number> = new Map();\n",
    );
    check_with(
        "rs",
        &off,
        "fn f() {\n    let v: Vec<u8> = vec![];\n    let y = a & &b;\n    let z = a < -b;\n}\n",
        "fn f() {\n    let v: Vec<u8> = vec![];\n    let y=a & &b;\n    let z=a < -b;\n}\n",
    );
    check_with("cs", &off, "var r = a ? [1] : b;\n", "var r=a ? [1]:b;\n");
}

#[test]
fn operator_spacing_switches_back_and_forth() {
    let on = Config::default();
    let off = settings("[formatting]\nspace_around_operators = false\n");
    let cases = [
        ("js", "const result = a+b * 10;\nlet x = -1;\ny = a - -b;\nfor (let i = 0; i<n; i++) { s += i; }\nw = a ? b : c;\n"),
        ("ts", "let t: Map<string, number> = new Map();\ntype U = A | B;\n"),
        ("rs", "fn f() -> i32 {\n    let v: Vec<u8> = vec![];\n    let y = a & &b;\n    match v {\n        1 | 2 => 3,\n        _ => 4,\n    }\n}\n"),
        ("cs", "var total = price*count - discount;\nFunc<int, int> f = x => x + 1;\n"),
        ("ps1", "$x = $a + 1\nGet-Process | Select-Object Name\n"),
        ("toml", "a = 1\nb=2\n"),
    ];
    for (extension, source) in cases {
        switch_back_and_forth(extension, source, &on, &off);
    }
}

#[test]
fn operator_spacing_per_language() {
    let config = settings(
        "[formatting]\nspace_around_operators = false\n\n[formatting.javascript]\nspace_around_operators = true\n",
    );
    check_with("js", &config, "x=a+b;\n", "x = a + b;\n");
    check_with("ts", &config, "x=a+b;\n", "x=a+b;\n");
}

// ---------------------------------------------------------------------------------------
// final_newline and trim_trailing_whitespace

#[test]
fn final_newline_on_and_off() {
    check("rs", "fn main() {}", "fn main() {}\n");
    check("rs", "fn main() {}\n\n\n", "fn main() {}\n");
    let off = settings("[formatting]\nfinal_newline = false\n");
    check_with("rs", &off, "fn main() {}\n\n", "fn main() {}");
    check_with("toml", &off, "a = 1\n", "a = 1");
    // An empty file stays empty either way.
    check_with("rs", &off, "", "");
    check("rs", "\n\n", "");
}

#[test]
fn trailing_whitespace_is_removed_outside_literals() {
    let source = "fn main() {   \n    let s = \"a  \nb  \";  \n    // note   \n}   \n";
    let expected = "fn main() {\n    let s = \"a  \nb  \";\n    // note\n}\n";
    check("rs", source, expected);
    check(
        "rs",
        "/* one   \n   two */\nfn f() {}\n",
        "/* one\n   two */\nfn f() {}\n",
    );
}

#[test]
fn trailing_whitespace_can_be_kept() {
    let keep = settings("[formatting]\ntrim_trailing_whitespace = false\n");
    let source = "fn main() {   \n    let s = \"a  \nb  \";  \n    // note   \n}\n";
    check_with("rs", &keep, source, source);
}

#[test]
fn whitespace_inside_literals_never_changes() {
    for config in [
        Config::default(),
        settings("[formatting]\ntrim_trailing_whitespace = false\nindent_size = 2\n"),
    ] {
        let cases = [
            ("rs", "fn f() {\n    let a = r#\"  x  \n   y   \"#;\n}\n"),
            ("js", "const t = `  a  \n   b   `;\n"),
            ("ts", "const t = `${ a }  \n  b  `;\n"),
            ("cs", "class A\n{\n    string v = @\"  a  \n  b  \";\n}\n"),
            ("toml", "text = \"\"\"\n  a  \n  b  \"\"\"\n"),
            ("ps1", "$t = @\"\n  a  \n\"@\n"),
        ];
        for (extension, source) in cases {
            let formatted = format(extension, source, &config);
            let literal_lines: Vec<&str> =
                source.lines().filter(|line| line.contains("  ")).collect();
            for line in literal_lines {
                let inner = line.trim_start();
                assert!(
                    formatted.contains(inner) || formatted.contains(line),
                    "{}: lost `{}` in\n{}",
                    extension,
                    line,
                    formatted
                );
            }
        }
    }
}

#[test]
fn line_endings_are_applied_everywhere() {
    let crlf = Config {
        line_ending: LineEnding::Crlf,
        ..Config::default()
    };
    check_with(
        "rs",
        &crlf,
        "fn a() {\n    one();\n}\nfn b() {}\n",
        "fn a() {\r\n    one();\r\n}\r\n\r\nfn b() {}\r\n",
    );
    // CRLF input becomes LF with the default setting.
    check("toml", "a = 1\r\nb = 2\r\n", "a = 1\nb = 2\n");
    let crlf_no_final = settings("[formatting]\nline_ending = \"crlf\"\nfinal_newline = false\n");
    check_with("toml", &crlf_no_final, "a = 1\nb = 2\n", "a = 1\r\nb = 2");
}

#[test]
fn new_options_follow_language_tables() {
    let config = settings(
        "[formatting]\nfinal_newline = true\n\n[formatting.toml]\nfinal_newline = false\nblank_lines = false\n",
    );
    check_with("toml", &config, "a = 1\n[t]\nb = 2\n", "a = 1\n[t]\nb = 2");
    check_with("rs", &config, "fn a() {}", "fn a() {}\n");
}

// ---------------------------------------------------------------------------------------
// C# property patterns (regression: `{ }` was collapsed to `{}`)

#[test]
fn csharp_empty_property_patterns_keep_their_space() {
    let report = "if (channel.ParseReply(message) is not { } reply || channel.TryDeliver(reply))\n    return;\n";
    check("cs", report, report);
    check(
        "cs",
        "var result = value is { };\n",
        "var result = value is { };\n",
    );
    check(
        "cs",
        "var result = value is not { };\n",
        "var result = value is not { };\n",
    );
    let declaration = "if (value is not { } item)\n{\n    return;\n}\n";
    check(
        "cs",
        declaration,
        "if (value is not { } item) {\n    return;\n}\n",
    );
    let allman = settings("[formatting]\nbrace_style = \"allman\"\n");
    check_with("cs", &allman, declaration, declaration);
    // Written without the space, it stays without.
    check(
        "cs",
        "var result = value is {};\n",
        "var result = value is {};\n",
    );
}

#[test]
fn csharp_property_patterns_with_content() {
    let source = "if (value is { Name: \"Test\" })\n{\n    return;\n}\n";
    let allman = settings("[formatting]\nbrace_style = \"allman\"\n");
    check_with("cs", &allman, source, source);
    check(
        "cs",
        "var ok = value is {Name:\"Test\",Age:>18};\n",
        "var ok = value is { Name: \"Test\", Age: > 18 };\n",
    );
}

// ---------------------------------------------------------------------------------------
// Comments next to syntax boundaries

#[test]
fn comments_next_to_braces_stay_in_place() {
    let source = "\
function run() {  // starts here
    work();
    // last step
}  // end of run
";
    check("js", source, source);
    // A comment between `}` and `else` keeps `else` on its own line.
    let source = "if (a) {\n    one();\n}\n// otherwise\nelse {\n    two();\n}\n";
    check("js", source, source);
}

#[test]
fn comments_between_declarations_move_with_the_next_one() {
    let source =
        "fn a() {\n    one();\n}\n// About b.\n// More about b.\nfn b() {\n    two();\n}\n";
    let expected =
        "fn a() {\n    one();\n}\n\n// About b.\n// More about b.\nfn b() {\n    two();\n}\n";
    check("rs", source, expected);
}

#[test]
fn csharp_region_lines_are_not_treated_as_comments_about_the_next_member() {
    let source = "class A\n{\n    void One()\n    {\n    }\n    #endregion\n    void Two()\n    {\n    }\n}\n";
    let expected =
        "class A {\n    void One() {\n    }\n    #endregion\n\n    void Two() {\n    }\n}\n";
    check("cs", source, expected);
}

// ---------------------------------------------------------------------------
// continuation_indent
// ---------------------------------------------------------------------------

/// A PowerShell pipeline that goes on after a block, written the usual way.
const PIPELINE_USUAL: &str = "\
Get-WindowsImage -ImagePath $img.FullName | ForEach-Object {
    Get-WindowsImage -ImagePath $img.FullName -Index $_.ImageIndex
} | Select-Object ImageIndex, ImageName
";

/// The same pipeline with `continuation_indent = true`.
const PIPELINE_INDENTED: &str = "\
Get-WindowsImage -ImagePath $img.FullName | ForEach-Object {
        Get-WindowsImage -ImagePath $img.FullName -Index $_.ImageIndex
    } | Select-Object ImageIndex, ImageName
";

fn continuation_indent_on() -> Config {
    settings("[formatting.powershell]\ncontinuation_indent = true\n")
}

#[test]
fn continuation_indent_is_off_by_default() {
    check("ps1", PIPELINE_USUAL, PIPELINE_USUAL);
    check("ps1", PIPELINE_INDENTED, PIPELINE_USUAL);
}

#[test]
fn continuation_indent_pushes_in_a_block_in_the_middle_of_a_pipeline() {
    let on = continuation_indent_on();
    check_with("ps1", &on, PIPELINE_USUAL, PIPELINE_INDENTED);
    check_with("ps1", &on, PIPELINE_INDENTED, PIPELINE_INDENTED);
}

#[test]
fn continuation_indent_does_not_pile_up_for_several_blocks() {
    let source = "\
$r = $list | Where-Object {
$_ -gt 1
} | ForEach-Object {
$_ * 2
} | Sort-Object
";
    let expected = "\
$r = $list | Where-Object {
        $_ -gt 1
    } | ForEach-Object {
        $_ * 2
    } | Sort-Object
";
    check_with("ps1", &continuation_indent_on(), source, expected);
}

#[test]
fn continuation_indent_works_inside_other_blocks_and_after_parameters() {
    let source = "\
function Show-It {
    Get-A | ForEach-Object {
        if ($_) {
            Get-B | Where-Object {
                $_.Ok
            } | Select-Object N
        }
    } | Out-Null
}

Invoke-Command -ScriptBlock {
    Get-Date
} -ArgumentList 1
";
    let expected = "\
function Show-It {
    Get-A | ForEach-Object {
            if ($_) {
                Get-B | Where-Object {
                        $_.Ok
                    } | Select-Object N
            }
        } | Out-Null
}

Invoke-Command -ScriptBlock {
        Get-Date
    } -ArgumentList 1
";
    check_with("ps1", &continuation_indent_on(), source, expected);
}

#[test]
fn continuation_indent_leaves_everything_else_alone() {
    // Nothing follows the closing brace, the block is on one line, the pipeline is
    // already split one step per line, or the block sits inside other brackets.
    let source = "\
Get-Process | ForEach-Object {
    $_.Name
}
Get-Process | ForEach-Object { $_.Name } | Sort-Object
Get-A |
    ForEach-Object {
        $_
    } |
    Select-Object N
$h = @{ a = {
    1
}; b = 2 }
foo (bar | % {
    1
}) 2
if ($a) {
    Get-A
} else {
    Get-B
}
";
    check_with("ps1", &continuation_indent_on(), source, source);
    check("ps1", source, source);
}

#[test]
fn continuation_indent_does_nothing_in_other_languages() {
    let on = settings("[formatting]\ncontinuation_indent = true\n");
    let cases = [
        ("rs", "fn f() {\n    let v = items.iter().map(|x| {\n        x + 1\n    }).collect();\n}\n"),
        ("js", "const v = items.map((x) => {\n    return x + 1;\n}).filter(Boolean);\n"),
        ("cs", "class A\n{\n    void M()\n    {\n        var v = items.Select(x =>\n        {\n            return x + 1;\n        }).ToList();\n    }\n}\n"),
    ];
    for (extension, source) in cases {
        let usual = format(extension, source, &Config::default());
        assert_eq!(format(extension, source, &on), usual, "{}", extension);
    }
}

// ---------------------------------------------------------------------------
// PowerShell: a space before a script block that follows a command
// ---------------------------------------------------------------------------

#[test]
fn powershell_script_block_after_a_command_gets_a_space() {
    check(
        "ps1",
        "Get-A|ForEach-Object{ $_ }\nInvoke-Command -ScriptBlock{ 1 }\n",
        "Get-A | ForEach-Object { $_ }\nInvoke-Command -ScriptBlock { 1 }\n",
    );
    check(
        "ps1",
        "Get-A | Where-Object{$_.Ok} | ForEach-Object{\n$_\n}\n",
        "Get-A | Where-Object { $_.Ok } | ForEach-Object {\n    $_\n}\n",
    );
}

#[test]
fn powershell_braces_that_must_stay_attached_are_not_moved() {
    // A method call with a script block, a parameter value attached with `:`, a
    // hashtable and a variable in braces all mean something else with a space.
    let source =
        "$list.ForEach{ $_ }\nInvoke-Command -ScriptBlock:{ 1 }\n$h = @{ a = 1 }\n$v = ${my var}\n";
    check("ps1", source, source);
}

/// Runs the safety check on two PowerShell texts, the way the engine does.
fn powershell_means_the_same(before: &str, after: &str) -> bool {
    let languages = Languages::new();
    let adapter = languages.by_extension("ps1").unwrap();
    let original = adapter.grammar().parse(before).unwrap();
    let formatted = adapter.grammar().parse(after).unwrap();
    let is_atomic = |node: Node| adapter.compare_as_atomic(node);
    let text_matches = |_node: Node, left: &str, right: &str| left == right;
    let is_wrapper = |node: Node| adapter.is_plain_wrapper(node);
    autoformat_core::parser::compare_trees(
        &original,
        &formatted,
        &is_atomic,
        &text_matches,
        &is_wrapper,
    )
    .is_ok()
}

#[test]
fn the_safety_check_only_looks_through_a_lone_script_block() {
    // The one difference that is allowed: a space before the block.
    assert!(powershell_means_the_same("Get-A{ 1 }\n", "Get-A { 1 }\n"));
    assert!(powershell_means_the_same(
        "Get-A -P{ 1 }\n",
        "Get-A -P { 1 }\n"
    ));
    // Everything else is still caught.
    assert!(!powershell_means_the_same("Get-A{ 1 }\n", "Get-A { 2 }\n"));
    assert!(!powershell_means_the_same(
        "Get-A{ 1 }\n",
        "Get-A { 1 } { 1 }\n"
    ));
    assert!(!powershell_means_the_same(
        "Get-A { 1 }, { 2 }\n",
        "Get-A { 1 } { 2 }\n"
    ));
    assert!(!powershell_means_the_same(
        "Get-A{ 1 }\n",
        "Get-A -not { 1 }\n"
    ));
    assert!(!powershell_means_the_same("Get-A{ 1 }\n", "Get-A 1\n"));
}
