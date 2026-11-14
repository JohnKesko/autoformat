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
        "json",
        "javascript",
        "typescript",
        "csharp",
        "java",
        "kotlin",
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
            "continuation_indent false <-> true",
            base.clone(),
            Config {
                continuation_indent: true,
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
    ("k.json", "{ \"name\": \"demo\", \"tags\": [\"alpha\", \"beta\", \"gamma\", \"delta\", \"epsilon\", \"zeta\"], \"point\": { \"x\": 1, \"y\": 2 } }\n"),
    ("l.json", "[\n    [1, 0, 0],\n    [0, 1, 0],\n    { \"rows\": [\"first\", \"second\", \"third\", \"fourth\", \"fifth\"] }\n]\n"),
    ("m.jsonc", "{\n    // sizes\n    \"sizes\": [10, 20, 30, 40, 50, 60, 70, 80, 90, 100], // all of them\n    \"names\": [\n        \"a\", // first\n        \"b\"\n    ]\n}\n"),
    ("n.java", "class A {\n    <T> Map<String, List<T>> run(Map<String, List<Map<String, Integer>>> first, List<? extends Number> second, T... rest) throws IOException { go(first, second); }\n}\n"),
    ("o.java", "@Entity\n@Table(name = \"orders\", indexes = {@Index(columnList = \"customer\"), @Index(columnList = \"created\")})\npublic class Order extends Base<Order>\n    implements Comparable<Order>, Serializable {\n    @Id\n    @Column(name = \"id\")\n    private long id;\n    int[] sizes = {10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160, 170, 180};\n}\n"),
    ("p.java", "class A {\n    int run(int k) {\n        outer:\n        for (String s : names) {\n            if (s == null) continue outer;\n            else if (s.isEmpty()) { break; }\n        }\n        try (InputStream in = open()) {\n            in.read();\n        } catch (IOException | RuntimeException e) {\n            log(e);\n        } finally {\n            done();\n        }\n        switch (k) {\n            case 1:\n            case 2:\n                return 1;\n            default:\n                break;\n        }\n        Runnable r = new Runnable() {\n            @Override\n            public void run() {\n                items.forEach(item -> {\n                    use(item, String::valueOf, (int) k + 1);\n                });\n            }\n        };\n        return switch (k) {\n            case 3 -> {\n                yield k > 0 ? k : -k;\n            }\n            default -> list.stream().map(String::valueOf).filter(v -> !v.isEmpty()).mapToInt(String::length).sum();\n        };\n    }\n}\n"),
    ("q.java", "enum Op {\n    ADD(\"+\") {\n        @Override int apply(int a, int b) { return a + b; }\n    },\n    SUB(\"-\") {\n        @Override int apply(int a, int b) { return a - b; }\n    };\n    String text = \"\"\"\n          keep   this\n        \"\"\";\n    record Point(int x, int y) implements Comparable<Point> {\n        Point {\n            if (x < 0) throw new IllegalArgumentException();\n        }\n    }\n}\n"),
    ("r.cs", "class A\n{\n    void Run()\n    {\n        var sizes = new[] { 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160, 170, 180, 190 };\n        var names = new List<string> {\n            \"alpha\",\n            \"beta\"\n        };\n        var kept = new[]\n        {\n            1,\n            2\n        };\n        var item = new Item { Name = \"a\", Tags = new[] { \"x\", \"y\" }, Sizes = { 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14 } };\n        int[,] grid = { { 1, 0, 0 }, { 0, 1, 0 }, { 0, 0, 1 } };\n    }\n}\n"),
    ("s.java", "class A {\n    int[][] grid = {{1, 0, 0}, {0, 1, 0}, {0, 0, 1}};\n    String[] names = {\n        \"alpha\",\n        \"beta\"\n    };\n    @Test(tags = {\"fast\", \"unit\", \"core\", \"smoke\"}, groups = {1, 2, 3})\n    void run() {\n        use(new int[] {10, 20, 30, 40, 50, 60, 70, 80, 90, 100}, new String[] {\"a\", \"b\"});\n    }\n}\n"),
    ("r.kt", "class A {\n    fun <T : Any> run(first: Map<String, List<Map<String, Int>>>, second: List<out Number>, vararg rest: T, block: (T, Int) -> Unit = { _, _ -> }): Map<String?, T>? = null\n}\n"),
    ("s.kt", "@file:JvmName(\"Orders\")\npackage shop\n\n@Entity\n@Table(name = \"orders\", indexes = [Index(columnList = \"customer\"), Index(columnList = \"created\")])\ndata class Order(val id: Long, val customer: String, val created: Instant = Instant.now(), val lines: List<Line> = emptyList()) : Base<Order>(),\n    Comparable<Order> {\n    @get:Rule\n    val rule = Rule()\n    var total: Double = 0.0\n        get() = field + 1\n        private set(value) {\n            field = value\n        }\n    val sizes = intArrayOf(10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 160, 170, 180, 190, 200)\n}\n"),
    ("t.kt", "fun run(k: Int): Int {\n    outer@ for ((key, value) in entries) {\n        for (item in value)\n            send(item)\n        if (key == null) continue@outer\n        else if (key.isEmpty()) { break }\n    }\n    try {\n        input.read()\n    } catch (error: IOException) {\n        log(error)\n    } finally {\n        done()\n    }\n    val listener = object : Listener {\n        override fun done() {\n            items.forEach { item ->\n                use(item, String::length, k as? Long ?: -1L)\n            }\n        }\n    }\n    start()\n    { later() }\n    val total = first\n    + second\n    val names = users\n        .filter { it.active && it.name != null }\n        .map { it.name!! }\n        ?: emptyList()\n    return when (k) {\n        1, 2 -> {\n            if (k > 0) k else -k\n        }\n        in 3..9,\n        !in 10..<99 -> list.map(String::valueOf).filter { v -> !v.isEmpty() }.sumOf { it.length }\n        else -> 0\n    }\n}\n"),
    ("u.kt", "enum class Op(val sign: String) {\n    ADD(\"+\") {\n        override fun apply(a: Int, b: Int) = a + b\n    },\n    SUB(\"-\") {\n        override fun apply(a: Int, b: Int) = a - b\n    };\n    val text = \"\"\"\n          keep   this $sign ${ sign.length }\n        \"\"\"\n    abstract fun apply(a: Int, b: Int): Int\n    companion object {\n        fun of(sign: String) = entries.first { it.sign == sign }\n    }\n}\n"),
    ("v.kts", "plugins {\n    kotlin(\"jvm\") version \"2.0.0\"\n}\nval names = listOf(\"alpha\", \"beta\", \"gamma\", \"delta\", \"epsilon\", \"zeta\", \"eta\", \"theta\", \"iota\", \"kappa\", \"lambda\", \"mu\")\ntasks.register(\"hello\") {\n    doLast { println(names.joinToString()) }\n}\n"),
    ("j.ps1", "function Show-It {\n    Get-A | ForEach-Object {\n        Get-B | Where-Object {\n            $_.Ok\n        } | Select-Object N\n    } | Where-Object {\n        $_\n    } | Out-Null\n    Get-C | ForEach-Object {\n        $_\n    }\n}\n"),
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
