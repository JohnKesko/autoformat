// Unit tests that run with plain Node.js (`npm run test:unit`), without VS Code.

import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import { test } from "node:test";

import { executableName, platformFolder, resolveExecutable } from "../binary";
import {
    converse,
    findExplanation,
    findMarks,
    findSuggestions,
    frame,
    isConfigFile,
    SUGGESTION_TRIGGERS,
    unframe,
} from "../configHelp";
import { autoformatLanguage, formatArguments } from "../languages";
import { runAutoformat } from "../runner";

test("bundled binaries are found per platform", () => {
    const found = resolveExecutable("/ext", "", "linux", "x64", (file) => file === path.join("/ext", "bin", "linux-x64", "autoformat"));
    assert.deepStrictEqual(found, { path: path.join("/ext", "bin", "linux-x64", "autoformat"), source: "bundled" });
    assert.strictEqual(executableName("win32"), "autoformat.exe");
    assert.strictEqual(platformFolder("darwin", "arm64"), "darwin-arm64");
});

test("a configured executable wins over the bundled one", () => {
    const found = resolveExecutable("/ext", "/custom/autoformat", "linux", "x64", () => true);
    assert.deepStrictEqual(found, { path: "/custom/autoformat", source: "setting" });
});

test("missing binaries give a clear message", () => {
    const found = resolveExecutable("/ext", "", "sunos", "x64", () => false);
    assert.strictEqual(typeof found, "string");
    assert.ok(String(found).includes("sunos-x64"));
    const missingSetting = resolveExecutable("/ext", "/nope", "linux", "x64", () => false);
    assert.ok(String(missingSetting).includes("/nope"));
});

test("VS Code languages map to AutoFormat languages", () => {
    assert.strictEqual(autoformatLanguage("typescriptreact"), "tsx");
    assert.strictEqual(autoformatLanguage("javascriptreact"), "javascript");
    assert.strictEqual(autoformatLanguage("csharp"), "csharp");
    assert.strictEqual(autoformatLanguage("java"), "java");
    assert.strictEqual(autoformatLanguage("kotlin"), "kotlin");
    assert.strictEqual(autoformatLanguage("json"), "json");
    assert.strictEqual(autoformatLanguage("jsonc"), "json");
    assert.strictEqual(autoformatLanguage("markdown"), undefined);
    assert.deepStrictEqual(formatArguments("rust", "/a/b.rs"), ["format", "--stdin", "--language", "rust", "--stdin-filepath", "/a/b.rs"]);
});

/** The release or debug build of the formatter, if one exists. */
function localBinary(): string | undefined {
    const root = path.resolve(__dirname, "..", "..", "..", "..");
    const name = executableName(process.platform);
    for (const profile of ["release", "debug"]) {
        const candidate = path.join(root, "target", profile, name);
        if (fs.existsSync(candidate)) {
            return candidate;
        }
    }
    return undefined;
}

test("the real formatter formats text through standard input", async (context) => {
    const binary = localBinary();
    if (binary === undefined) {
        context.skip("build the Rust workspace first");
        return;
    }
    const result = await runAutoformat(binary, formatArguments("rust", "/tmp/x.rs"), "fn main(){let x=1;}\n", 10000);
    assert.strictEqual(result.exitCode, 0, result.stderr);
    assert.strictEqual(result.stdout, "fn main() { let x = 1; }\n");

    const broken = await runAutoformat(binary, formatArguments("rust", "/tmp/x.rs"), "fn main( {\n", 10000);
    assert.strictEqual(broken.exitCode, 1);
    assert.strictEqual(broken.stdout, "");
    assert.ok(broken.stderr.includes("syntax error"));
});

test("only files named autoformat.toml get help", () => {
    assert.strictEqual(isConfigFile(path.join("project", "autoformat.toml")), true);
    assert.strictEqual(isConfigFile("autoformat.toml"), true);
    assert.strictEqual(isConfigFile(path.join("project", "Cargo.toml")), false);
    assert.strictEqual(isConfigFile(path.join("project", "my-autoformat.toml")), false);
});

test("messages for the server are wrapped and unwrapped by their length in bytes", () => {
    const messages = [{ id: 1, method: "a", params: { text: "å∫😀" } }, { method: "exit" }];
    const framed = frame(messages);
    // å is two bytes, ∫ three and 😀 four, so the length is larger than the number of characters.
    const body = JSON.stringify({ jsonrpc: "2.0", ...messages[0] });
    assert.ok(framed.startsWith("Content-Length: " + (body.length + 5) + "\r\n\r\n" + body));
    assert.deepStrictEqual(unframe(framed), [
        { jsonrpc: "2.0", id: 1, method: "a", params: { text: "å∫😀" } },
        { jsonrpc: "2.0", method: "exit" },
    ]);
    assert.deepStrictEqual(unframe(""), []);
    assert.deepStrictEqual(unframe("not from the server"), []);
});

// A half-written file: a number that is too large, a value that is not typed yet, and the
// start of a setting name.
const HALF_WRITTEN = "[formatting]\nindent_size = 99\nbrace_style = \nco";
const CONFIG_URI = "file:///project/autoformat.toml";

test("the real program marks the mistakes in a half-written autoformat.toml", async (context) => {
    const binary = localBinary();
    if (binary === undefined) {
        context.skip("build the Rust workspace first");
        return;
    }
    const marks = await findMarks(binary, CONFIG_URI, HALF_WRITTEN, 10000);
    assert.strictEqual(marks.length, 3);
    assert.strictEqual(marks[0].message, "`indent_size` must be a whole number from 1 to 16, found 99");
    assert.deepStrictEqual(marks[0].range, { start: { line: 1, character: 14 }, end: { line: 1, character: 16 } });

    // The marks go away when the mistakes do.
    assert.deepStrictEqual(await findMarks(binary, CONFIG_URI, "[formatting]\nindent_size = 9\n", 10000), []);
});

test("the real program suggests settings, values and table names", async (context) => {
    const binary = localBinary();
    if (binary === undefined) {
        context.skip("build the Rust workspace first");
        return;
    }
    const values = await findSuggestions(binary, CONFIG_URI, HALF_WRITTEN, { line: 2, character: 14 }, 10000);
    assert.deepStrictEqual(
        values.map((value) => value.label),
        ['"same_line"', '"allman"', '"gnu"']
    );
    assert.deepStrictEqual(values[1].textEdit, {
        range: { start: { line: 2, character: 14 }, end: { line: 2, character: 14 } },
        newText: '"allman"',
    });

    const names = (await findSuggestions(binary, CONFIG_URI, HALF_WRITTEN, { line: 3, character: 2 }, 10000)).map((name) => name.label);
    assert.ok(names.includes("column_width"));
    assert.ok(names.includes("continuation_indent"));
    // These two are already written in the table.
    assert.ok(!names.includes("indent_size"));
    assert.ok(!names.includes("brace_style"));

    const tables = await findSuggestions(binary, CONFIG_URI, "[formatting]\n\n[formatting.", { line: 2, character: 12 }, 10000);
    assert.ok(tables.some((table) => table.label === "formatting.csharp"));
    assert.ok(!tables.some((table) => table.label === "formatting"));
});

test("the real program explains a setting", async (context) => {
    const binary = localBinary();
    if (binary === undefined) {
        context.skip("build the Rust workspace first");
        return;
    }
    const explanation = await findExplanation(binary, CONFIG_URI, HALF_WRITTEN, { line: 1, character: 3 }, 10000);
    assert.ok(explanation !== undefined);
    assert.strictEqual(
        explanation.contents.value,
        "**indent_size**\n\nHow deep one indentation step is.\n\nAllowed: a whole number from 1 to 16. Default: 4."
    );
    assert.deepStrictEqual(explanation.range, { start: { line: 1, character: 0 }, end: { line: 1, character: 11 } });
    // The first line is `[formatting]`, which is not a setting.
    assert.strictEqual(await findExplanation(binary, CONFIG_URI, HALF_WRITTEN, { line: 0, character: 3 }, 10000), undefined);
});

test("other TOML files get no marks, suggestions or explanations from the real program", async (context) => {
    const binary = localBinary();
    if (binary === undefined) {
        context.skip("build the Rust workspace first");
        return;
    }
    const uri = "file:///project/Cargo.toml";
    assert.deepStrictEqual(await findMarks(binary, uri, HALF_WRITTEN, 10000), []);
    assert.deepStrictEqual(await findSuggestions(binary, uri, HALF_WRITTEN, { line: 3, character: 2 }, 10000), []);
    assert.strictEqual(await findExplanation(binary, uri, HALF_WRITTEN, { line: 1, character: 3 }, 10000), undefined);
    // Only the answers to the question and to the goodbye come back.
    const answers = await converse(binary, uri, HALF_WRITTEN, { method: "textDocument/hover", position: { line: 1, character: 3 } }, 10000);
    assert.deepStrictEqual(
        answers.map((answer) => answer.id),
        [1, 2]
    );
});

test("VS Code asks for suggestions after the same characters as the real program", async (context) => {
    const binary = localBinary();
    if (binary === undefined) {
        context.skip("build the Rust workspace first");
        return;
    }
    const result = await runAutoformat(binary, ["lsp"], frame([{ id: 1, method: "initialize", params: {} }]), 10000);
    const capabilities = unframe(result.stdout)[0].result.capabilities;
    assert.deepStrictEqual(capabilities.completionProvider.triggerCharacters, SUGGESTION_TRIGGERS);
});

/** Every text pattern in the colour rules, with the name of the rule it belongs to. */
function colourPatterns(rule: any, name: string, found: [string, string][]): void {
    for (const key of ["match", "begin", "end"]) {
        if (typeof rule[key] === "string") {
            found.push([name, rule[key]]);
        }
    }
    for (const inner of rule.patterns || []) {
        colourPatterns(inner, name, found);
    }
}

test("the TOML colour rules are registered and every pattern in them is valid", () => {
    const extensionRoot = path.resolve(__dirname, "..", "..");
    const manifest = JSON.parse(fs.readFileSync(path.join(extensionRoot, "package.json"), "utf8"));
    assert.deepStrictEqual(manifest.contributes.grammars, [
        { language: "toml", scopeName: "source.toml", path: "./syntaxes/toml.tmLanguage.json" },
    ]);
    const grammar = JSON.parse(fs.readFileSync(path.join(extensionRoot, "syntaxes", "toml.tmLanguage.json"), "utf8"));
    assert.strictEqual(grammar.scopeName, "source.toml");

    const patterns: [string, string][] = [];
    for (const name of Object.keys(grammar.repository)) {
        colourPatterns(grammar.repository[name], name, patterns);
    }
    assert.ok(patterns.length >= 15);
    for (const [name, pattern] of patterns) {
        assert.doesNotThrow(() => new RegExp(pattern), name + ": " + pattern);
    }
    // Every rule that another rule includes exists.
    const text = JSON.stringify(grammar);
    for (const include of text.match(/"include":"#[^"]+"/g) || []) {
        const name = include.slice('"include":"#'.length, -1);
        assert.ok(grammar.repository[name] !== undefined, name);
    }

    // A few spot checks of what the patterns pick up.
    const pattern = (name: string) => new RegExp(grammar.repository[name].match);
    assert.strictEqual(pattern("table").exec("[formatting.csharp] # C#")![2], "formatting.csharp");
    assert.strictEqual(pattern("key").exec('brace_style = "allman"')![1], "brace_style");
    assert.strictEqual(pattern("key").exec('"my key".sub = 1')![1], '"my key".sub');
    assert.strictEqual(pattern("comment").exec("x = 1 # note")![0], "# note");
    assert.strictEqual(pattern("boolean").exec("x = true")![0], "true");
    for (const number of ["140", "-4", "1_000", "0x1F", "3.14", "1e-9", "inf"]) {
        assert.strictEqual(pattern("number").exec("x = " + number)![0], number);
    }
    assert.strictEqual(pattern("number").exec("name2 = abc3"), null);
    assert.strictEqual(pattern("date").exec("d = 2026-11-14T10:30:00Z")![0], "2026-11-14T10:30:00Z");
});
