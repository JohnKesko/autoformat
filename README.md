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

## Get Started

1. Install the AutoFormat extension in VS Code.
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
```

4. Save a file. It is formatted automatically.

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
```

- Languages without their own table use the settings under `[formatting]`.
- Language names: `rust`, `toml`, `javascript`, `typescript` (also used for `.tsx`), `csharp`, `powershell`.
- Right now `brace_style` is the only setting you can override.

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

## What it never does

- It never adds, removes or reorders anything. Commas, semicolons, quotes and parentheses stay exactly as you wrote them. Only spaces and line breaks change.
- It never changes the inside of strings or comments.
- It never joins lines you split. (Only exception: `brace_style = "same_line"` moves a `{` up to the line before it.)
- It never formats a file that has syntax errors.
- It never moves a comment to a different place in the code.
- It never changes what your code does. After formatting, AutoFormat reads the result again and compares it with the original. If anything other than spacing changed, it does less, or nothing.

## Supported editors

| Editor | Status |
|---|---|
| VS Code 1.90 or newer | Supported and tested |
| Editors based on VS Code (Cursor, VSCodium, Windsurf) | Should work if you install the `.vsix` file by hand. Not tested yet, and not in their extension store (Open VSX) yet. |
| Other editors (Visual Studio, JetBrains, Neovim, Zed, Helix) | No extension. Editors that can run an external formatter can use the command line: `autoformat format --stdin --stdin-filepath <file>`. Not tested yet. |

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

## Command line

Download it from a GitHub release, or build it yourself:

```bash
cargo install --path crates/autoformat-cli
```

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
- **PowerShell**: PowerShell cares about spaces, so only safe spacing is changed (after commas, around `=` and `|`, after `if`, inside brackets).
- Spaces used to line up `=` signs are reduced to one.

## For developers

### Build and test

You need Rust and a C compiler. For the extension you also need Node.js.

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

### How the code is organized

```
crates/
├── autoformat-config      reads and checks autoformat.toml
├── autoformat-parser      reads code with Tree-sitter
├── autoformat-core        the formatter itself, the same for every language
├── autoformat-languages   one small folder per language with its rules
└── autoformat-cli         the autoformat command
vscode/extension           the VS Code extension
```

### Adding a language

1. Find a Tree-sitter grammar for it with a license that allows use (MIT or similar).
2. Copy one of the folders in `crates/autoformat-languages/` and fill in the rules. `autoformat tree <file>` shows how the grammar sees the code.
3. Add it to the list in `crates/autoformat-languages/src/lib.rs`.
4. Add example files in `tests/fixtures/` and run `AUTOFORMAT_BLESS=1 cargo test` to create the expected results. Check them by hand.

### Release

1. Raise the version in `vscode/extension/package.json` and in `Cargo.toml`. They must be the same.
2. Push to `master`.

GitHub then builds the formatter for Windows, macOS and Linux, publishes the extension to the VS Code Marketplace, and creates a GitHub release. This needs the repository secret `VSC_PERSONAL_TOKEN`.

Pushes that only change `.md` files (like this README) or the license files don't start the workflow. The README on the Marketplace page is updated with the next release.

## License

MIT. See `LICENSE`. Licenses of the libraries we use are in `THIRD_PARTY_LICENSES.md`.
