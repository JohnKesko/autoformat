// Integration tests that run inside VS Code (started by runVsCodeTests.ts).

import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";

async function formatWithProvider(document: vscode.TextDocument): Promise<void> {
    const edits = await vscode.commands.executeCommand<vscode.TextEdit[]>(
        "vscode.executeFormatDocumentProvider",
        document.uri,
        { tabSize: 4, insertSpaces: true }
    );
    const workspaceEdit = new vscode.WorkspaceEdit();
    workspaceEdit.set(document.uri, edits || []);
    await vscode.workspace.applyEdit(workspaceEdit);
}

async function testFormatDocument(folder: string): Promise<void> {
    const file = path.join(folder, "main.rs");
    fs.writeFileSync(file, "fn main(){\nlet x=1;\n}\n");
    const document = await vscode.workspace.openTextDocument(file);
    await vscode.window.showTextDocument(document);
    await formatWithProvider(document);
    // indent_size = 2 comes from the workspace's autoformat.toml.
    assert.strictEqual(document.getText(), "fn main() {\n  let x = 1;\n}\n");
}

async function testEveryLanguage(folder: string): Promise<void> {
    const samples: [string, string, string][] = [
        ["a.toml", "x=1\n", "x = 1\n"],
        ["a.js", "let a=1\n", "let a = 1\n"],
        ["a.jsx", "const a=<b>hi</b>\n", "const a = <b>hi</b>\n"],
        ["a.ts", "let a:number=1\n", "let a: number = 1\n"],
        ["a.tsx", "const a=<b>hi</b>\n", "const a = <b>hi</b>\n"],
        ["a.cs", "class A{int x=1;}\n", "class A { int x = 1; }\n"],
        ["a.ps1", "if($a){$b=1}\n", "if ($a) { $b = 1 }\n"],
    ];
    for (const [name, input, expected] of samples) {
        const file = path.join(folder, name);
        fs.writeFileSync(file, input);
        const document = await vscode.workspace.openTextDocument(file);
        if (document.languageId === "plaintext") {
            // VS Code has no built-in language for this extension (TOML without an extension).
            continue;
        }
        await formatWithProvider(document);
        assert.strictEqual(document.getText(), expected, name + " (" + document.languageId + ")");
    }
}

async function testFormatOnSave(folder: string): Promise<void> {
    const file = path.join(folder, "save.ts");
    fs.writeFileSync(file, "let a=1\n");
    const document = await vscode.workspace.openTextDocument(file);
    const editor = await vscode.window.showTextDocument(document);
    await editor.edit((builder) => builder.insert(new vscode.Position(1, 0), "let b=2\n"));
    await document.save();
    assert.strictEqual(fs.readFileSync(file, "utf8"), "let a = 1\nlet b = 2\n");
}

async function testSyntaxErrorsChangeNothing(folder: string): Promise<void> {
    const file = path.join(folder, "broken.rs");
    fs.writeFileSync(file, "fn main( {\n");
    const document = await vscode.workspace.openTextDocument(file);
    await formatWithProvider(document);
    assert.strictEqual(document.getText(), "fn main( {\n");
}

export async function run(): Promise<void> {
    const folders = vscode.workspace.workspaceFolders;
    assert.ok(folders && folders.length > 0, "a workspace folder is open");
    const folder = folders[0].uri.fsPath;

    const extension = vscode.extensions.getExtension("JohnK.autoformat");
    assert.ok(extension, "the extension is installed");
    await extension.activate();

    const tests: [string, (folder: string) => Promise<void>][] = [
        ["format document", testFormatDocument],
        ["every language", testEveryLanguage],
        ["format on save", testFormatOnSave],
        ["syntax errors change nothing", testSyntaxErrorsChangeNothing],
    ];
    for (const [name, body] of tests) {
        await body(folder);
        console.log("passed: " + name);
    }
}
