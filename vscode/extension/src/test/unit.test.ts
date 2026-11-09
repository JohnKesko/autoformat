// Unit tests that run with plain Node.js (`npm run test:unit`), without VS Code.

import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import { test } from "node:test";

import { executableName, platformFolder, resolveExecutable } from "../binary";
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
