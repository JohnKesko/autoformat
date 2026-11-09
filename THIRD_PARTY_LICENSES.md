# Third-party licenses

AutoFormat itself is MIT licensed (see `LICENSE`). The `autoformat` executable is built
from the Rust crates below. All of them use licenses that are compatible with MIT.
The list was produced from `cargo metadata` (normal and build dependencies of
`autoformat-cli`).

## Tree-sitter and the language grammars

| Crate | Version | License | Source |
|---|---|---|---|
| tree-sitter | 0.27.2 | MIT | https://github.com/tree-sitter/tree-sitter |
| tree-sitter-language | 0.1.9 | MIT | https://github.com/tree-sitter/tree-sitter |
| tree-sitter-rust | 0.24.2 | MIT | https://github.com/tree-sitter/tree-sitter-rust |
| tree-sitter-toml-ng | 0.7.0 | MIT | https://github.com/tree-sitter-grammars/tree-sitter-toml |
| tree-sitter-javascript | 0.25.0 | MIT | https://github.com/tree-sitter/tree-sitter-javascript |
| tree-sitter-typescript | 0.23.2 | MIT | https://github.com/tree-sitter/tree-sitter-typescript |
| tree-sitter-c-sharp | 0.23.5 | MIT | https://github.com/tree-sitter/tree-sitter-c-sharp |
| tree-sitter-powershell | 0.26.6 | MIT | https://github.com/airbus-cert/tree-sitter-powershell |

## Other crates

| Crate | Version | License | Source |
|---|---|---|---|
| serde, serde_core, serde_derive | 1.0.229 | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| serde_json | 1.0.154 | MIT OR Apache-2.0 | https://github.com/serde-rs/json |
| serde_spanned | 1.1.3 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| toml | 1.2.1 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| toml_datetime, toml_parser, toml_writer | 1.1.x / 1.2.0 | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| winnow | 1.0.5 | MIT | https://github.com/winnow-rs/winnow |
| regex, regex-automata, regex-syntax | 1.13.1 / 0.4.18 / 0.8.11 | MIT OR Apache-2.0 | https://github.com/rust-lang/regex |
| aho-corasick | 1.1.5 | Unlicense OR MIT | https://github.com/BurntSushi/aho-corasick |
| memchr | 2.8.3 | Unlicense OR MIT | https://github.com/BurntSushi/memchr |
| streaming-iterator | 0.1.9 | MIT OR Apache-2.0 | https://github.com/sfackler/streaming-iterator |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/indexmap |
| equivalent | 1.0.3 | Apache-2.0 OR MIT | https://github.com/indexmap-rs/equivalent |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown |
| foldhash | 0.2.0 | Zlib | https://github.com/orlp/foldhash |
| itoa | 1.0.19 | MIT OR Apache-2.0 | https://github.com/dtolnay/itoa |
| zmij | 1.0.23 | MIT | https://github.com/dtolnay/zmij |
| cc, find-msvc-tools (build only) | 1.11.1 / 0.1.15 | MIT OR Apache-2.0 | https://github.com/rust-lang/cc-rs |
| shlex (build only) | 2.0.1 | MIT OR Apache-2.0 | https://github.com/comex/rust-shlex |
| proc-macro2, quote, syn (build only) | 1.0.107 / 1.0.47 / 3.0.10 | MIT OR Apache-2.0 | https://github.com/dtolnay |
| unicode-ident (build only) | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | https://github.com/dtolnay/unicode-ident |

## Only used by `autoformat upgrade`

| Crate | License | Source |
|---|---|---|
| ureq (HTTPS downloads) | MIT OR Apache-2.0 | https://github.com/algesten/ureq |
| rustls, rustls-pki-types, rustls-webpki | Apache-2.0 OR ISC OR MIT | https://github.com/rustls/rustls |
| ring (cryptography used by rustls) | Apache-2.0 AND ISC | https://github.com/briansmith/ring |
| webpki-roots (trusted certificate list) | CDLA-Permissive-2.0 | https://github.com/rustls/webpki-roots |
| sha2 (checksums) | MIT OR Apache-2.0 | https://github.com/RustCrypto/hashes |
| flate2, tar (unpacking .tar.gz, macOS and Linux) | MIT OR Apache-2.0 | https://github.com/rust-lang/flate2-rs, https://github.com/alexcrichton/tar-rs |
| zip (unpacking .zip, Windows) | MIT | https://github.com/zip-rs/zip2 |
| self-replace (replacing the running program) | Apache-2.0 | https://github.com/mitsuhiko/self-replace |

These crates also pull in small helper crates. All of them use permissive licenses that
are compatible with MIT: MIT, Apache-2.0, ISC, BSD-3-Clause, Zlib, 0BSD, CC0-1.0 or
CDLA-Permissive-2.0 (where a crate offers a choice, MIT or Apache-2.0 is used). Run
`cargo metadata` for the complete list.

## VS Code extension

The extension has no runtime npm dependencies. Its development tools (TypeScript,
`@types/node`, `@types/vscode`, `@vscode/vsce`, `@vscode/test-electron`) are MIT or
Apache-2.0 licensed and are not included in the packaged extension.
