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
        ["a.json", '{"a":[1,2]}\n', '{ "a": [1, 2] }\n'],
        ["a.jsonc", '// note\n{"a":1}\n', '// note\n{ "a": 1 }\n'],
        ["a.js", "let a=1\n", "let a = 1\n"],
        ["a.jsx", "const a=<b>hi</b>\n", "const a = <b>hi</b>\n"],
        ["a.ts", "let a:number=1\n", "let a: number = 1\n"],
        ["a.tsx", "const a=<b>hi</b>\n", "const a = <b>hi</b>\n"],
        ["a.cs", "class A{int x=1;}\n", "class A { int x = 1; }\n"],
        ["a.java", "class A{int x=1;}\n", "class A { int x = 1; }\n"],
        ["a.kt", "class A{val x=1}\n", "class A { val x = 1 }\n"],
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

/** Asks again and again until `check` gives a value, for at most ten seconds. */
async function waitFor<T>(what: string, check: () => T | undefined): Promise<T> {
    for (let attempt = 0; attempt < 100; attempt++) {
        const value = check();
        if (value !== undefined) {
            return value;
        }
        await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error("waited too long for " + what);
}

async function suggestedLabels(uri: vscode.Uri, line: number, character: number): Promise<string[]> {
    const list = await vscode.commands.executeCommand<vscode.CompletionList>(
        "vscode.executeCompletionItemProvider",
        uri,
        new vscode.Position(line, character)
    );
    return list.items.map((item) => (typeof item.label === "string" ? item.label : item.label.label));
}

async function testConfigHelp(folder: string): Promise<void> {
    // Its own folder, so the half-written file does not disturb the other tests.
    const helpFolder = path.join(folder, "help");
    fs.mkdirSync(helpFolder, { recursive: true });
    const halfWritten = "[formatting]\nindent_size = 99\nbrace_style = \nco";
    const file = path.join(helpFolder, "autoformat.toml");
    fs.writeFileSync(file, halfWritten);
    const document = await vscode.workspace.openTextDocument(file);
    const editor = await vscode.window.showTextDocument(document);
    assert.strictEqual(document.languageId, "toml");

    // Marks: on the number that is too large, and on the two unfinished places.
    const marks = await waitFor("the marks", () => {
        const found = vscode.languages.getDiagnostics(document.uri);
        return found.length > 0 ? found : undefined;
    });
    assert.strictEqual(marks.length, 3);
    assert.strictEqual(marks[0].message, "`indent_size` must be a whole number from 1 to 16, found 99");
    assert.ok(marks[0].range.isEqual(new vscode.Range(1, 14, 1, 16)));
    assert.strictEqual(marks[0].severity, vscode.DiagnosticSeverity.Error);

    // Suggestions: the values after `brace_style = `, and the settings that are left.
    assert.deepStrictEqual(await suggestedLabels(document.uri, 2, 14), ['"same_line"', '"allman"', '"gnu"']);
    const names = await suggestedLabels(document.uri, 3, 2);
    assert.ok(names.includes("column_width"), names.join(", "));
    assert.ok(!names.includes("indent_size"), names.join(", "));

    // An explanation when pointing at `indent_size`.
    const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
        "vscode.executeHoverProvider",
        document.uri,
        new vscode.Position(1, 3)
    );
    assert.strictEqual(hovers.length, 1);
    const explanation = (hovers[0].contents[0] as vscode.MarkdownString).value;
    assert.ok(explanation.startsWith("**indent_size**"), explanation);
    assert.ok(explanation.includes("Default: 4."), explanation);

    // The marks go away while typing, without saving.
    const everything = new vscode.Range(document.positionAt(0), document.positionAt(halfWritten.length));
    await editor.edit((builder) => builder.replace(everything, "[formatting]\nindent_size = 9\n"));
    await waitFor("the marks to go away", () => (vscode.languages.getDiagnostics(document.uri).length === 0 ? true : undefined));

    // Another TOML file with the same text gets none of this.
    const other = path.join(helpFolder, "Cargo.toml");
    fs.writeFileSync(other, halfWritten);
    const otherDocument = await vscode.workspace.openTextDocument(other);
    await vscode.window.showTextDocument(otherDocument);
    await new Promise((resolve) => setTimeout(resolve, 1000));
    assert.strictEqual(vscode.languages.getDiagnostics(otherDocument.uri).length, 0);
    const otherHovers = await vscode.commands.executeCommand<vscode.Hover[]>(
        "vscode.executeHoverProvider",
        otherDocument.uri,
        new vscode.Position(1, 3)
    );
    assert.strictEqual(otherHovers.length, 0);
    // `continuation_indent` is not a word in any open file, so only AutoFormat could suggest it.
    assert.ok(!(await suggestedLabels(otherDocument.uri, 3, 2)).includes("continuation_indent"));
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
        ["help while editing autoformat.toml", testConfigHelp],
    ];
    for (const [name, body] of tests) {
        await body(folder);
        console.log("passed: " + name);
    }
}
