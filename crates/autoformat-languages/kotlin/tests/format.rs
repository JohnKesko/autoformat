use std::path::Path;

use autoformat_core::config::{BraceStyle, Config};
use autoformat_core::format_source;
use autoformat_core::testing::{check_fixtures, format_or_panic};
use autoformat_lang_kotlin::KotlinLanguage;

#[test]
fn fixtures() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let count = check_fixtures(&KotlinLanguage::new(), &directory);
    assert!(count >= 3);
}

/// Formats with the default settings, checks the result, and checks that formatting the
/// result again changes nothing.
fn check(source: &str, expected: &str) {
    check_with(&Config::default(), source, expected);
}

fn check_with(config: &Config, source: &str, expected: &str) {
    let kotlin = KotlinLanguage::new();
    let formatted = format_or_panic(&kotlin, source, config);
    assert_eq!(formatted, expected);
    assert_eq!(format_or_panic(&kotlin, &formatted, config), expected);
}

/// Checks that formatting changes nothing.
fn check_unchanged(source: &str) {
    check(source, source);
}

/// Puts statements inside a function, so that the examples below stay short.
fn in_function(body: &str) -> String {
    format!("fun run() {{\n{}}}\n", body)
}

fn check_statements(source: &str, expected: &str) {
    check(&in_function(source), &in_function(expected));
}

fn with_style(style: BraceStyle) -> Config {
    Config {
        brace_style: style,
        ..Config::default()
    }
}

/// The lines of a text without their indentation and without empty lines.
fn code_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.lines() {
        if !line.trim().is_empty() {
            lines.push(line.trim().to_string());
        }
    }
    lines
}

/// Undoes the line breaks that Kotlin ignores: after `(` or `,` and before `)`.
fn join_bracket_breaks(lines: Vec<String>) -> Vec<String> {
    let mut joined: Vec<String> = Vec::new();
    for line in lines {
        match joined.last_mut() {
            Some(previous)
                if previous.ends_with('(') || previous.ends_with(',') || line.starts_with(')') =>
            {
                previous.push_str(&line);
            }
            _ => joined.push(line),
        }
    }
    joined
}

// ---------------------------------------------------------------------------------------
// Text that is never changed.

#[test]
fn strings_and_templates_are_printed_exactly_as_written() {
    check_statements(
        "val a=\"$name  has ${ items.size+1 } items, ${if(x){\"a\"}else{\"b\"}}\"\n",
        "    val a = \"$name  has ${ items.size+1 } items, ${if(x){\"a\"}else{\"b\"}}\"\n",
    );
    check_statements(
        "val b='\\n';val c=\"\\\"quoted\\\" \\$notATemplate\"\n",
        "    val b = '\\n'; val c = \"\\\"quoted\\\" \\$notATemplate\"\n",
    );
}

#[test]
fn raw_strings_keep_every_line_as_written() {
    let source = "fun run() {\n        val text=\"\"\"\n   first   line\n\t$name ${ a+b }\n      \"quoted\"   \n  \"\"\".trimIndent()\n}\n";
    let expected = "fun run() {\n    val text = \"\"\"\n   first   line\n\t$name ${ a+b }\n      \"quoted\"   \n  \"\"\".trimIndent()\n}\n";
    check(source, expected);
}

#[test]
fn numbers_keep_their_text() {
    check_statements(
        "val a=1_000_000L+0xFF_FFu-0b1010+1.5e-3f+.5\n",
        "    val a = 1_000_000L + 0xFF_FFu - 0b1010 + 1.5e-3f + .5\n",
    );
}

#[test]
fn comments_keep_their_text() {
    check(
        "/**\n   * KDoc for [run].\n   * @param name   the name\n */\nfun run(name:String){ // trailing\n/* outer /* nested */ still outer */\n  go()   /* why */\n//last\n}\n",
        "/**\n * KDoc for [run].\n * @param name   the name\n */\nfun run(name: String) {  // trailing\n    /* outer /* nested */ still outer */\n    go()  /* why */\n    //last\n}\n",
    );
}

// ---------------------------------------------------------------------------------------
// Kotlin's own shapes.

#[test]
fn when_with_arrows() {
    check_statements(
        "val text=when(x){\n1,2->\"low\"\nin 3..9->\"mid\"\n!in 10..99->\"big\"\nis String->x.length\n!is Int->\"other\"\nelse->{\n\"none\"\n}\n}\n",
        "    val text = when (x) {\n        1, 2 -> \"low\"\n        in 3..9 -> \"mid\"\n        !in 10..99 -> \"big\"\n        is String -> x.length\n        !is Int -> \"other\"\n        else -> {\n            \"none\"\n        }\n    }\n",
    );
    // `when` without a value, and conditions on lines of their own.
    check_statements(
        "when{\na>b&&c->one()\nFirst,\nSecond,\nThird->\ntwo()\n}\n",
        "    when {\n        a > b && c -> one()\n        First,\n        Second,\n        Third ->\n            two()\n    }\n",
    );
}

#[test]
fn safe_calls_elvis_not_null_and_references() {
    check_statements(
        "val a=user ?. name?.length?:0\nval b=user !!.name !!\nval c=String :: length\nval d=::run\nval e=user?.let{it.name}?:return\n",
        "    val a = user?.name?.length ?: 0\n    val b = user!!.name!!\n    val c = String::length\n    val d = ::run\n    val e = user?.let { it.name } ?: return\n",
    );
}

#[test]
fn ranges_keep_the_spacing_they_were_written_with() {
    check_statements(
        "for(i in 0..10)a()\nfor(i in 0..<size)b()\nfor(i in 10 downTo 0 step 2)c()\nval r=first .. last\n",
        "    for (i in 0..10) a()\n    for (i in 0..<size) b()\n    for (i in 10 downTo 0 step 2) c()\n    val r = first .. last\n",
    );
}

#[test]
fn type_checks_and_casts() {
    check_statements(
        "val a=x as String\nval b=x as? String\nval c=x is String&&y !is Int\nval d=x in list||y !in list\nif(x!is String)return\n",
        "    val a = x as String\n    val b = x as? String\n    val c = x is String && y !is Int\n    val d = x in list || y !in list\n    if (x!is String) return\n",
    );
}

#[test]
fn nullable_types_and_generics() {
    check(
        "val a:Map < String ? ,List < Int > ? > ?=null\nfun < T:Any > first(items:List < out T >,sink:MutableList < in T >,any:Array < * >):T ?=null\nclass Box < T > (val item:T):Base < T > ()\nval b=listOf < String > ()\nval c=a<b&&b>c\n",
        "val a: Map<String?, List<Int>?>? = null\nfun <T : Any> first(items: List<out T>, sink: MutableList<in T>, any: Array<*>): T? = null\nclass Box<T>(val item: T) : Base<T>()\nval b = listOf<String>()\nval c = a < b && b > c\n",
    );
}

#[test]
fn annotations() {
    check(
        "@file:JvmName( \"Demo\" )\n@file:Suppress(\"a\",\"b\")\npackage demo\nclass A{\n@get:Rule val rule=1\n@field:Inject lateinit var x:X\n@Test fun a(){}\n@Test\n@Ignore(\"slow\")\nfun b(){}\n@Deprecated(\nmessage=\"old\",\nreplaceWith=ReplaceWith(\"b()\"),\n)\nfun c(){}\n}\n",
        "@file:JvmName(\"Demo\")\n@file:Suppress(\"a\", \"b\")\npackage demo\n\nclass A {\n    @get:Rule val rule = 1\n    @field:Inject lateinit var x: X\n    @Test fun a() {}\n\n    @Test\n    @Ignore(\"slow\")\n    fun b() {}\n\n    @Deprecated(\n        message = \"old\",\n        replaceWith = ReplaceWith(\"b()\"),\n    )\n    fun c() {}\n}\n",
    );
    // `@Composable (Int) -> Unit` is not `@Composable(Int) -> Unit`: the space between
    // an annotation and a `(` that is not its own is kept.
    check(
        "val draw:@Composable (Int)->Unit={}\nval size=@Suppress(\"a\") (b+c)\n",
        "val draw: @Composable (Int) -> Unit = {}\nval size = @Suppress(\"a\") (b + c)\n",
    );
}

#[test]
fn labels_are_kept_together() {
    check_statements(
        "outer@ for(a in list){\ninner@ while(true){\nif(a)break@outer else continue@inner\n}\n}\nlist.forEach lit@{\nif(it==0)return@lit\nreturn@forEach\n}\nval b=this@Outer.name+super@Outer.name\nreturn@run 1\n",
        "    outer@ for (a in list) {\n        inner@ while (true) {\n            if (a) break@outer else continue@inner\n        }\n    }\n    list.forEach lit@{\n        if (it == 0) return@lit\n        return@forEach\n    }\n    val b = this@Outer.name + super@Outer.name\n    return@run 1\n",
    );
}

#[test]
fn lambdas() {
    check_statements(
        "val a={x:Int,y:Int->x+y}\nval b={println()}\nval c:(Int)->Unit={}\nlist.map{it*2}.filter{v->v>0}.forEach(::println)\nrun{\nwork()\n}\nfold(0){acc,(key,value)->\nacc+key+value\n}\ncall(1,2){done()}\ncall({first()},{second()})\n",
        "    val a = { x: Int, y: Int -> x + y }\n    val b = { println() }\n    val c: (Int) -> Unit = {}\n    list.map { it * 2 }.filter { v -> v > 0 }.forEach(::println)\n    run {\n        work()\n    }\n    fold(0) { acc, (key, value) ->\n        acc + key + value\n    }\n    call(1, 2) { done() }\n    call({ first() }, { second() })\n",
    );
}

#[test]
fn arguments_spread_and_unary_minus() {
    check_statements(
        "call(1,name=2,*items,other=-x,flag=!done)\nval a=-b+ -c*+d\nval e=i++ + --j\n",
        "    call(1, name = 2, *items, other = -x, flag = !done)\n    val a = -b + -c * +d\n    val e = i++ + --j\n",
    );
    check(
        "fun join(vararg parts:String,separator:String=\", \",limit:Int=-1)=parts.joinToString(separator)\n",
        "fun join(vararg parts: String, separator: String = \", \", limit: Int = -1) = parts.joinToString(separator)\n",
    );
}

#[test]
fn properties_with_get_and_set_are_indented_under_the_property() {
    check(
        "class A{\nvar name:String=\"\"\nget()=field.trim()\nprivate set(value){\nfield=value\n}\nval size:Int get()=1\nval lazyOne by lazy{compute()}\n}\nval top:Int\nget()=2\n",
        "class A {\n    var name: String = \"\"\n        get() = field.trim()\n        private set(value) {\n            field = value\n        }\n\n    val size: Int get() = 1\n    val lazyOne by lazy { compute() }\n}\n\nval top: Int\n    get() = 2\n",
    );
}

#[test]
fn classes_objects_and_enums() {
    check(
        "data class Point(val x:Int,val y:Int=0)\nsealed class Shape:Drawable\nsealed interface Node\nvalue class Id(val raw:Long)\nfun interface Action{fun run()}\nobject Registry:Base(),Closeable{\nconst val MAX=1\n}\nclass Outer{\ninner class Inner\ncompanion object Factory{\nfun create()=Outer()\n}\n}\nenum class Color(val rgb:Int){\nRED(0xFF0000),\nGREEN(0x00FF00){\noverride fun label()=\"g\"\n};\nopen fun label()=name\n}\n",
        "data class Point(val x: Int, val y: Int = 0)\nsealed class Shape : Drawable\nsealed interface Node\nvalue class Id(val raw: Long)\nfun interface Action { fun run() }\n\nobject Registry : Base(), Closeable {\n    const val MAX = 1\n}\n\nclass Outer {\n    inner class Inner\n\n    companion object Factory {\n        fun create() = Outer()\n    }\n}\n\nenum class Color(val rgb: Int) {\n    RED(0xFF0000),\n    GREEN(0x00FF00) {\n        override fun label() = \"g\"\n    };\n\n    open fun label() = name\n}\n",
    );
}

#[test]
fn constructors_and_initializers() {
    check(
        "class A private constructor(val a:Int):B(a){\ninit{\ncheck(a>0)\n}\nconstructor():this(1)\nconstructor(a:String):this(a.length){\nlog(a)\n}\n}\n",
        "class A private constructor(val a: Int) : B(a) {\n    init {\n        check(a > 0)\n    }\n\n    constructor() : this(1)\n\n    constructor(a: String) : this(a.length) {\n        log(a)\n    }\n}\n",
    );
}

#[test]
fn extension_and_single_expression_functions() {
    check(
        "fun String.shout()=uppercase()+\"!\"\nfun <T> List<T>.second():T=this[1]\nval <T> List<T>.last:T get()=this[size-1]\ninfix fun Int.plus2(other:Int):Int=this+other\noperator fun Point.plus(other:Point)=Point(x+other.x,y+other.y)\nfun long():Int=\ncompute(1)\nsuspend fun <T> retry(times:Int,block:suspend ()->T):T=block()\n",
        "fun String.shout() = uppercase() + \"!\"\nfun <T> List<T>.second(): T = this[1]\nval <T> List<T>.last: T get() = this[size - 1]\ninfix fun Int.plus2(other: Int): Int = this + other\noperator fun Point.plus(other: Point) = Point(x + other.x, y + other.y)\n\nfun long(): Int =\n    compute(1)\n\nsuspend fun <T> retry(times: Int, block: suspend () -> T): T = block()\n",
    );
}

#[test]
fn destructuring() {
    check_statements(
        "val (a,b)=pair\nval (c,_)=other\nfor((key,value) in map)use(key,value)\nmap.forEach{(key,value)->use(key,value)}\n",
        "    val (a, b) = pair\n    val (c, _) = other\n    for ((key, value) in map) use(key, value)\n    map.forEach { (key, value) -> use(key, value) }\n",
    );
}

#[test]
fn colons() {
    check(
        "class A:B,C\nclass D<T:Any>(val x:T):E<T>() where T:Comparable<T>\nval o=object:Runnable{override fun run(){}}\nfun f(a:Int,b:(x:Int)->Unit):String?=null\n",
        "class A : B, C\nclass D<T : Any>(val x: T) : E<T>() where T : Comparable<T>\nval o = object : Runnable { override fun run() {} }\nfun f(a: Int, b: (x: Int) -> Unit): String? = null\n",
    );
}

#[test]
fn control_flow_without_braces_is_indented() {
    check_statements(
        "if(a)\nb()\nelse if(c)\nd()\nelse\ne()\nfor(item in items)\nsend(item)\nwhile(busy)\nwait()\nval x=if(a)1\nelse 2\n",
        "    if (a)\n        b()\n    else if (c)\n        d()\n    else\n        e()\n    for (item in items)\n        send(item)\n    while (busy)\n        wait()\n    val x = if (a) 1\n    else 2\n",
    );
}

#[test]
fn chained_calls_are_indented_once() {
    check_statements(
        "val names=users\n.filter{it.active}\n.map{\nit.name\n}\n?.sorted()\n?:emptyList()\n",
        "    val names = users\n        .filter { it.active }\n        .map {\n            it.name\n        }\n        ?.sorted()\n        ?: emptyList()\n",
    );
}

#[test]
fn infix_calls_and_brackets_around_values() {
    check_statements(
        "val a=1 shl 2 or (3 and 4)\nval b=map to (key to value)\nif(ready) (first as Runnable).run()\ncheck (a>0){\"bad\"}\n",
        "    val a = 1 shl 2 or (3 and 4)\n    val b = map to (key to value)\n    if (ready) (first as Runnable).run()\n    check(a > 0) { \"bad\" }\n",
    );
}

#[test]
fn scripts() {
    check(
        "plugins{\nkotlin(\"jvm\") version \"2.0.0\"\n}\ndependencies{\nimplementation(\"a:b:1\")\n}\n",
        "plugins {\n    kotlin(\"jvm\") version \"2.0.0\"\n}\ndependencies {\n    implementation(\"a:b:1\")\n}\n",
    );
}

// ---------------------------------------------------------------------------------------
// Kotlin has no semicolons: a line break can end a statement.

/// Code where a line break decides the meaning. Only the indentation may change.
const LINE_BREAK_SOURCES: &[&str] = &[
    // The second line is its own statement: `+ c` is not added to `b`.
    "fun run() {\n    val a = b\n    + c\n    val d = e\n    - f\n}\n",
    // A call, then a separate block in braces.
    "fun run() {\n    start()\n    { later() }\n    val block = make\n    { work() }\n}\n",
    // These operators may start a line, and must stay at the start of that line.
    "fun run() {\n    val a = first\n        ?: second\n    val b = left\n        && right\n        || other\n    val c = value\n        .trim()\n        ?.length\n    val d = value\n        as String\n}\n",
    // An operator at the end of a line continues on the next line.
    "fun run() {\n    val total = first +\n        second -\n        third\n    val both = left &&\n        right\n}\n",
    // A trailing lambda must start on the line of its call.
    "fun run() {\n    items.forEach { item ->\n        use(item)\n    }\n    val job = launch(context) {\n        work()\n    }\n    val listener = object : Listener {\n        override fun done() {}\n    }\n    val action = fun(x: Int): Int {\n        return x\n    }\n}\n",
    // `return` alone, then a value on the next line: two statements.
    "fun run(): Int {\n    if (done) return\n    compute()\n    val name = infix\n    call(1)\n    return 1\n}\n",
    // A very long line with no brackets is never split.
    "fun run() {\n    val total = firstNumberWithALongName + secondNumberWithALongName * thirdNumberWithALongName - fourthNumberWithALongName / fifthNumberWithALongName\n    val chain = firstValueWithALongName.secondValueWithALongName.thirdValueWithALongName.fourthValueWithALongName.fifthValueWithALongName\n    val flag = firstConditionWithALongName && secondConditionWithALongName || thirdConditionWithALongName && fourthConditionWithALongName\n}\n",
];

#[test]
fn meaning_changing_line_breaks_are_never_added_or_removed() {
    let kotlin = KotlinLanguage::new();
    let styles = [BraceStyle::SameLine, BraceStyle::Allman, BraceStyle::Gnu];
    for source in LINE_BREAK_SOURCES {
        for style in styles {
            for column_width in [20, 60, 140] {
                for space_around_operators in [true, false] {
                    let config = Config {
                        brace_style: style,
                        column_width,
                        space_around_operators,
                        ..Config::default()
                    };
                    let formatted = format_or_panic(&kotlin, source, &config);
                    // Only the `{` and `}` of `fun run()` and of `override fun done()` may
                    // move. Everything else stays on the line it was written on.
                    let moved_braces = formatted
                        .replace("fun run()\n{", "fun run() {")
                        .replace("fun run(): Int\n{", "fun run(): Int {")
                        .replace("fun run()\n    {", "fun run() {")
                        .replace("fun run(): Int\n    {", "fun run(): Int {");
                    // Line breaks inside round brackets are undone before comparing:
                    // Kotlin ignores those.
                    let without_spaces = |text: &str| -> Vec<String> {
                        join_bracket_breaks(code_lines(text))
                            .iter()
                            .map(|line| line.replace(' ', ""))
                            .collect()
                    };
                    assert_eq!(
                        without_spaces(&moved_braces),
                        without_spaces(source),
                        "style {:?}, width {}, operator spaces {}\n{}",
                        style,
                        column_width,
                        space_around_operators,
                        formatted
                    );
                }
            }
        }
    }
}

#[test]
fn long_lines_are_only_wrapped_after_commas_inside_brackets() {
    let config = Config {
        column_width: 40,
        ..Config::default()
    };
    check_with(
        &config,
        "fun connect(host: String, port: Int = 80, secure: Boolean = false): Connection = open(host, port, secure)\n",
        "fun connect(\n    host: String,\n    port: Int = 80,\n    secure: Boolean = false\n): Connection = open(host, port, secure)\n",
    );
    check_with(
        &config,
        "class Server(val host: String, val port: Int, val secure: Boolean) : Base(host, port)\n",
        "class Server(\n    val host: String,\n    val port: Int,\n    val secure: Boolean\n) : Base(host, port)\n",
    );
    // The lambda after the call stays on the line of the `)`.
    check_with(
        &config,
        "fun run() {\n    val result = combine(firstValue, secondValue, thirdValue) { a, b, c -> a + b + c }\n}\n",
        "fun run() {\n    val result = combine(\n        firstValue,\n        secondValue,\n        thirdValue\n    ) { a, b, c -> a + b + c }\n}\n",
    );
    // A wrapped list goes back on one line when it fits again.
    check(
        "fun connect(\n    host: String,\n    port: Int = 80\n): Connection = open(\n    host,\n    port\n)\n",
        "fun connect(host: String, port: Int = 80): Connection = open(host, port)\n",
    );
    // A final comma keeps the list as it was written.
    check_unchanged(
        "fun connect(\n    host: String,\n    port: Int = 80,\n): Connection = open(host, port)\n",
    );
}

// ---------------------------------------------------------------------------------------
// brace_style.

const BRACES_SAME_LINE: &str = "\
class Orders(val items: List<Order>) : Service() {
    val size: Int
        get() {
            return items.size
        }

    init {
        check(items.isNotEmpty())
    }

    constructor() : this(emptyList()) {
        log()
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
        while (busy) {
            wait()
        }
        do {
            retry()
        } while (busy)
        try {
            save()
        } catch (error: IOException) {
            log(error)
        } finally {
            close()
        }
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

object Registry {
    val all = mutableListOf<Orders>()
}

interface Named {
    val name: String
}

enum class Kind {
    FIRST,
    SECOND
}
";

const BRACES_ALLMAN: &str = "\
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

    constructor() : this(emptyList())
    {
        log()
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
        while (busy)
        {
            wait()
        }
        do
        {
            retry()
        }
        while (busy)
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

object Registry
{
    val all = mutableListOf<Orders>()
}

interface Named
{
    val name: String
}

enum class Kind
{
    FIRST,
    SECOND
}
";

#[test]
fn block_braces_follow_brace_style() {
    check_with(
        &with_style(BraceStyle::Allman),
        BRACES_SAME_LINE,
        BRACES_ALLMAN,
    );
    check_with(
        &with_style(BraceStyle::SameLine),
        BRACES_ALLMAN,
        BRACES_SAME_LINE,
    );
    check_with(
        &with_style(BraceStyle::SameLine),
        BRACES_SAME_LINE,
        BRACES_SAME_LINE,
    );
    // `gnu` and back.
    let kotlin = KotlinLanguage::new();
    let gnu = format_or_panic(&kotlin, BRACES_SAME_LINE, &with_style(BraceStyle::Gnu));
    check_with(&with_style(BraceStyle::SameLine), &gnu, BRACES_SAME_LINE);
    check_with(&with_style(BraceStyle::Allman), &gnu, BRACES_ALLMAN);
}

/// Braces that are part of an expression. Moving them could change the meaning, so
/// they stay where they were written in every style.
const EXPRESSION_BRACES: &str = "\
fun run()
{
    items.forEach {
        use(it)
    }
    val job = scope.launch(context) { value ->
        work(value)
    }
    val listener = object : Listener {
        override fun done()
        {
            finish()
        }
    }
    val action = fun(x: Int): Int {
        return x
    }
    start()
        {
            later()
        }
}

enum class Mode
{
    FAST {
        override fun speed() = 2
    },
    SLOW {
        override fun speed() = 1
    };

    abstract fun speed(): Int
}
";

#[test]
fn lambda_and_other_expression_braces_stay_where_they_are() {
    check_with(
        &with_style(BraceStyle::Allman),
        EXPRESSION_BRACES,
        EXPRESSION_BRACES,
    );
    let same_line = EXPRESSION_BRACES
        .replace("fun run()\n{", "fun run() {")
        .replace("fun done()\n        {", "fun done() {")
        .replace("enum class Mode\n{", "enum class Mode {");
    check_with(
        &with_style(BraceStyle::SameLine),
        EXPRESSION_BRACES,
        &same_line,
    );
    check_with(
        &with_style(BraceStyle::Allman),
        &same_line,
        EXPRESSION_BRACES,
    );
}

// ---------------------------------------------------------------------------------------
// Other settings.

#[test]
fn without_operator_spaces() {
    let config = Config {
        space_around_operators: false,
        ..Config::default()
    };
    let spaced = "fun run(a: Int = 1, b: List<Int>? = null): Int = a + b!!.size\n\nfun main() {\n    val x = if (a >= b && c != d) a - -b else a * b % c\n    val y = x as? String ?: name ?: return\n    call(name = x, other = -y) { v -> v + 1 }\n    val z = a in b || c !in d || e is F\n    total += 1\n    when (x) {\n        1 -> one()\n        else -> two()\n    }\n}\n";
    let tight = "fun run(a: Int=1, b: List<Int>?=null): Int=a+b!!.size\n\nfun main() {\n    val x=if (a>=b&&c!=d) a - -b else a*b%c\n    val y=x as? String?:name?:return\n    call(name=x, other=-y) { v -> v+1 }\n    val z=a in b||c !in d||e is F\n    total+=1\n    when (x) {\n        1 -> one()\n        else -> two()\n    }\n}\n";
    check_with(&config, spaced, tight);
    check(tight, spaced);
}

#[test]
fn indentation_blank_lines_and_final_newline() {
    let config = Config {
        indent_size: 2,
        blank_lines: false,
        final_newline: false,
        ..Config::default()
    };
    check_with(
        &config,
        "class A {\n    fun a() {\n        go()\n    }\n\n    fun b() {\n        go()\n    }\n}\n",
        "class A {\n  fun a() {\n    go()\n  }\n  fun b() {\n    go()\n  }\n}",
    );
    check(
        "class A {\n  fun a() {\n    go()   \n  }\n  fun b() {\n    go()\n  }\n}",
        "class A {\n    fun a() {\n        go()\n    }\n\n    fun b() {\n        go()\n    }\n}\n",
    );
}

#[test]
fn a_file_with_a_syntax_error_is_left_unchanged() {
    let kotlin = KotlinLanguage::new();
    for broken in [
        "fun run( {\n}\n",
        "class A {\n    fun a() {\n",
        "val x = = 1\n",
    ] {
        let result = format_source(&kotlin, broken, &Config::default());
        assert!(result.is_err(), "expected an error for {:?}", broken);
    }
}

#[test]
fn an_empty_line_follows_the_package_line_and_the_imports() {
    let without = "package a.b\nimport c.d\nimport e.f\n// about A\nclass A\nfun main() {}\n";
    let with = "package a.b\n\nimport c.d\nimport e.f\n\n// about A\nclass A\nfun main() {}\n";
    let off = Config {
        blank_lines: false,
        ..Config::default()
    };
    // `blank_lines = true` adds the empty lines, `false` takes them away again.
    check(without, with);
    check_with(&off, with, without);
    check("import c.d\nval x = 1\n", "import c.d\n\nval x = 1\n");
}
