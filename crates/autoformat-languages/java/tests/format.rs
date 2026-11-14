use std::path::Path;

use autoformat_core::config::{BraceStyle, Config};
use autoformat_core::format_source;
use autoformat_core::testing::{check_fixtures, format_or_panic};
use autoformat_lang_java::JavaLanguage;

#[test]
fn fixtures() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let count = check_fixtures(&JavaLanguage::new(), &directory);
    assert!(count >= 2);
}

/// Formats with the default settings, checks the result, and checks that formatting the
/// result again changes nothing.
fn check(source: &str, expected: &str) {
    check_with(&Config::default(), source, expected);
}

fn check_with(config: &Config, source: &str, expected: &str) {
    let java = JavaLanguage::new();
    let formatted = format_or_panic(&java, source, config);
    assert_eq!(formatted, expected);
    assert_eq!(format_or_panic(&java, &formatted, config), expected);
}

/// Puts statements inside a method, so that the examples below stay short.
fn in_method(body: &str) -> String {
    format!("class A {{\n    void run() {{\n{}    }}\n}}\n", body)
}

fn check_statements(source: &str, expected: &str) {
    check(&in_method(source), &in_method(expected));
}

#[test]
fn generics_have_no_spaces_inside_the_brackets() {
    check_statements(
        "List < Map < String , Integer > > a=new ArrayList < > ();\n",
        "        List<Map<String, Integer>> a = new ArrayList<>();\n",
    );
    check_statements(
        "Map<?,? extends Number> b=Collections.< String , Integer >emptyMap();\n",
        "        Map<?, ? extends Number> b = Collections.<String, Integer>emptyMap();\n",
    );
    check_statements(
        "Supplier<List<String>> c=Lists::< String >of;\n",
        "        Supplier<List<String>> c = Lists::<String>of;\n",
    );
    // Comparing and shifting are still operators with spaces.
    check_statements(
        "boolean d=a<b&&b>c;\nint e=a>>2|b>>>3|c<<1;\n",
        "        boolean d = a < b && b > c;\n        int e = a >> 2 | b >>> 3 | c << 1;\n",
    );
}

#[test]
fn generic_declarations() {
    check(
        "class Box < T extends Comparable<T>&Runnable > extends Base < T >{\n    public< R >R map(Function<? super T,? extends R> f,T ... rest){ return null; }\n}\n",
        "class Box<T extends Comparable<T> & Runnable> extends Base<T> {\n    public <R> R map(Function<? super T, ? extends R> f, T... rest) { return null; }\n}\n",
    );
}

#[test]
fn annotations_do_not_indent_the_line_after_them() {
    let source = "@Entity\n@Table( name=\"orders\" )\npublic class Order{\n@Id\n@Column(name=\"id\",nullable=false)\nprivate long id;\n@Override public String toString(){ return \"\"; }\n@Deprecated\n@SuppressWarnings({\"a\",\"b\"})\nvoid old(@NotNull final String name,@Size(max=5)List<@NonNull String> tags)\nthrows IOException{\n}\n}\n";
    let expected = "@Entity\n@Table(name = \"orders\")\npublic class Order {\n    @Id\n    @Column(name = \"id\", nullable = false)\n    private long id;\n\n    @Override public String toString() { return \"\"; }\n\n    @Deprecated\n    @SuppressWarnings({\"a\", \"b\"})\n    void old(@NotNull final String name, @Size(max = 5) List<@NonNull String> tags)\n        throws IOException {\n    }\n}\n";
    check(source, expected);
}

#[test]
fn lambdas_and_method_references() {
    check_statements(
        "Function<Integer,Integer> inc=x->x+1;\nBiFunction<Integer,Integer,Integer> add=( a,b )->{ return a+b; };\nRunnable r=()->{};\n",
        "        Function<Integer, Integer> inc = x -> x + 1;\n        BiFunction<Integer, Integer, Integer> add = (a, b) -> { return a + b; };\n        Runnable r = () -> {};\n",
    );
    check_statements(
        "names.stream().map(String :: valueOf).forEach(System.out::println);\nSupplier<List<String>> make=ArrayList :: new;\n",
        "        names.stream().map(String::valueOf).forEach(System.out::println);\n        Supplier<List<String>> make = ArrayList::new;\n",
    );
}

#[test]
fn switch_with_colons_and_with_arrows() {
    check_statements(
        "switch(k){\ncase 1:\ncase 2:\nreturn 1;\ncase 3:{\nreturn 3;\n}\ndefault:\nreturn 0;\n}\n",
        "        switch (k) {\n            case 1:\n            case 2:\n                return 1;\n            case 3: {\n                return 3;\n            }\n            default:\n                return 0;\n        }\n",
    );
    check_statements(
        "String s=switch(day){\ncase MON,TUE->\"early\";\ncase WED->{\nyield \"mid\";\n}\ndefault->\"late\";\n};\n",
        "        String s = switch (day) {\n            case MON, TUE -> \"early\";\n            case WED -> {\n                yield \"mid\";\n            }\n            default -> \"late\";\n        };\n",
    );
}

#[test]
fn text_blocks_and_literals_are_printed_exactly_as_written() {
    // The text block is not moved, and no space inside it changes.
    let source = "class A {\n        String a = \"\"\"\n  Hello,   \"World\"\n      a=b+c\n  \"\"\";\n  String b=\"x  +  y\";char c='\\t';long d=1_000L+0xFFL;double e=1.5e-3;\n}\n";
    let expected = "class A {\n    String a = \"\"\"\n  Hello,   \"World\"\n      a=b+c\n  \"\"\";\n\n    String b = \"x  +  y\"; char c = '\\t'; long d = 1_000L + 0xFFL; double e = 1.5e-3;\n}\n";
    check(source, expected);
}

#[test]
fn enums_with_bodies() {
    let source = "enum Op implements IntBinaryOperator{\nADD(\"+\"){\n@Override public int applyAsInt(int a,int b){ return a+b; }\n},\nSUB(\"-\"){\n@Override public int applyAsInt(int a,int b){ return a-b; }\n};\nprivate final String symbol;\nOp(String symbol){\nthis.symbol=symbol;\n}\n}\n";
    let expected = "enum Op implements IntBinaryOperator {\n    ADD(\"+\") {\n        @Override public int applyAsInt(int a, int b) { return a + b; }\n    },\n    SUB(\"-\") {\n        @Override public int applyAsInt(int a, int b) { return a - b; }\n    };\n    private final String symbol;\n\n    Op(String symbol) {\n        this.symbol = symbol;\n    }\n}\n";
    check(source, expected);
    check("enum Day{MON,TUE,WED}\n", "enum Day { MON, TUE, WED }\n");
}

#[test]
fn records() {
    let source = "record Point(int x,int y)implements Comparable<Point>{\nPoint{\nif(x<0)throw new IllegalArgumentException();\n}\npublic int compareTo(Point o){ return Integer.compare(x,o.x); }\n}\n";
    let expected = "record Point(int x, int y) implements Comparable<Point> {\n    Point {\n        if (x < 0) throw new IllegalArgumentException();\n    }\n\n    public int compareTo(Point o) { return Integer.compare(x, o.x); }\n}\n";
    check(source, expected);
}

#[test]
fn anonymous_classes() {
    check_statements(
        "Runnable r=new Runnable(){\n@Override\npublic void run(){\ngo();\n}\n};\n",
        "        Runnable r = new Runnable() {\n            @Override\n            public void run() {\n                go();\n            }\n        };\n",
    );
}

#[test]
fn array_initializers_have_no_spaces_inside_the_braces() {
    check_statements(
        "int[] a={ 1,2,3 };\nString[] b=new String[]{\"x\",\"y\"};\nint[][] c=new int [3] [4];\nc [0] [1]=a [2];\nint[] d={ };\n",
        "        int[] a = {1, 2, 3};\n        String[] b = new String[] {\"x\", \"y\"};\n        int[][] c = new int[3][4];\n        c[0][1] = a[2];\n        int[] d = {};\n",
    );
}

#[test]
fn array_initializers_follow_column_width() {
    let narrow = Config {
        column_width: 40,
        ..Config::default()
    };
    let one_line = "class A {\n    int[] sizes = {10, 20, 30, 40, 50, 60, 70};\n}\n";
    let wrapped = "class A {\n    int[] sizes = {\n        10,\n        20,\n        30,\n        40,\n        50,\n        60,\n        70\n    };\n}\n";
    check_with(&narrow, one_line, wrapped);
    // With room again, the list goes back on one line. So does a short list that was
    // written over several lines.
    check(wrapped, one_line);
    check(
        "class A {\n    String[] names = {\n        \"a\",\n        \"b\"\n    };\n}\n",
        "class A {\n    String[] names = {\"a\", \"b\"};\n}\n",
    );
    // The lists inside a list are kept together; the outer list is wrapped first.
    check_with(
        &narrow,
        "class A {\n    int[][] grid = {{1, 0, 0}, {0, 1, 0}, {0, 0, 1}};\n}\n",
        "class A {\n    int[][] grid = {\n        {1, 0, 0},\n        {0, 1, 0},\n        {0, 0, 1}\n    };\n}\n",
    );
    // The same for the lists of an annotation.
    check_with(
        &narrow,
        "@SuppressWarnings({\"unchecked\", \"rawtypes\", \"deprecation\"})\nclass A {}\n",
        "@SuppressWarnings(\n    {\n        \"unchecked\",\n        \"rawtypes\",\n        \"deprecation\"\n    }\n)\nclass A {}\n",
    );
}

#[test]
fn array_initializers_with_a_final_comma_or_a_comment_keep_their_lines() {
    let narrow = Config {
        column_width: 40,
        ..Config::default()
    };
    let kept = [
        "class A {\n    int[] a = {\n        1, 2,\n        3,\n    };\n}\n",
        "class A {\n    int[] a = {\n        1,  // one\n        2\n    };\n}\n",
        "class A {\n    int[][] grid = {\n        {1, 0},\n        {0, 1},\n    };\n}\n",
        "class A {\n    int[] sizes = {10, 20, 30, 40, 50, 60, 70, 80,};\n}\n",
    ];
    for source in kept {
        check(source, source);
        check_with(&narrow, source, source);
    }
}

#[test]
fn ternaries_and_casts() {
    check_statements(
        "int x=a>b?a:b;\nint y=(int)3.7+(int)-z;\nObject o=(Object)\"text\";\nString s=((String)o).trim();\nvar both=(Runnable&Serializable)()->{};\n",
        "        int x = a > b ? a : b;\n        int y = (int) 3.7 + (int) -z;\n        Object o = (Object) \"text\";\n        String s = ((String) o).trim();\n        var both = (Runnable & Serializable) () -> {};\n",
    );
}

#[test]
fn labels_and_loops() {
    check_statements(
        "outer:\nfor(int i=0;i<10;i++){\nfor(String s:names){\nif(s==null)continue outer;\nelse break outer;\n}\n}\ndo{\ni--;\n}while(i>0);\nassert i==0:\"done\";\n",
        "        outer:\n        for (int i = 0; i < 10; i++) {\n            for (String s : names) {\n                if (s == null) continue outer;\n                else break outer;\n            }\n        }\n        do {\n            i--;\n        } while (i > 0);\n        assert i == 0 : \"done\";\n",
    );
}

const TRY_SAME_LINE: &str = "class A {\n    void run() {\n        try (InputStream in = open(); OutputStream out = sink()) {\n            in.transferTo(out);\n        } catch (IOException | RuntimeException e) {\n            log(e);\n        } finally {\n            done();\n        }\n        if (a) {\n            one();\n        } else if (b) {\n            two();\n        } else {\n            three();\n        }\n        synchronized (this) {\n            count++;\n        }\n    }\n}\n";

const TRY_ALLMAN: &str = "class A\n{\n    void run()\n    {\n        try (InputStream in = open(); OutputStream out = sink())\n        {\n            in.transferTo(out);\n        }\n        catch (IOException | RuntimeException e)\n        {\n            log(e);\n        }\n        finally\n        {\n            done();\n        }\n        if (a)\n        {\n            one();\n        }\n        else if (b)\n        {\n            two();\n        }\n        else\n        {\n            three();\n        }\n        synchronized (this)\n        {\n            count++;\n        }\n    }\n}\n";

#[test]
fn try_with_resources_and_else_catch_finally_placement() {
    let messy = "class A{\nvoid run(){\ntry(InputStream in=open();OutputStream out=sink()){\nin.transferTo(out);\n}\ncatch(IOException|RuntimeException e){\nlog(e);\n}\nfinally{\ndone();\n}\nif(a){\none();\n}\nelse if(b){\ntwo();\n}\nelse{\nthree();\n}\nsynchronized(this){\ncount++;\n}\n}\n}\n";
    let allman = Config {
        brace_style: BraceStyle::Allman,
        ..Config::default()
    };
    check(messy, TRY_SAME_LINE);
    check_with(&allman, messy, TRY_ALLMAN);
    // Each style can be reached from the other one.
    check(TRY_ALLMAN, TRY_SAME_LINE);
    check_with(&allman, TRY_SAME_LINE, TRY_ALLMAN);
}

#[test]
fn switch_braces_follow_brace_style() {
    let same_line = "class A {\n    int run(int k) {\n        switch (k) {\n            case 1 -> {\n                one();\n            }\n            default -> two();\n        }\n        return switch (k) {\n            case 1 -> 10;\n            default -> 20;\n        };\n    }\n}\n";
    let allman_text = "class A\n{\n    int run(int k)\n    {\n        switch (k)\n        {\n            case 1 ->\n            {\n                one();\n            }\n            default -> two();\n        }\n        return switch (k)\n        {\n            case 1 -> 10;\n            default -> 20;\n        };\n    }\n}\n";
    let allman = Config {
        brace_style: BraceStyle::Allman,
        ..Config::default()
    };
    check_with(&allman, same_line, allman_text);
    check(allman_text, same_line);
}

#[test]
fn lambda_and_anonymous_class_braces_stay_where_they_are() {
    let source = "class A\n{\n    void run()\n    {\n        items.forEach(item -> {\n            use(item);\n        });\n        Runnable r = new Runnable() {\n            public void run()\n            {\n            }\n        };\n    }\n}\n";
    let allman = Config {
        brace_style: BraceStyle::Allman,
        ..Config::default()
    };
    check_with(&allman, source, source);
}

#[test]
fn comments_and_javadoc_keep_their_text() {
    let source = "// first\nclass A{   // after brace\n/**\n   * Says  hello.\n     * @param name  who\n */\nvoid hello(String name){/* inline */go();// end\n}\n/* a block\n   comment */\nint x;\n}\n";
    let expected = "// first\nclass A {  // after brace\n    /**\n     * Says  hello.\n     * @param name  who\n     */\n    void hello(String name) {  /* inline */\n        go();  // end\n    }\n\n    /* a block\n       comment */\n    int x;\n}\n";
    check(source, expected);
}

#[test]
fn interfaces_and_annotation_types() {
    let source = "interface Shape{\ndouble area();\ndefault String name(){ return \"shape\"; }\n}\n@interface Marker{\nString value()default \"x\";\nint[] ids()default{1,2};\n}\n";
    let expected = "interface Shape {\n    double area();\n    default String name() { return \"shape\"; }\n}\n\n@interface Marker {\n    String value() default \"x\";\n    int[] ids() default {1, 2};\n}\n";
    check(source, expected);
}

#[test]
fn chained_calls_and_wrapped_declarations_are_indented_once() {
    let source = "public class Cache<K,V> extends Base<K,V>\nimplements Map<K,V>,Closeable{\nvoid run(){\nlist.stream()\n.filter(x->x!=null)\n.count();\nint total=first\n+second;\n}\n}\n";
    let expected = "public class Cache<K, V> extends Base<K, V>\n    implements Map<K, V>, Closeable {\n    void run() {\n        list.stream()\n            .filter(x -> x != null)\n            .count();\n        int total = first\n            + second;\n    }\n}\n";
    check(source, expected);
}

#[test]
fn long_lines_are_wrapped_at_column_width() {
    let narrow = Config {
        column_width: 60,
        ..Config::default()
    };
    let source = "class A {\n    void run(String firstName, String secondName, List<Map<String, Integer>> counts) {\n    }\n}\n";
    let expected = "class A {\n    void run(\n        String firstName,\n        String secondName,\n        List<Map<String, Integer>> counts\n    ) {\n    }\n}\n";
    check_with(&narrow, source, expected);
    // With room again, the list goes back on one line.
    check(expected, source);
}

#[test]
fn without_operator_spaces() {
    let off = Config {
        space_around_operators: false,
        ..Config::default()
    };
    let on = "        int x = a + b * c;\n        boolean ok = x >= 1 && !done;\n        Map<?, ?> any = null;\n        Function<Integer, Integer> inc = v -> v + 1;\n        for (String s : names) {}\n        int y = i - -x;\n";
    let without = "        int x=a+b*c;\n        boolean ok=x>=1&&!done;\n        Map<?, ?> any=null;\n        Function<Integer, Integer> inc=v -> v+1;\n        for (String s : names) {}\n        int y=i - -x;\n";
    check_with(&off, &in_method(on), &in_method(without));
    check(&in_method(without), &in_method(on));
    check_with(
        &off,
        "class A {\n    void run(String ... names) {}\n}\n",
        "class A {\n    void run(String... names) {}\n}\n",
    );
}

#[test]
fn a_file_with_a_syntax_error_is_left_unchanged() {
    let java = JavaLanguage::new();
    let broken = [
        "class A { void run( { } }\n",
        "class A {\n    int x = ;\n}\n",
        "class { }\n",
    ];
    for source in broken {
        let result = format_source(&java, source, &Config::default());
        assert!(result.is_err(), "expected a syntax error for: {}", source);
    }
}

#[test]
fn an_empty_line_follows_the_package_line_and_the_imports() {
    let without = "// License\npackage a.b;\nimport c.d;\nimport e.f;\n// about A\nclass A {}\ninterface B {}\n";
    let with = "// License\npackage a.b;\n\nimport c.d;\nimport e.f;\n\n// about A\nclass A {}\ninterface B {}\n";
    let off = Config {
        blank_lines: false,
        ..Config::default()
    };
    // `blank_lines = true` adds the empty lines, `false` takes them away again.
    check(without, with);
    check_with(&off, with, without);
    check("package a;\nclass A {}\n", "package a;\n\nclass A {}\n");
}
