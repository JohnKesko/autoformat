# AutoFormat

One code formatter with a single configuration file.  
No other formatters, runtimes or language tools are needed.  
Uses [Tree-sitter](https://tree-sitter.github.io/) grammars to understand the code.

**Supported languages:**

- Rust   
- TOML   
- TypeScript   
- JavaScript   
- C#   
- PowerShell   

## Supported IDE's

| Editor | Status |
|---|---|
| VS Code 1.90 or newer | Supported and tested |
| Editors based on VS Code (Cursor, VSCodium, Windsurf) | Should work if you install the `.vsix` file by hand. Not tested yet, and not in their extension store (Open VSX) yet. |
| Zed | Supported. See [Zed](#zed). |
| Other editors (Visual Studio, JetBrains, Neovim, Helix) | No extension. Editors that can use a language server can start `autoformat lsp`. Editors that can run an external formatter can use `autoformat format --stdin --stdin-filepath <file>`. Not tested yet. |

## Get Started

1. Install the AutoFormat extension in VS Code or in Zed.
2. Create a file called `autoformat.toml` anywhere in your project. It doesn't matter if it's a mono repo or not.
3. Add the settings you want:

```toml
# autoformat.toml

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
```

4. Save a file. It is formatted automatically. (Zed needs a few lines of settings first, see [Zed](#zed).)

All settings are optional. A missing setting uses the value shown above.  
No `autoformat.toml` at all? Then all the values above are used.  
Each file uses the closest `autoformat.toml`: in its own folder, or the nearest folder above it.

## Override

Want one language to look different? Give it its own table, `[formatting.<language>]`.  
Settings in that table override the ones under `[formatting]`, for that language only.

```toml
[formatting]
brace_style = "same_line"    # every language uses same_line ...

[formatting.csharp]
brace_style = "allman"       # ... except C#, which uses allman

[formatting.powershell]
indent_size = 2              # PowerShell is indented 2, everything else 4
```

- Every setting can be overridden.
- A language table only changes the settings you put in it. The rest come from `[formatting]`.
- Languages without their own table use the settings under `[formatting]`.
- Language names: `rust`, `toml`, `javascript`, `typescript` (also used for `.tsx`), `csharp`, `powershell`.

## What each setting does

```
# indent_style - Use spaces or tabs for indentation
indent_style = "spaces"
```

```
# indent_size - How deep should indentation be (1 to 16)
indent_size = 4
```

```
# column_width - How many characters a line can be before wrap (20 to 1000)
column_width = 140
```

Lists in `( )` and `[ ]` (function parameters, call arguments, arrays) are put on one line when they fit, and one item per line when they don't:

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

A list is left exactly as you wrote it if it ends with a comma after the last item, or has a comment inside. Lists in `{ }` (objects, initializers, struct bodies) are never wrapped.

```
# comment_alignment -  Aligns comments based on the length of the left side.
# var first = 10;           // This is some comment
# var averylongline = 20;   // This is another comment
# var x = 20;               // TODO: Yep, let's implement this

comment_alignment = true (or false)
```

```
# line_ending - "lf" (macOS and Linux) or "crlf" (Windows)
line_ending = "lf"
```

```
# brace_style
# same_line –   Opening brace stays on the same line (K&R).
# allman    –   Opening brace moves to a new line.
# gnu       –   Opening brace moves to a new line with an additional indentation level for the brace itself.

brace_style = "same_line"
```

### Brace style examples

`brace_style` decides where the `{` of a function, class, `if`, loop and so on goes.  
The three examples below are the same code, formatted with each style.

**`brace_style = "same_line"`** (default)  
The `{` stays at the end of the line. `else` stays on the same line as `}`.

```csharp
public void DoSomething() {
    if (ready) {
        Start();
    } else {
        Wait();
    }
}
```

**`brace_style = "allman"`**  
The `{` goes on its own line. `else` also goes on its own line.

```csharp
public void DoSomething()
{
    if (ready)
    {
        Start();
    }
    else
    {
        Wait();
    }
}
```

**`brace_style = "gnu"`**  
Like `allman`, but the `{` and `}` are indented one extra step.

```csharp
public void DoSomething()
    {
        if (ready)
            {
                Start();
            }
        else
            {
                Wait();
            }
    }
```

`catch`, `finally` and the `while` in `do { } while (...)` work the same way as `else`.

Want a different brace style for one language? See [Override](#override).

### Which braces move?

Only the braces of real code blocks: functions, methods, classes, structs, interfaces, enums, and the blocks of `if`, `else`, loops, `switch`, `try`, `catch` and `finally`.

These never move:
- Object literals, destructuring and arrow functions (JavaScript, TypeScript)
- Object and collection initializers, lambdas and switch expressions (C#)
- Macros, struct literals, closures, and `if` / `match` used as a value (Rust)
- Script blocks like `ForEach-Object { }` and hashtables (PowerShell)
- Inline tables (TOML has no code blocks, so `brace_style` does nothing there)
- Anything inside strings and comments

A block written on one line, like `if (x) { return; }`, stays on one line.

```
# blank_lines - One empty line between functions, classes, methods and other declarations
blank_lines = true
```

```rust
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
```

- The empty line is only added when one of the two takes up more than one line. Short one-liners, like the two `const` lines, stay together.
- Comments and attributes above a declaration (`/// docs`, `#[test]`, `[Fact]`, `@Component`) stay with it. The empty line goes above them.
- Nothing is added inside function bodies. That is your code, and you decide where the empty lines go.
- Several empty lines in a row become one.
- `false`: no empty line between declarations that take up more than one line. Empty lines inside function bodies and between one-liners are still left as you wrote them.

```
# space_around_operators - One space on each side of operators like = + - * / == && ||
space_around_operators = true
```

```js
// before
const result=a+b*10;

// after
const result = a + b * 10;
```

- Never inside strings, comments, regular expressions or template text.
- Operators that belong to one value stay attached to it: `-x`, `!done`, `i++`, `&value`.
- `false`: no spaces around operators.

```js
// space_around_operators = false
const result=a+b*10;
```

- With `false`, a space stays where two signs would otherwise run together and mean something else. `a - -b` keeps its spaces, because `a--b` is different code.
- The arrows `=>` and `->` always keep their spaces.
- In TOML, the `=` between a key and its value counts as an operator too.

```
# final_newline - End the file with one line break
final_newline = true
```

- `true`: the file ends with exactly one line break. Extra empty lines at the end are removed.
- `false`: the file ends right after the last character, with no line break.
- An empty file stays empty.

```
# trim_trailing_whitespace - Remove spaces and tabs at the end of lines
trim_trailing_whitespace = true
```

- Never inside strings. Multi-line strings, raw strings, template strings, here-strings and C# `@"..."` strings keep every space.
- `false`: spaces at the end of a line are left as you wrote them.

### Which declarations blank_lines looks at

| Language | Declarations |
|---|---|
| Rust | Functions, structs, enums, traits, `impl`, `mod`, `use`, `const`, `static` and macros. At the top of the file and inside `mod`, `impl` and `trait`. |
| C# | Namespaces, classes, structs, interfaces, records, enums, methods, constructors, properties, fields and events |
| JavaScript / TypeScript | Functions, classes and `export` at the top of the file, and methods and fields inside classes. TypeScript also: interfaces, types, enums and namespaces. `const` and `let` are normal code and are left alone. |
| PowerShell | Functions, classes and enums, and methods and properties inside classes |
| TOML | Tables: `[table]` and `[[array]]` |

## Changing settings

You can change any setting and change it back later. Formatting with the old setting again gives exactly the same result as before, so you are never stuck with an earlier choice.

One small exception: `trim_trailing_whitespace = false` means "don't remove spaces at the end of lines". It can't bring back spaces that were already removed, because there is nothing left to put back.

## What it never does

- It never adds, removes or reorders anything. Commas, semicolons, quotes and parentheses stay exactly as you wrote them. Only spaces and line breaks change.
- It never changes the inside of strings or comments. The only exception: spaces at the end of a comment line are removed (see `trim_trailing_whitespace`).
- It never joins lines you split, except in two cases: lists in `( )` and `[ ]` that fit within `column_width` are put back on one line, and `brace_style = "same_line"` moves a `{` up to the line before it.
- It never formats a file that has syntax errors.
- It never moves a comment to a different place in the code.
- It never changes what your code does. After formatting, AutoFormat reads the result again and compares it with the original. If anything other than spacing changed, it does less, or nothing.

## VS Code

- The formatter is included in the extension. Nothing is downloaded.
- Errors (like a syntax error) show up in the status bar. Your file is not changed.

### Format on save

AutoFormat formats every supported file when you save it. This is on by default.

- It does this even if VS Code's own `"editor.formatOnSave"` is `false`.
- It does not turn other formatters off. If another formatter is also set to format on save, both run, one after the other. To avoid that, remove the other formatter's settings for that language.
- To turn it off, set `"autoformat.formatOnSave": false` in your VS Code settings.

### Format Document (`Shift+Alt+F`)

Here VS Code decides which formatter to use. If you have more than one formatter for a language, VS Code asks you once which one you want.

To always use AutoFormat, add this to your VS Code settings:

```json
"[rust][toml][javascript][javascriptreact][typescript][typescriptreact][csharp][powershell]": {
    "editor.defaultFormatter": "JohnK.autoformat"
}
```

## Zed

1. Open the Extensions page in Zed, search for **AutoFormat** and install it.
2. Tell Zed to use it. Zed never switches formatter by itself, so add this to your Zed settings (`zed: open settings file`). Keep only the languages you want:

```json
"languages": {
    "Rust":       { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "TOML":       { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "JavaScript": { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "TypeScript": { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "TSX":        { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "CSharp":     { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "PowerShell": { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" }
}
```

3. Save a file. It is formatted. To format without saving, run `editor: format`.

Good to know:

- Zed does not allow extensions to include programs. The first time it is needed, the extension downloads the `autoformat` program from this project's GitHub releases, and it gets newer versions the same way. Without internet it keeps using the one it already has.
- Zed knows Rust, JavaScript and TypeScript from the start. For TOML, C# and PowerShell files you also need Zed's extension for that language. Zed offers to install it when you open such a file.
- `.jsx` files are "JavaScript" in Zed, and `.tsx` files are "TSX".
- A file with a syntax error is not changed. The reason is in Zed's log for the AutoFormat language server (`dev: open language server logs`).
- A mistake in `autoformat.toml` is shown as a message in Zed.

Two settings work differently in Zed, because Zed does these things itself:

- `line_ending`: Zed decides the line endings of a file, with its own `"line_ending"` setting. The one in `autoformat.toml` is not used in Zed.
- `trim_trailing_whitespace = false`: Zed removes spaces at the end of lines when you save. To keep them, also set `"remove_trailing_whitespace_on_save": false` in your Zed settings.

To use your own `autoformat` program instead of the downloaded one:

```json
"lsp": {
    "autoformat": {
        "binary": { "path": "/path/to/autoformat", "arguments": ["lsp"] }
    }
}
```

## Command line

### Install

macOS and Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/JohnKesko/autoformat/master/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/JohnKesko/autoformat/master/install.ps1 | iex
```

The script downloads the newest release for your computer, checks it against the published checksum, and installs it. No administrator rights are needed:

- macOS and Linux: `~/.local/bin/autoformat`. If that folder is not in your PATH, the script tells you the line to add.
- Windows: `%LOCALAPPDATA%\Programs\autoformat\autoformat.exe`, and the folder is added to your PATH.

To install somewhere else, or a specific version, set `AUTOFORMAT_INSTALL_DIR` or `AUTOFORMAT_VERSION` first, for example:

```bash
curl -fsSL https://raw.githubusercontent.com/JohnKesko/autoformat/master/install.sh | AUTOFORMAT_VERSION=0.1.2 sh
```

Or build it yourself (needs Rust and a C compiler):

```bash
cargo install --git https://github.com/JohnKesko/autoformat autoformat-cli
```

### Upgrade

```bash
autoformat upgrade
```

It checks for a newer version, asks before installing, checks the download against the published checksum, and replaces itself. `autoformat upgrade --check` only tells you whether a newer version exists, and `--yes` skips the question.

The copy of autoformat that comes with the VS Code extension is updated by VS Code, together with the extension. The copy that the Zed extension downloads is updated by the Zed extension.

### Use

Format files or whole folders:

```bash
autoformat format src/main.rs config.toml scripts/
```

Check without changing anything (useful in automated builds):

```bash
autoformat format --check src/
```

Other commands:

| Command | What it does |
|---|---|
| `autoformat config` | Shows which settings are used and which `autoformat.toml` they come from |
| `autoformat languages` | Lists the languages and file types |
| `autoformat format --stdin --stdin-filepath a.rs` | Reads code from input and writes the result to output |
| `autoformat lsp` | Runs as a language server that only formats. This is how Zed uses it, and other editors that can talk to language servers can use it too. |

## Supported languages

| Language | File types |
|---|---|
| Rust | `.rs` |
| TOML | `.toml` |
| JavaScript | `.js` `.mjs` `.cjs` `.jsx` |
| TypeScript | `.ts` `.mts` `.cts` `.tsx` |
| C# | `.cs` `.csx` |
| PowerShell | `.ps1` `.psm1` `.psd1` |

Good to know:
- **Rust**: inside macros like `println!(...)` only commas and brackets get spaced. A `{` after a `where` clause stays on its own line.
- **TOML**: keys inside tables are not indented.
- **JavaScript / TypeScript**: JSX markup is kept as written, it is only moved left or right as a whole. The braces of `namespace X { }` never move.
- **C#**: `#if`, `#else` and `#endif` start at the left edge. `#region` lines are indented like code.
- **PowerShell**: PowerShell cares about spaces, so only safe spacing is changed (after commas, around `=` and `|`, after `if`, inside brackets). Other operators like `+`, `-` and `-eq` are left as you wrote them, because in PowerShell a space can change what they mean.
- Spaces used to line up `=` signs are reduced to one.

## For developers

### Build and test

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

`test:vscode` opens a separate VS Code window with its own settings, so your own VS Code is not touched.

The Zed extension is its own small Rust project:

```bash
cd zed/extension && cargo test
```

To try it in Zed, open the Extensions page, click **Install Dev Extension** and pick the `zed/extension` folder. Zed builds it for you (Rust must be installed with rustup).

The extension downloads the newest released `autoformat`. To use the one you just built instead, point Zed to `target/release/autoformat` with the `"lsp"` setting shown under [Zed](#zed).

### How the code is organized

```
crates/
├── autoformat-config      reads and checks autoformat.toml
├── autoformat-parser      reads code with Tree-sitter
├── autoformat-core        the formatter itself, the same for every language
├── autoformat-languages   one small folder per language with its rules
└── autoformat-cli         the autoformat command, including the language server (autoformat lsp)
vscode/extension           the VS Code extension
zed/extension              the Zed extension
```

### Adding a language

1. Find a Tree-sitter grammar for it with a license that allows use (MIT or similar).
2. Copy one of the folders in `crates/autoformat-languages/` and fill in the rules. `autoformat tree <file>` shows how the grammar sees the code.
3. Add it to the list in `crates/autoformat-languages/src/lib.rs`.
4. Add example files in `tests/fixtures/` and run `AUTOFORMAT_BLESS=1 cargo test` to create the expected results. Check them by hand.

### Release

1. Raise the version in `vscode/extension/package.json` and in `Cargo.toml`. They must be the same.
2. Push to `master`.

GitHub then builds the formatter for Windows, macOS and Linux, publishes the extension to the VS Code Marketplace, and creates a GitHub release.

Publishing uses Trusted Publishing: the Marketplace trusts the workflow `release.yml` in this repository, so no token is stored anywhere. This is set up on the publisher page under **Trusted Publishing**. To test the sign-in without publishing anything, run the **Build & Release** workflow by hand on GitHub with **check sign-in only** ticked.

After publishing, the workflow waits until every platform can be downloaded. It can still take a few minutes before VS Code finds the new version.

Pushes that only change `.md` files (like this README) or the license files don't start the workflow. The README on the Marketplace page is updated with the next release.

### Release for Zed

Zed users get every new `autoformat` automatically, because the Zed extension downloads the newest GitHub release. Nothing needs to be done for Zed in a normal release.

The Zed extension itself (`zed/extension`) has its own version, in `zed/extension/extension.toml` and `zed/extension/Cargo.toml`. It only changes when the extension's own code changes. To publish it, or a new version of it, open a pull request to [zed-industries/extensions](https://github.com/zed-industries/extensions):

1. Add this repository as a submodule in `extensions/autoformat` (the first time), or move the submodule to the new commit.
2. In `extensions.toml`, add or update:

```toml
[autoformat]
submodule = "extensions/autoformat"
path = "zed/extension"
version = "0.1.0"
```

The first `autoformat` release that can be used by Zed is the one after 0.1.2, so publish that release before the Zed extension.

## License

MIT. See `LICENSE`. Licenses of the libraries we use are in `THIRD_PARTY_LICENSES.md`.
