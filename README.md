# AutoFormat

One code formatter with one settings file, for all your languages.  
No other formatters, runtimes or language tools are needed.  
It uses [Tree-sitter](https://tree-sitter.github.io/) grammars to understand the code.

| Language | File types | Name in settings |
|---|---|---|
| Rust | `.rs` | `rust` |
| TOML | `.toml` | `toml` |
| JSON | `.json` `.jsonc` | `json` |
| JavaScript | `.js` `.mjs` `.cjs` `.jsx` | `javascript` |
| TypeScript | `.ts` `.mts` `.cts` `.tsx` | `typescript` |
| C# | `.cs` `.csx` | `csharp` |
| Java | `.java` | `java` |
| Kotlin | `.kt` `.kts` | `kotlin` |
| PowerShell | `.ps1` `.psm1` `.psd1` | `powershell` |

| Editor | Status |
|---|---|
| VS Code 1.90 or newer | Supported and tested |
| Zed | Supported. See [Zed](#zed). |
| Cursor, VSCodium, Windsurf | Should work if you install the `.vsix` file by hand. Not tested. |
| Other editors | No extension. Use `autoformat lsp` or `autoformat format --stdin`. Not tested. |

## Get started

1. Install the AutoFormat extension in VS Code or Zed.
2. Create a file called `autoformat.toml` anywhere in your project.
3. Save a code file. It is formatted. (Zed needs a few lines of settings first, see [Zed](#zed).)

```toml
# autoformat.toml - every setting, with its default value

[formatting]
indent_size = 4
indent_style = "spaces"
column_width = 140
comment_alignment = true
line_ending = "lf"
brace_style = "same_line"
blank_lines = true
space_around_operators = true
final_newline = true
trim_trailing_whitespace = true
continuation_indent = false
```

- Every setting is optional. A missing setting, or a missing file, uses the values above.
- Each code file uses the closest `autoformat.toml`: in its own folder, or the nearest folder above it.

## Settings

| Setting | Values | What it does |
|---|---|---|
| `indent_style` | `"spaces"`, `"tabs"` | Indent with spaces or tabs |
| `indent_size` | 1 to 16 | How deep one indentation step is |
| `column_width` | 20 to 1000 | How long a line may be before lists are wrapped |
| `comment_alignment` | `true`, `false` | Lines up comments at the end of neighbouring lines |
| `line_ending` | `"lf"`, `"crlf"` | `lf` for macOS and Linux, `crlf` for Windows |
| `brace_style` | `"same_line"`, `"allman"`, `"gnu"` | Where the `{` of a code block goes |
| `blank_lines` | `true`, `false` | One empty line between functions, classes and other declarations |
| `space_around_operators` | `true`, `false` | One space on each side of `=` `+` `-` `*` `/` `==` `&&` `\|\|` |
| `final_newline` | `true`, `false` | End the file with exactly one line break, or with none |
| `trim_trailing_whitespace` | `true`, `false` | Remove spaces and tabs at the end of lines |
| `continuation_indent` | `true`, `false` | Push in a block in the middle of a statement (PowerShell only) |

You can change any setting and change it back later. The result is always the same as before, so you are never stuck with an earlier choice. One exception: spaces at the end of lines that were already removed can't come back.

### One language different

Give a language its own table, `[formatting.<name>]`. It only changes the settings you put in it.

```toml
[formatting]
brace_style = "same_line"    # every language ...

[formatting.csharp]
brace_style = "allman"       # ... except C#
```

### column_width

Lists in `( )` and `[ ]` go on one line when they fit, and one item per line when they don't:

```rust
// column_width = 80
pub async fn incoming(
    stream: TcpStream,
    tls_acceptor: TlsAcceptor,
    context: Arc<Context>
) {}

// column_width = 140
pub async fn incoming(stream: TcpStream, tls_acceptor: TlsAcceptor, context: Arc<Context>) {}
```

- A list with a comma after the last item, or a comment inside, is left exactly as you wrote it.
- Array lists in `{ }` (Java `{1, 2, 3}`, C# `new[] { 1, 2, 3 }`) and JSON objects work the same way.
- Everything else in `{ }` (JavaScript objects, C# initializers, struct bodies) keeps your line breaks.

### comment_alignment

```js
var first = 10;           // This is some comment
var averylongline = 20;   // This is another comment
```

### brace_style

```csharp
// same_line: the { stays at the end of the line, else stays next to }
if (ready) {
    Start();
} else {
    Wait();
}

// allman: the { and else go on their own lines
if (ready)
{
    Start();
}
else
{
    Wait();
}

// gnu: like allman, but the braces are indented one extra step
if (ready)
    {
        Start();
    }
```

- `catch`, `finally` and the `while` in `do { } while (...)` work like `else`.
- Only the braces of real code blocks move: functions, classes, `if`, `else`, loops, `switch`, `try`.
- Braces that are part of a value never move: objects, initializers, lambdas, closures, anonymous classes, PowerShell script blocks and hashtables, and anything in JSON and TOML.
- A block written on one line, like `if (x) { return; }`, stays on one line.

### blank_lines

```rust
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
```

- The empty line is only added when one of the two takes up more than one line. One-liners stay together.
- Comments and attributes above a declaration (`/// docs`, `#[test]`, `@Override`) stay with it.
- Nothing is added inside function bodies. Several empty lines in a row become one.
- Declarations are functions, classes, methods and the like. In TOML they are tables. In JavaScript and TypeScript, `const` and `let` are normal code and are left alone.
- Java and Kotlin also get one empty line after the `package` line and after the imports.

### space_around_operators

```js
const result=a+b*10;      // false
const result = a + b * 10;  // true
```

- Never inside strings, comments or regular expressions.
- Signs that belong to one value stay attached: `-x`, `!done`, `i++`, `&value`.
- With `false`, a space stays where two signs would otherwise mean something else (`a - -b`). The arrows `=>` and `->` always keep their spaces.

### continuation_indent

```powershell
# false
Get-Process | ForEach-Object {
    $_.Name
} | Sort-Object

# true
Get-Process | ForEach-Object {
        $_.Name
    } | Sort-Object
```

- Only when the statement goes on after the `}`. A block that ends the statement is left alone.
- Blocks on one line, like `ForEach-Object { $_.Name }`, are left alone.

## What it never does

- It never adds, removes or reorders anything. Only spaces and line breaks change.
- It never changes the inside of strings or comments.
- It never joins lines you split, except lists that fit within `column_width`, and a `{` that `brace_style = "same_line"` moves up.
- It never formats a file that has syntax errors.
- It never changes what your code does. After formatting, AutoFormat reads the result again and compares it with the original. If anything other than spacing changed, it does less, or nothing.

## VS Code

- The formatter is included in the extension. Nothing is downloaded.
- Files are formatted when you save. To turn that off, set `"autoformat.formatOnSave": false`.
- Other formatters are not turned off. If another one also formats on save, both run.
- Errors (like a syntax error) show up in the status bar. Your file is not changed.

For Format Document (`Shift+Alt+F`), VS Code picks the formatter. To always use AutoFormat:

```json
"[rust][toml][json][jsonc][javascript][javascriptreact][typescript][typescriptreact][csharp][java][kotlin][powershell]": {
    "editor.defaultFormatter": "JohnK.autoformat"
}
```

## Zed

1. Install the AutoFormat extension. Until it is in Zed's extension list, install it from this repository: see [zed/extension](zed/extension/README.md).
2. Zed never switches formatter by itself. Add one line per language to your Zed settings (`zed: open settings file`):

```json
"languages": {
    "Rust": { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "TOML": { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" }
}
```

Zed's names are `Rust`, `TOML`, `JSON`, `JSONC`, `JavaScript`, `TypeScript`, `TSX`, `CSharp`, `Java`, `Kotlin` and `PowerShell`.

3. Save a file. It is formatted.

Good to know:

- The extension downloads the `autoformat` program from this project's GitHub releases, and updates it the same way.
- For TOML, C#, Java, Kotlin and PowerShell you also need Zed's extension for that language.
- A file with a syntax error is not changed. A mistake in `autoformat.toml` is shown as a message.
- Zed handles line endings itself, so `line_ending` is not used in Zed.
- Zed removes spaces at the end of lines on save. To keep them, also set `"remove_trailing_whitespace_on_save": false`.

## Command line

Install on macOS and Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/JohnKesko/autoformat/master/install.sh | sh
```

Install on Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/JohnKesko/autoformat/master/install.ps1 | iex
```

No administrator rights are needed. To pick the folder or the version, set `AUTOFORMAT_INSTALL_DIR` or `AUTOFORMAT_VERSION` first.

| Command | What it does |
|---|---|
| `autoformat format src/ a.toml` | Formats files and whole folders |
| `autoformat format --check src/` | Only checks, changes nothing (for automated builds) |
| `autoformat format --stdin --stdin-filepath a.rs` | Reads code from input and writes the result to output |
| `autoformat config` | Shows which settings are used and where they come from |
| `autoformat languages` | Lists the languages and file types |
| `autoformat upgrade` | Installs a newer version, after asking. `--check` only looks. |
| `autoformat lsp` | Runs as a language server that only formats (used by Zed) |

## Notes per language

- **Rust**: inside macros like `println!(...)` only commas and brackets get spaced.
- **TOML**: keys inside tables are not indented.
- **JSON**: comments in `.jsonc` are kept. A file with a comma after the last item (`[1, 2,]`) or a number like `1e+5` is left unchanged.
- **JavaScript / TypeScript**: JSX markup is kept as written, and only moved left or right as a whole.
- **C#**: `#if`, `#else` and `#endif` start at the left edge. `#region` lines are indented like code.
- **Java**: no spaces inside generics (`Map<String, List<Integer>>`) or array lists (`{1, 2, 3}`). Text blocks (`"""`) are printed exactly as written.
- **Kotlin**: a line break can end a statement in Kotlin, so long lines are only wrapped after a comma inside `( )` and `[ ]`, and the `{` of a lambda never moves.
- **PowerShell**: a space can change the meaning in PowerShell, so only safe spacing is changed: after commas, around `=` and `|`, after `if`, inside brackets, and before the `{` in `ForEach-Object {`. Operators like `+` and `-eq` are left as you wrote them.

## For developers

You need Rust and a C compiler. For the VS Code extension you also need Node.js.

```bash
cargo build --release
```

```bash
cargo test --workspace
```

```bash
cd vscode/extension && npm ci && npm run test:unit && npm run test:vscode
```

```bash
cd zed/extension && cargo test
```

```
crates/
├── autoformat-config      reads and checks autoformat.toml
├── autoformat-parser      reads code with Tree-sitter
├── autoformat-core        the formatter itself, the same for every language
├── autoformat-languages   one small folder per language with its rules
└── autoformat-cli         the autoformat command, including autoformat lsp
vscode/extension           the VS Code extension
zed/extension              the Zed extension
```

**Add a language**

1. Find a Tree-sitter grammar with a license that allows use (MIT or similar).
2. Copy one of the folders in `crates/autoformat-languages/` and fill in the rules. `autoformat tree <file>` shows how the grammar sees the code.
3. Add it to the list in `crates/autoformat-languages/src/lib.rs`.
4. Add example files in `tests/fixtures/`, run `AUTOFORMAT_BLESS=1 cargo test`, and check the results by hand.

**Release**

1. Raise the version in `Cargo.toml` and in `vscode/extension/package.json`. They must be the same.
2. Push to `master`.

GitHub then builds the program for Windows, macOS and Linux, publishes the VS Code extension and creates a GitHub release. No token is stored: the Marketplace trusts the workflow `release.yml` (Trusted Publishing). Pushes that only change `.md` or license files don't start a release.

**Release for Zed**

Zed users get every new `autoformat` automatically. The Zed extension itself has its own version in `zed/extension/extension.toml` and `zed/extension/Cargo.toml`, and only changes when its own code changes. To publish it, open a pull request to [zed-industries/extensions](https://github.com/zed-industries/extensions) that adds this repository as a submodule in `extensions/autoformat` and this to `extensions.toml`:

```toml
[autoformat]
submodule = "extensions/autoformat"
path = "zed/extension"
version = "0.1.1"
```

## License

MIT. See `LICENSE`. Licenses of the libraries we use are in `THIRD_PARTY_LICENSES.md`.
