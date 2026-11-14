//! Tests for the `brace_style` option in every language that has code blocks.
//!
//! Each check formats the input, compares it with the expected text, and formats the
//! result again to prove that it does not change any more (idempotency).

use autoformat_core::config::{BraceStyle, Config};
use autoformat_core::format_source;
use autoformat_languages::Languages;

fn config(style: BraceStyle) -> Config {
    Config {
        brace_style: style,
        ..Config::default()
    }
}

/// For examples written without empty lines between declarations.
fn compact(style: BraceStyle) -> Config {
    Config {
        brace_style: style,
        blank_lines: false,
        ..Config::default()
    }
}

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

/// Formats, compares with `expected`, and checks idempotency.
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

fn check(extension: &str, style: BraceStyle, source: &str, expected: &str) {
    check_with(extension, &config(style), source, expected);
}

/// Every style must leave this text exactly as it is.
fn check_unchanged_in_every_style(extension: &str, source: &str) {
    for style in [BraceStyle::SameLine, BraceStyle::Allman, BraceStyle::Gnu] {
        check(extension, style, source, source);
    }
}

// ---------------------------------------------------------------------------------------
// The examples from the specification.

#[test]
fn spec_example_same_line() {
    check(
        "cs",
        BraceStyle::SameLine,
        "public void DoSomething()\n{\n    Console.WriteLine(\"Hello\");\n}\n",
        "public void DoSomething() {\n    Console.WriteLine(\"Hello\");\n}\n",
    );
}

#[test]
fn spec_example_allman() {
    check(
        "cs",
        BraceStyle::Allman,
        "public void DoSomething() {\n    Console.WriteLine(\"Hello\");\n}\n",
        "public void DoSomething()\n{\n    Console.WriteLine(\"Hello\");\n}\n",
    );
}

#[test]
fn spec_example_gnu() {
    let gnu = Config {
        brace_style: BraceStyle::Gnu,
        indent_size: 2,
        ..Config::default()
    };
    check_with(
        "cs",
        &gnu,
        "void DoSomething() {\n    printf(\"Hello\");\n}\n",
        "void DoSomething()\n  {\n    printf(\"Hello\");\n  }\n",
    );
}

// ---------------------------------------------------------------------------------------
// C#

const CSHARP_SAME_LINE: &str = "\
namespace Shop {
    public class Cart : IDisposable {
        public int Count {
            get { return items.Count; }
        }

        public void Add(Item item) {
            if (item == null) {
                throw new ArgumentNullException(nameof(item));
            } else if (item.Price < 0) {
                return;
            } else {
                items.Add(item);
            }
            foreach (var other in items) {
                Check(other);
            }
            try {
                Save();
            } catch (IOException error) {
                Log(error);
            } finally {
                Close();
            }
            do {
                Retry();
            } while (busy);
            switch (item.Kind) {
                case 1:
                    break;
            }
        }

        public void Empty() {
        }
    }
}
";

const CSHARP_ALLMAN: &str = "\
namespace Shop
{
    public class Cart : IDisposable
    {
        public int Count
        {
            get { return items.Count; }
        }

        public void Add(Item item)
        {
            if (item == null)
            {
                throw new ArgumentNullException(nameof(item));
            }
            else if (item.Price < 0)
            {
                return;
            }
            else
            {
                items.Add(item);
            }
            foreach (var other in items)
            {
                Check(other);
            }
            try
            {
                Save();
            }
            catch (IOException error)
            {
                Log(error);
            }
            finally
            {
                Close();
            }
            do
            {
                Retry();
            }
            while (busy);
            switch (item.Kind)
            {
                case 1:
                    break;
            }
        }

        public void Empty()
        {
        }
    }
}
";

const CSHARP_GNU: &str = "\
namespace Shop
  {
    public class Cart : IDisposable
      {
        public int Count
          {
            get { return items.Count; }
          }

        public void Add(Item item)
          {
            if (item == null)
              {
                throw new ArgumentNullException(nameof(item));
              }
            else if (item.Price < 0)
              {
                return;
              }
            else
              {
                items.Add(item);
              }
            foreach (var other in items)
              {
                Check(other);
              }
            try
              {
                Save();
              }
            catch (IOException error)
              {
                Log(error);
              }
            finally
              {
                Close();
              }
            do
              {
                Retry();
              }
            while (busy);
            switch (item.Kind)
              {
                case 1:
                  break;
              }
          }

        public void Empty()
          {
          }
      }
  }
";

fn gnu_with_two_spaces() -> Config {
    Config {
        brace_style: BraceStyle::Gnu,
        indent_size: 2,
        ..Config::default()
    }
}

#[test]
fn csharp_converts_between_all_styles() {
    // Every style can be reached from every other style.
    for source in [CSHARP_SAME_LINE, CSHARP_ALLMAN] {
        check("cs", BraceStyle::SameLine, source, CSHARP_SAME_LINE);
        check("cs", BraceStyle::Allman, source, CSHARP_ALLMAN);
    }
    check(
        "cs",
        BraceStyle::SameLine,
        &format("cs", CSHARP_ALLMAN, &config(BraceStyle::Gnu)),
        CSHARP_SAME_LINE,
    );
    check(
        "cs",
        BraceStyle::Allman,
        &format("cs", CSHARP_SAME_LINE, &config(BraceStyle::Gnu)),
        CSHARP_ALLMAN,
    );
}

#[test]
fn csharp_gnu_indents_braces_one_level() {
    // With indent_size 2, GNU braces sit 2 spaces in and their contents 4 spaces in.
    check_with("cs", &gnu_with_two_spaces(), CSHARP_SAME_LINE, CSHARP_GNU);
    check_with("cs", &gnu_with_two_spaces(), CSHARP_ALLMAN, CSHARP_GNU);
    // With the default indent_size 4 the extra level is 4 spaces.
    let four = format(
        "cs",
        "void F() {\n    Run();\n}\n",
        &config(BraceStyle::Gnu),
    );
    assert_eq!(four, "void F()\n    {\n        Run();\n    }\n");
}

#[test]
fn csharp_braces_that_are_not_code_blocks_never_move() {
    let source = "\
class A {
    int[] numbers = new[] { 1, 2, 3 };
    Point point = new Point {
        X = 1,
        Y = 2
    };
    List<int> list = new List<int>
    {
        1,
        2
    };
    string text = $\"{name} has {count:N0} {{items}}\";
    string Name { get; set; }
    Func<int, int> twice = x =>
    {
        return x * 2;
    };
    string Describe(int x) => x switch
    {
        1 => \"one\",
        _ => \"many\"
    };
}
";
    // Only the `{` of `class A` is a code block. Everything else stays put.
    let allman = source.replacen("class A {\n", "class A\n{\n", 1);
    check_with("cs", &compact(BraceStyle::SameLine), source, source);
    check_with("cs", &compact(BraceStyle::Allman), source, &allman);
}

#[test]
fn csharp_single_line_blocks_stay_on_one_line() {
    // An empty block keeps its spacing: `{ }` stays `{ }` and `{}` stays `{}`.
    let source =
        "class A {\n    void F() { Run(); }\n    int G() { return 1; }\n    void Empty() { }\n    void Tight() {}\n}\n";
    let allman =
        "class A\n{\n    void F() { Run(); }\n    int G() { return 1; }\n    void Empty() { }\n    void Tight() {}\n}\n";
    check("cs", BraceStyle::Allman, source, allman);
    check("cs", BraceStyle::SameLine, allman, source);
}

#[test]
fn csharp_comments_around_braces() {
    // A comment between the header and the `{` keeps the brace on its own line.
    let source =
        "void F()  // explains F\n{\n    Run();\n}\n\nvoid G()\n// explains G\n{\n    Run();\n}\n";
    check("cs", BraceStyle::SameLine, source, source);

    // A comment after `{` stays with the brace.
    let source = "void F() { // starts here\n    Run(); // runs\n    // last line\n}\n";
    let expected = "void F()\n{  // starts here\n    Run();  // runs\n    // last line\n}\n";
    check("cs", BraceStyle::Allman, source, expected);
}

#[test]
fn a_closed_comment_before_a_brace_can_be_changed_back() {
    // `/* note */` behind code: the brace joins the line again in same_line style.
    let samples = [
        (
            "cs",
            "class A /* note */ {\n    void M() {\n        if (x) {\n            Y();\n        } else /* why */ {\n            Z();\n        }\n    }\n}\n",
            "class A  /* note */\n{\n    void M()\n    {\n        if (x)\n        {\n            Y();\n        }\n        else  /* why */\n        {\n            Z();\n        }\n    }\n}\n",
        ),
        (
            "java",
            "class A /* note */ {\n    void run() /* why */ {\n        go();\n    }\n}\n",
            "class A  /* note */\n{\n    void run()  /* why */\n    {\n        go();\n    }\n}\n",
        ),
        (
            "rs",
            "fn main() /* a */ {\n    if x /* b */ {\n        y();\n    }\n}\n",
            "fn main()  /* a */\n{\n    if x  /* b */\n    {\n        y();\n    }\n}\n",
        ),
        (
            "ts",
            "function f() /* a */ {\n    while (x) /* b */ {\n        y();\n    }\n}\n",
            "function f()  /* a */\n{\n    while (x)  /* b */\n    {\n        y();\n    }\n}\n",
        ),
        (
            "ps1",
            "function F {\n    if ($x) <# b #> {\n        1\n    }\n}\n",
            "function F\n{\n    if ($x)  <# b #>\n    {\n        1\n    }\n}\n",
        ),
    ];
    for (extension, same_line, allman) in samples {
        check(extension, BraceStyle::Allman, same_line, allman);
        check(extension, BraceStyle::SameLine, allman, same_line);
        let gnu = format(extension, same_line, &config(BraceStyle::Gnu));
        check(extension, BraceStyle::SameLine, &gnu, same_line);
        check(extension, BraceStyle::Allman, &gnu, allman);
    }
}

#[test]
fn other_comments_before_a_brace_keep_it_on_its_own_line() {
    // After `// note` a joined brace would become part of the comment, and a comment on
    // a line of its own stays the last thing on its line.
    let source = "while (x)  // note\n{\n    Go();\n}\nfor (;;)\n/* own line */\n{\n    Go();\n}\n";
    check("cs", BraceStyle::SameLine, source, source);
    check("cs", BraceStyle::Allman, source, source);
}

#[test]
fn csharp_comment_alignment_after_brace_moves() {
    let source = "void F()\n{\n    int a = 1; // first\n    if (a > 0)\n    { // second\n        Run();\n    }\n}\n";
    let expected = "void F() {\n    int a = 1;    // first\n    if (a > 0) {  // second\n        Run();\n    }\n}\n";
    check("cs", BraceStyle::SameLine, source, expected);
}

#[test]
fn csharp_multi_line_declarations() {
    // The declaration continues on a second line (`: base(...)`), so the `{` joins that
    // line in same_line style, and the body is indented from the declaration.
    let allman = "\
class Order : Base
{
    public Order(string customer, decimal total)
        : base(customer)
    {
        Total = total;
    }
}
";
    let same_line = "\
class Order : Base {
    public Order(string customer, decimal total)
        : base(customer) {
        Total = total;
    }
}
";
    check("cs", BraceStyle::SameLine, allman, same_line);
    check("cs", BraceStyle::Allman, same_line, allman);
}

#[test]
fn csharp_brace_moves_work_with_column_width() {
    let narrow = Config {
        brace_style: BraceStyle::SameLine,
        column_width: 36,
        ..Config::default()
    };
    // `void Compute(int first, int second)` fits in 36 columns, but joining the brace
    // makes it 37 wide, so the parameters are wrapped.
    let source = "void Compute(int first, int second)\n{\n    Run();\n}\n";
    let expected = "void Compute(\n    int first,\n    int second\n) {\n    Run();\n}\n";
    check_with("cs", &narrow, source, expected);

    // Lists in braces are never wrapped (so that changing column_width can always be
    // undone), and a one-line block keeps its braces where they are.
    let narrow_allman = Config {
        brace_style: BraceStyle::Allman,
        column_width: 40,
        ..Config::default()
    };
    let source = "enum Colors { Red, Green, Blue, Cyan, Magenta, Yellow }\n";
    check_with("cs", &narrow_allman, source, source);
}

// ---------------------------------------------------------------------------------------
// JavaScript and TypeScript

const JS_SAME_LINE: &str = "\
class Store extends Base {
    load(id) {
        if (!id) {
            return null;
        } else {
            return this.items[id];
        }
    }
}

function run(options) {
    try {
        start(options);
    } catch (error) {
        report(error);
    } finally {
        stop();
    }
    for (const item of list) {
        handle(item);
    }
}
";

const JS_ALLMAN: &str = "\
class Store extends Base
{
    load(id)
    {
        if (!id)
        {
            return null;
        }
        else
        {
            return this.items[id];
        }
    }
}

function run(options)
{
    try
    {
        start(options);
    }
    catch (error)
    {
        report(error);
    }
    finally
    {
        stop();
    }
    for (const item of list)
    {
        handle(item);
    }
}
";

#[test]
fn javascript_converts_between_styles() {
    check("js", BraceStyle::Allman, JS_SAME_LINE, JS_ALLMAN);
    check("js", BraceStyle::SameLine, JS_ALLMAN, JS_SAME_LINE);
    let gnu = format("js", JS_SAME_LINE, &config(BraceStyle::Gnu));
    assert!(
        gnu.starts_with("class Store extends Base\n    {\n        load(id)\n            {\n"),
        "{}",
        gnu
    );
    check("js", BraceStyle::SameLine, &gnu, JS_SAME_LINE);
}

#[test]
fn javascript_object_literals_destructuring_and_arrows_never_move() {
    let source = "\
const settings = {
    name: 'demo',
    nested: { deep: true }
};
const { name, nested: { deep } } = settings;
const handler = (event) => {
    return event.target;
};
items.forEach(function (item) {
    use(item);
});
const text = `value: ${ { a: 1 }.a }`;
";
    check_unchanged_in_every_style("js", source);
}

#[test]
fn typescript_declarations_follow_the_style() {
    let source = "\
interface Shape {
    area(): number;
}
enum Kind {
    Circle,
    Square
}
abstract class Base<T> implements Shape {
    abstract area(): number;
}
type Point = {
    x: number;
    y: number;
};
";
    let expected = "\
interface Shape
{
    area(): number;
}
enum Kind
{
    Circle,
    Square
}
abstract class Base<T> implements Shape
{
    abstract area(): number;
}
type Point = {
    x: number;
    y: number;
};
";
    check_with("ts", &compact(BraceStyle::Allman), source, expected);
    check_with("ts", &compact(BraceStyle::SameLine), expected, source);
}

#[test]
fn typescript_namespaces_keep_their_braces() {
    // `namespace X` with `{` on the next line means something else to the grammar.
    check_unchanged_in_every_style(
        "ts",
        "namespace Geometry {\n    export const unit = 1;\n}\n",
    );
}

#[test]
fn tsx_markup_is_untouched() {
    let source =
        "function App() {\n    return <div style={{ color: 'red' }}>{items.map((x) => { return x; })}</div>;\n}\n";
    let expected =
        "function App()\n{\n    return <div style={{ color: 'red' }}>{items.map((x) => { return x; })}</div>;\n}\n";
    check("tsx", BraceStyle::Allman, source, expected);
}

// ---------------------------------------------------------------------------------------
// Rust

const RUST_SAME_LINE: &str = "\
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn sign(&self) -> i32 {
        if self.x > 0 {
            1
        } else if self.x < 0 {
            -1
        } else {
            0
        }
    }

    fn walk(&self) {
        for step in 0..self.x {
            println!(\"{}\", step);
        }
        match self.y {
            0 => {}
            _ => {
                loop {
                    break;
                }
            }
        }
    }
}
";

const RUST_ALLMAN: &str = "\
struct Point
{
    x: i32,
    y: i32,
}

impl Point
{
    fn sign(&self) -> i32
    {
        if self.x > 0
        {
            1
        }
        else if self.x < 0
        {
            -1
        }
        else
        {
            0
        }
    }

    fn walk(&self)
    {
        for step in 0..self.x
        {
            println!(\"{}\", step);
        }
        match self.y
        {
            0 => {}
            _ => {
                loop
                {
                    break;
                }
            }
        }
    }
}
";

#[test]
fn rust_converts_between_styles() {
    check("rs", BraceStyle::Allman, RUST_SAME_LINE, RUST_ALLMAN);
    check("rs", BraceStyle::SameLine, RUST_ALLMAN, RUST_SAME_LINE);
    let gnu = format("rs", RUST_SAME_LINE, &config(BraceStyle::Gnu));
    check("rs", BraceStyle::SameLine, &gnu, RUST_SAME_LINE);
}

#[test]
fn rust_macros_struct_literals_closures_and_values_never_move() {
    let source = "\
macro_rules! square {
    ($x:expr) => {
        $x * $x
    };
}

fn main() {
    let point = Point {
        x: 1,
        y: 2,
    };
    let values = vec![
        1,
        2,
    ];
    let double = |x: i32| {
        x * 2
    };
    let sign = if point.x > 0 {
        1
    } else {
        -1
    };
    let name = match sign {
        1 => \"positive\",
        _ => \"other\",
    };
    let text = format!(\"{{}} {}\", name);
    let _ = unsafe {
        read()
    };
}
";
    let allman = source.replacen("fn main() {\n", "fn main()\n{\n", 1);
    check("rs", BraceStyle::SameLine, source, source);
    check("rs", BraceStyle::Allman, source, &allman);
}

#[test]
fn rust_brace_after_where_clause_stays_on_its_own_line() {
    let source =
        "fn show<T>(value: T)\nwhere\n    T: Display,\n{\n    println!(\"{}\", value);\n}\n";
    check_unchanged_in_every_style("rs", source);
}

// ---------------------------------------------------------------------------------------
// PowerShell

const POWERSHELL_SAME_LINE: &str = "\
function Get-Size {
    param($Path)
    if (Test-Path $Path) {
        $item = Get-Item $Path
    } elseif ($Path -eq '') {
        return 0
    } else {
        throw 'missing'
    }
    try {
        $item.Length
    } catch {
        0
    } finally {
        Write-Verbose 'done'
    }
    foreach ($file in $files) {
        $file.Name
    }
}
";

const POWERSHELL_ALLMAN: &str = "\
function Get-Size
{
    param($Path)
    if (Test-Path $Path)
    {
        $item = Get-Item $Path
    }
    elseif ($Path -eq '')
    {
        return 0
    }
    else
    {
        throw 'missing'
    }
    try
    {
        $item.Length
    }
    catch
    {
        0
    }
    finally
    {
        Write-Verbose 'done'
    }
    foreach ($file in $files)
    {
        $file.Name
    }
}
";

#[test]
fn powershell_converts_between_styles() {
    check(
        "ps1",
        BraceStyle::Allman,
        POWERSHELL_SAME_LINE,
        POWERSHELL_ALLMAN,
    );
    check(
        "ps1",
        BraceStyle::SameLine,
        POWERSHELL_ALLMAN,
        POWERSHELL_SAME_LINE,
    );
    let gnu = format("ps1", POWERSHELL_SAME_LINE, &config(BraceStyle::Gnu));
    check("ps1", BraceStyle::SameLine, &gnu, POWERSHELL_SAME_LINE);
}

#[test]
fn powershell_script_blocks_and_hashtables_never_move() {
    let source = "\
Get-ChildItem | Where-Object {
    $_.Length -gt 0
} | ForEach-Object {
    $_.Name
}
$settings = @{
    Name = 'demo'
    Size = 3
}
$action = {
    param($x)
    $x * 2
}
$text = \"braces { in } a string\"
";
    check_unchanged_in_every_style("ps1", source);
}

// ---------------------------------------------------------------------------------------
// TOML

#[test]
fn toml_inline_tables_never_move() {
    let source = "[dependencies]\nserde = { version = \"1.0\", features = [\"derive\"] }\npoint = { x = 1, y = 2 }\n";
    check_unchanged_in_every_style("toml", source);
}

// ---------------------------------------------------------------------------------------
// Configuration: language overrides and inheritance

#[test]
fn language_override_beats_the_global_style() {
    let config = autoformat_core::config::parse_config(
        "[formatting]\nbrace_style = \"allman\"\n\n[formatting.csharp]\nbrace_style = \"same_line\"\n",
    )
    .unwrap();
    // C# uses its own setting.
    check_with(
        "cs",
        &config,
        "void F()\n{\n    Run();\n}\n",
        "void F() {\n    Run();\n}\n",
    );
    // JavaScript inherits the global setting.
    check_with(
        "js",
        &config,
        "function f() {\n    run();\n}\n",
        "function f()\n{\n    run();\n}\n",
    );
}

#[test]
fn tsx_files_use_the_typescript_override() {
    let config = autoformat_core::config::parse_config(
        "[formatting.typescript]\nbrace_style = \"allman\"\n",
    )
    .unwrap();
    check_with(
        "tsx",
        &config,
        "function f() {\n    run();\n}\n",
        "function f()\n{\n    run();\n}\n",
    );
    check_with(
        "ts",
        &config,
        "function f() {\n    run();\n}\n",
        "function f()\n{\n    run();\n}\n",
    );
    // Other languages keep the default.
    check_with(
        "js",
        &config,
        "function f()\n{\n    run();\n}\n",
        "function f() {\n    run();\n}\n",
    );
}

// ---------------------------------------------------------------------------------------
// Java.

const JAVA_SAME_LINE: &str = "\
public class Orders {
    public void process(Order item) {
        if (item == null) {
            return;
        } else if (item.isEmpty()) {
            skip();
        } else {
            run(item);
        }
        for (Order other : items) {
            check(other);
        }
        try {
            save();
        } catch (IOException error) {
            log(error);
        } finally {
            close();
        }
        do {
            retry();
        } while (busy);
        switch (item.kind()) {
            case 1 -> {
                one();
            }
            default -> two();
        }
    }

    public void empty() {
    }
}
";

const JAVA_ALLMAN: &str = "\
public class Orders
{
    public void process(Order item)
    {
        if (item == null)
        {
            return;
        }
        else if (item.isEmpty())
        {
            skip();
        }
        else
        {
            run(item);
        }
        for (Order other : items)
        {
            check(other);
        }
        try
        {
            save();
        }
        catch (IOException error)
        {
            log(error);
        }
        finally
        {
            close();
        }
        do
        {
            retry();
        }
        while (busy);
        switch (item.kind())
        {
            case 1 ->
            {
                one();
            }
            default -> two();
        }
    }

    public void empty()
    {
    }
}
";

#[test]
fn java_converts_between_all_styles() {
    // Every style can be reached from every other style.
    for source in [JAVA_SAME_LINE, JAVA_ALLMAN] {
        check("java", BraceStyle::SameLine, source, JAVA_SAME_LINE);
        check("java", BraceStyle::Allman, source, JAVA_ALLMAN);
    }
    let gnu = format("java", JAVA_SAME_LINE, &config(BraceStyle::Gnu));
    assert_eq!(gnu, format("java", JAVA_ALLMAN, &config(BraceStyle::Gnu)));
    check("java", BraceStyle::SameLine, &gnu, JAVA_SAME_LINE);
    check("java", BraceStyle::Allman, &gnu, JAVA_ALLMAN);
}

#[test]
fn java_gnu_indents_braces_one_level() {
    check_with(
        "java",
        &gnu_with_two_spaces(),
        "class A {\n  void run(int x) {\n    if (x > 0) {\n      go();\n    } else {\n      stop();\n    }\n  }\n}\n",
        "class A\n  {\n    void run(int x)\n      {\n        if (x > 0)\n          {\n            go();\n          }\n        else\n          {\n            stop();\n          }\n      }\n  }\n",
    );
}

#[test]
fn java_braces_that_are_not_code_blocks_never_move() {
    // Array initializers, lambdas and anonymous classes.
    let inside = "    int[] sizes = {\n        1, 2,\n    };\n\n    Runnable r = () -> {\n        go();\n    };\n\n    Object o = new Object() {\n    };\n";
    let same_line = format!("class A {{\n{}}}\n", inside);
    let allman = format!("class A\n{{\n{}}}\n", inside);
    check("java", BraceStyle::SameLine, &same_line, &same_line);
    check("java", BraceStyle::SameLine, &allman, &same_line);
    check("java", BraceStyle::Allman, &allman, &allman);
    check("java", BraceStyle::Allman, &same_line, &allman);
}

// ---------------------------------------------------------------------------------------
// Kotlin.

const KOTLIN_SAME_LINE: &str = "\
class Orders(val items: List<Order>) : Service() {
    val size: Int
        get() {
            return items.size
        }

    init {
        check(items.isNotEmpty())
    }

    fun process(item: Order?) {
        if (item == null) {
            return
        } else if (item.isEmpty()) {
            skip()
        } else {
            run(item)
        }
        for (other in items) {
            check(other)
        }
        try {
            save()
        } catch (error: IOException) {
            log(error)
        } finally {
            close()
        }
        do {
            retry()
        } while (busy)
        when (item.kind) {
            1 -> {
                one()
            }
            else -> two()
        }
    }

    companion object {
        const val MAX = 10
    }
}
";

const KOTLIN_ALLMAN: &str = "\
class Orders(val items: List<Order>) : Service()
{
    val size: Int
        get()
        {
            return items.size
        }

    init
    {
        check(items.isNotEmpty())
    }

    fun process(item: Order?)
    {
        if (item == null)
        {
            return
        }
        else if (item.isEmpty())
        {
            skip()
        }
        else
        {
            run(item)
        }
        for (other in items)
        {
            check(other)
        }
        try
        {
            save()
        }
        catch (error: IOException)
        {
            log(error)
        }
        finally
        {
            close()
        }
        do
        {
            retry()
        }
        while (busy)
        when (item.kind)
        {
            1 ->
            {
                one()
            }
            else -> two()
        }
    }

    companion object
    {
        const val MAX = 10
    }
}
";

#[test]
fn kotlin_block_braces_follow_brace_style() {
    for source in [KOTLIN_SAME_LINE, KOTLIN_ALLMAN] {
        check("kt", BraceStyle::SameLine, source, KOTLIN_SAME_LINE);
        check("kt", BraceStyle::Allman, source, KOTLIN_ALLMAN);
    }
    let gnu = format("kt", KOTLIN_SAME_LINE, &config(BraceStyle::Gnu));
    assert_eq!(gnu, format("kt", KOTLIN_ALLMAN, &config(BraceStyle::Gnu)));
    check("kt", BraceStyle::SameLine, &gnu, KOTLIN_SAME_LINE);
    check("kt", BraceStyle::Allman, &gnu, KOTLIN_ALLMAN);
}

#[test]
fn kotlin_lambda_braces_never_move() {
    // Kotlin has no semicolons: `run {` and `run` + line break + `{` do not mean the
    // same. Lambdas, `object : Type {` and anonymous functions stay as written.
    let inside = "    items.forEach {\n        use(it)\n    }\n    val job = launch(context) { value ->\n        work(value)\n    }\n    val listener = object : Listener {\n    }\n    val action = fun(x: Int): Int {\n        return x\n    }\n    start()\n        {\n            later()\n        }\n";
    let same_line = format!("fun run() {{\n{}}}\n", inside);
    let allman = format!("fun run()\n{{\n{}}}\n", inside);
    check("kt", BraceStyle::SameLine, &same_line, &same_line);
    check("kt", BraceStyle::SameLine, &allman, &same_line);
    check("kt", BraceStyle::Allman, &allman, &allman);
    check("kt", BraceStyle::Allman, &same_line, &allman);
}

// ---------------------------------------------------------------------------------------
// GNU style with exact expected output for the remaining languages (indent_size 2).

#[test]
fn gnu_exact_output_for_typescript_rust_and_powershell() {
    let gnu = gnu_with_two_spaces();
    check_with(
        "ts",
        &gnu,
        "class A {\n  run(x: number): void {\n    if (x > 0) {\n      go();\n    } else {\n      stop();\n    }\n  }\n}\n",
        "class A\n  {\n    run(x: number): void\n      {\n        if (x > 0)\n          {\n            go();\n          }\n        else\n          {\n            stop();\n          }\n      }\n  }\n",
    );
    check_with(
        "rs",
        &gnu,
        "fn main() {\n  if ready {\n    go();\n  } else {\n    stop();\n  }\n}\n",
        "fn main()\n  {\n    if ready\n      {\n        go();\n      }\n    else\n      {\n        stop();\n      }\n  }\n",
    );
    check_with(
        "ps1",
        &gnu,
        "function Test-It {\n  if ($a) {\n    'yes'\n  } else {\n    'no'\n  }\n}\n",
        "function Test-It\n  {\n    if ($a)\n      {\n        'yes'\n      }\n    else\n      {\n        'no'\n      }\n  }\n",
    );
    check_with(
        "js",
        &gnu,
        "function f() {\n  while (busy) {\n    wait();\n  }\n}\n",
        "function f()\n  {\n    while (busy)\n      {\n        wait();\n      }\n  }\n",
    );
}
