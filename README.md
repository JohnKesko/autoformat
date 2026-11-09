# AutoFormat

One small, predictable code formatter for **Rust, TOML, TypeScript, JavaScript, C# and
PowerShell**. Install one thing, write one configuration file, and every supported file
is formatted the same way.

- Written in Rust. No other formatters, runtimes or language tools are needed.
- Uses [Tree-sitter](https://tree-sitter.github.io/) grammars to understand the code.
- Has exactly five settings.
- Never changes what your code does: every result is parsed again and compared with
  the original before it is used.
- Available as a command line tool and as a VS Code extension.
- MIT licensed.

AutoFormat is deliberately not another Prettier or rustfmt. It does not rewrite your code
into one "correct" shape. It keeps your line breaks and fixes the things that make code
hard to read.

## What it does

- Re-indents every line from the structure of the code.
- Normalizes spaces between tokens: `a+b` becomes `a + b`, `foo (x)` becomes `foo(x)`,
  `if(x){` becomes `if (x) {`, `[ 1,2 ]` becomes `[1, 2]`, and so on.
- Removes trailing whitespace, keeps at most one blank line in a row, and removes blank
  lines right after an opening bracket or before a closing one.
- Wraps lines longer than `column_width` by putting the contents of a bracket group one
  item per line:

  ```rust
  let total = add_numbers(
      first_value,
      second_value,
      third_value
  );
  ```

- Aligns trailing comments on consecutive lines:

  ```csharp
  var first = 10;   // First value
  var second = 20;  // Second value
  var third = 30;   // Third value
  ```

- Applies the configured line ending and ends every file with exactly one newline.

## What it never does

- It never adds, removes or reorders tokens. Commas, semicolons, quotes and parentheses
  stay exactly as you wrote them. Only whitespace changes.
- It never changes the inside of strings, raw strings, template strings, here-strings,
  interpolated strings or regular expressions.
- It never joins lines you split, and never formats a file that has syntax errors.
- It never moves a comment to a different place in the code.

## Install

### VS Code

Install the AutoFormat extension (a `.vsix` file for your platform, see *Releasing*). The
formatter binary is included, so it works right away. Nothing is downloaded.

- **Format Document** (`Shift+Alt+F`) uses AutoFormat for all six languages.
- **Format on save** is on by default (`autoformat.formatOnSave`).
- To make AutoFormat the default formatter for a language, add for example
  `"[rust]": { "editor.defaultFormatter": "autoformat.autoformat" }` to your settings.
- Errors (such as syntax errors) appear in the status bar and in the *AutoFormat* output
  channel. The document is left unchanged.
- `autoformat.executablePath` lets you use your own build of the `autoformat` binary.

### Command line

Download the archive for your platform from a release, or build it yourself:

```bash
cargo install --path crates/autoformat-cli
```

## Usage

```bash
autoformat format src/main.rs
```

```bash
autoformat format src/App.tsx config.toml scripts/
```

Directories are searched for supported files. Hidden folders, `target` and
`node_modules` are skipped.

```bash
autoformat format --check src/
```

`--check` changes nothing and exits with code 1 if any file is not formatted. Use it in CI.

```bash
autoformat format --stdin --stdin-filepath src/lib.rs < src/lib.rs
```

`--stdin` reads code from standard input and writes the result to standard output. The
file path picks the language and is used to find `autoformat.toml`. Use `--language`
(`rust`, `toml`, `javascript`, `typescript`, `tsx`, `csharp`, `powershell`) when there is
no file name.

Other commands:

| Command | What it does |
|---|---|
| `autoformat config [path]` | Shows the configuration that applies to a path, and which file it came from |
| `autoformat languages` | Lists the supported languages and file extensions |
| `autoformat tree <file>` | Prints the syntax tree of a file (useful when writing language rules) |

Exit codes: `0` success, `1` a file could not be formatted (or is not formatted, with
`--check`), `2` invalid command line or configuration.

## Configuration

Create `autoformat.toml` in your project. AutoFormat uses the closest one found in the
file's folder or any parent folder. Without a file, the defaults are used.

```toml
[formatting]
indent_size = 4
indent_style = "spaces"
column_width = 140
comment_alignment = true
line_ending = "lf"
```

| Option | Values | Default | Meaning |
|---|---|---|---|
| `indent_size` | 1 to 16 | `4` | Width of one indentation level |
| `indent_style` | `"spaces"`, `"tabs"` | `"spaces"` | Indent with spaces or tabs |
| `column_width` | 20 to 1000 | `140` | Lines longer than this are wrapped when possible |
| `comment_alignment` | `true`, `false` | `true` | Align trailing comments on consecutive lines. When off, a trailing comment is separated by one space |
| `line_ending` | `"lf"`, `"crlf"` | `"lf"` | Line ending for the whole file |

All options are optional. Unknown options, unknown tables and invalid values are errors
with a clear message; they are never ignored.

Notes:
- Aligned trailing comments start two spaces after the longest line in the block. A block
  is a run of consecutive lines with the same indentation that all end with a comment.
- `line_ending` applies to the whole file, including line breaks inside multi-line
  strings, the same way other formatters handle it.
- Wrapping only happens at bracket groups such as argument lists, parameter lists,
  arrays and object literals. A line that cannot be wrapped (a long string, for example)
  is left as it is.

## Supported languages

| Language | Extensions | Grammar |
|---|---|---|
| Rust | `.rs` | tree-sitter-rust 0.24 |
| TOML | `.toml` | tree-sitter-toml-ng 0.7 |
| JavaScript | `.js` `.mjs` `.cjs` `.jsx` | tree-sitter-javascript 0.25 |
| TypeScript | `.ts` `.mts` `.cts` `.tsx` | tree-sitter-typescript 0.23 |
| C# | `.cs` `.csx` | tree-sitter-c-sharp 0.23 |
| PowerShell | `.ps1` `.psm1` `.psd1` | tree-sitter-powershell 0.26 |

Language notes and current limitations:

- **Rust**: inside macro calls such as `println!(...)` only commas and brackets are
  spaced, because the grammar cannot see the structure inside a macro. `macro_rules!`
  bodies keep their spacing exactly.
- **TOML**: keys inside tables are not indented. Inline tables are never wrapped, because
  TOML requires them to stay on one line.
- **JavaScript / TypeScript**: JSX/TSX markup is kept exactly as written; when its first
  line moves, the following lines move by the same amount (JSX ignores that whitespace).
  Code inside `{...}` in JSX and inside `${...}` in template strings is not reformatted.
- **C#**: `#if`, `#else`, `#endif` and similar directives start at column 0; `#region`
  lines are indented like code. Code where `#if` branches split a block in two cannot be
  parsed and is not formatted.
- **PowerShell**: PowerShell is sensitive to spaces (`-Path x` versus `-Pathx`), so only
  spacing known to be safe is changed: after commas, around `=` and `|`, after keywords
  like `if`, and inside brackets. Everything else keeps its original spacing.
- All languages: AutoFormat keeps your line breaks, so two differently broken versions of
  the same code can stay different. Spaces used to line up `=` signs are reduced to one.
- Files must be valid UTF-8. Code the grammar cannot parse (including very new syntax a
  grammar does not know yet) is reported as a syntax error and left unchanged.

## How it stays safe

Every time a file is formatted, AutoFormat:

1. parses the original with Tree-sitter and refuses to continue on syntax errors,
2. formats it,
3. checks that every character other than whitespace is unchanged and in the same order,
4. parses the result and checks that the syntax tree has exactly the same shape and the
   same token text as the original (comments may only change their indentation, strings
   not at all).

If the check fails, it tries again with less ambition: first without line wrapping, then
changing only indentation. If even that fails, the file is left alone and an error is
reported. `--verbose` shows when a fallback was used.

## How it works

```
crates/
├── autoformat-config      reads, validates and finds autoformat.toml
├── autoformat-parser      Tree-sitter integration: parsing, tokens, tree comparison
├── autoformat-core        the language-independent engine
├── autoformat-languages   the list of languages and file type detection
│   ├── rust, toml, javascript, typescript, csharp, powershell   (one adapter each)
└── autoformat-cli         the `autoformat` command
vscode/extension           the VS Code extension (a thin TypeScript wrapper)
```

The formatting pipeline:

1. **Parse** the source into a concrete syntax tree (`autoformat-parser`).
2. **Tokenize**: the leaves of the tree become tokens. Strings, comments and similar
   nodes are kept whole and printed exactly as written.
3. **Build the layout** (`autoformat-core`, `build_layout`): a language-independent
   document of tokens with layout facts: hard line breaks from the source, bracket groups
   (which indent their contents and offer break opportunities for wrapping), indentation
   anchors (where enclosing statements start, for continuation lines), and the spacing
   decision for each token.
4. **Print** (`print`): compute every line's indentation, wrap long lines at bracket
   groups, apply spacing, align trailing comments.
5. **Validate** the result as described above.

The engine contains no language-specific logic. A language adapter is a small crate that
implements the `LanguageAdapter` trait: it provides the grammar, tables of node kinds
(comments, strings, containers, wrappable lists...) and a hook for spacing rules that the
tables cannot express. JavaScript and TypeScript share their rules through
`autoformat_lang_javascript::ecmascript`.

### Adding a language

1. Check that a Tree-sitter grammar crate exists, is compatible with the `tree-sitter`
   version in `Cargo.toml` and has a compatible license.
2. Create `crates/autoformat-languages/<name>/` like the existing adapters and fill in
   `LanguageRules`. Use `autoformat tree <file>` to see the node kinds.
3. Register the adapter in `crates/autoformat-languages/src/lib.rs`.
4. Add fixtures (`tests/fixtures/<name>.input.<ext>`) and run
   `AUTOFORMAT_BLESS=1 cargo test -p autoformat-lang-<name>` to create the expected files.
   Review them carefully.
5. Add the VS Code language id to `vscode/extension/src/languages.ts` and
   `package.json`.

## Building and testing

Requirements: a Rust toolchain (1.90 or newer) and a C compiler (the grammars are C code).
For the extension: Node.js 20 or newer.

```bash
cargo build --release
```

```bash
cargo test --workspace
```

The tests cover, for every language: fixture files with expected output, idempotency
(formatting twice gives the same result) under five different configurations, that only
whitespace changes, that comments are not lost, that strings are untouched, syntax error
handling, column width wrapping, and configuration handling. There are also tests that run
the real `autoformat` binary.

```bash
cd vscode/extension && npm ci && npm run test:unit
```

```bash
cd vscode/extension && npm run test:vscode
```

`test:vscode` starts a separate VS Code instance (with its own settings and extension
folders) and checks Format Document, format on save, all languages and error handling. It
needs `cargo build --release` first. It uses the VS Code installed on your machine when it
can find it (set `VSCODE_EXECUTABLE` to point to one); otherwise it downloads a test copy.

## Releasing

1. Update the version in `Cargo.toml` (`workspace.package.version`) and
   `vscode/extension/package.json`.
2. Push a tag such as `v0.1.0`. The `Release` workflow (`.github/workflows/release.yml`)
   builds the binary for Windows (x64, arm64), macOS (x64, arm64) and Linux (x64, arm64,
   static musl builds that also run on Alpine), and packages one `.vsix` per platform plus
   a command line archive.
3. Publish the `.vsix` files with `vsce publish --packagePath <file>` (one per platform).

To build a package by hand for one platform:

```bash
scripts/package-extension.sh darwin-arm64 target/release/autoformat
```

## License

MIT. See `LICENSE`. Third-party licenses are listed in `THIRD_PARTY_LICENSES.md`.
