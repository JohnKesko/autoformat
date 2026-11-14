# AutoFormat for Zed

One formatter for Rust, TOML, JSON, TypeScript, JavaScript, C#, Java, Kotlin and PowerShell.
It uses the same `autoformat.toml` as the VS Code extension.

## Install

AutoFormat is not in Zed's extension list yet, so install it from this folder:

1. Clone this repository.
2. Install Rust with [rustup](https://rustup.rs). Zed uses it to build the extension.
3. In Zed, open the Extensions page (`Cmd+Shift+X`).
4. Click **Install Dev Extension** and pick this folder (`zed/extension`).

## Turn it on

Zed never switches formatter by itself. Add this to your Zed settings (`zed: open settings file`) and keep the languages you want:

```json
"languages": {
    "Rust":       { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "TOML":       { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "JSON":       { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "JSONC":      { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "JavaScript": { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "TypeScript": { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "TSX":        { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "CSharp":     { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "Java":       { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "Kotlin":     { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" },
    "PowerShell": { "formatter": { "language_server": { "name": "autoformat" } }, "format_on_save": "on" }
}
```

Save a file. It is formatted.

## Good to know

- The extension downloads the `autoformat` program from this project's GitHub releases. You need a release newer than 0.1.2. JSON, Java and Kotlin need release 0.1.4 or newer.
- For TOML, C#, Java, Kotlin and PowerShell you also need Zed's extension for that language.
- A file with a syntax error is not changed.

## Use your own autoformat program

Build it with `cargo build --release` in the repository root, then add this to your Zed settings:

```json
"lsp": {
    "autoformat": {
        "binary": { "path": "/full/path/to/target/release/autoformat", "arguments": ["lsp"] }
    }
}
```

All settings and details are in the [main README](../../README.md).
