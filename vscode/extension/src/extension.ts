// The AutoFormat VS Code extension.
//
// This is a thin adapter: it sends the document text to the bundled `autoformat`
// executable and applies the result. All formatting logic lives in the Rust engine.

import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";

import { ensureExecutable, resolveExecutable } from "./binary";
import { autoformatLanguage, formatArguments, SUPPORTED_LANGUAGE_IDS } from "./languages";
import { runAutoformat } from "./runner";

const EXTENSION_ID = "JohnK.autoformat";
const TIMEOUT_MS = 10000;

let output: vscode.OutputChannel;
let statusItem: vscode.StatusBarItem;
let extensionPath = "";
let reportedMissingBinary = false;

export function activate(context: vscode.ExtensionContext): void {
    extensionPath = context.extensionPath;
    output = vscode.window.createOutputChannel("AutoFormat");
    statusItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 0);
    statusItem.command = "autoformat.showOutput";
    context.subscriptions.push(output, statusItem);

    const selector: vscode.DocumentFilter[] = [];
    for (const language of SUPPORTED_LANGUAGE_IDS) {
        selector.push({ language, scheme: "file" });
        selector.push({ language, scheme: "untitled" });
    }

    context.subscriptions.push(
        vscode.languages.registerDocumentFormattingEditProvider(selector, {
            provideDocumentFormattingEdits(document, _options, token) {
                return formatDocument(document, token);
            },
        })
    );

    context.subscriptions.push(
        vscode.workspace.onWillSaveTextDocument((event) => {
            if (shouldFormatOnSave(event.document)) {
                event.waitUntil(formatDocument(event.document, undefined));
            }
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand("autoformat.showOutput", () => output.show())
    );
}

export function deactivate(): void {
    // Nothing to clean up: subscriptions are disposed by VS Code.
}

/**
 * Format on save runs when `autoformat.formatOnSave` is on. It is skipped when VS Code's
 * own format-on-save will already call AutoFormat as the default formatter.
 */
function shouldFormatOnSave(document: vscode.TextDocument): boolean {
    if (autoformatLanguage(document.languageId) === undefined) {
        return false;
    }
    const settings = vscode.workspace.getConfiguration("autoformat", document);
    if (!settings.get<boolean>("formatOnSave", true)) {
        return false;
    }
    const editor = vscode.workspace.getConfiguration("editor", document);
    const vscodeFormatsOnSave = editor.get<boolean>("formatOnSave", false);
    const defaultFormatter = editor.get<string>("defaultFormatter", "");
    return !(vscodeFormatsOnSave && defaultFormatter === EXTENSION_ID);
}

/** Formats a whole document. Returns no edits when the text is already formatted or on errors. */
async function formatDocument(
    document: vscode.TextDocument,
    token: vscode.CancellationToken | undefined
): Promise<vscode.TextEdit[]> {
    const language = autoformatLanguage(document.languageId);
    if (language === undefined) {
        return [];
    }

    const settings = vscode.workspace.getConfiguration("autoformat", document);
    const resolved = resolveExecutable(
        extensionPath,
        settings.get<string>("executablePath", ""),
        process.platform,
        process.arch,
        (file) => fs.existsSync(file)
    );
    if (typeof resolved === "string") {
        reportError(resolved, !reportedMissingBinary);
        reportedMissingBinary = true;
        return [];
    }
    if (resolved.source === "bundled") {
        ensureExecutable(resolved.path, process.platform);
    }

    const original = document.getText();
    let result;
    try {
        result = await runAutoformat(resolved.path, formatArguments(language, documentPath(document)), original, TIMEOUT_MS);
    } catch (error) {
        reportError("Could not run autoformat: " + String(error), false);
        return [];
    }
    if (token !== undefined && token.isCancellationRequested) {
        return [];
    }
    if (result.exitCode !== 0) {
        const message = result.stderr.trim().replace(/^error: /, "");
        reportError(message === "" ? "autoformat failed with exit code " + result.exitCode : message, false);
        return [];
    }

    statusItem.hide();
    const formatted = result.stdout;
    if (formatted === original) {
        return [];
    }
    const edits: vscode.TextEdit[] = [];
    const fullRange = new vscode.Range(document.positionAt(0), document.positionAt(original.length));
    edits.push(vscode.TextEdit.replace(fullRange, formatted));

    // Apply the configured line ending (VS Code keeps the document's own otherwise).
    const wantsCrlf = formatted.includes("\r\n");
    const wantsLf = !wantsCrlf && formatted.includes("\n");
    if (wantsCrlf && document.eol !== vscode.EndOfLine.CRLF) {
        edits.push(vscode.TextEdit.setEndOfLine(vscode.EndOfLine.CRLF));
    } else if (wantsLf && document.eol !== vscode.EndOfLine.LF) {
        edits.push(vscode.TextEdit.setEndOfLine(vscode.EndOfLine.LF));
    }
    return edits;
}

/**
 * The path used to find `autoformat.toml`. Unsaved documents use the first workspace
 * folder, so the project's configuration still applies.
 */
function documentPath(document: vscode.TextDocument): string {
    if (document.uri.scheme === "file") {
        return document.uri.fsPath;
    }
    const folders = vscode.workspace.workspaceFolders;
    if (folders !== undefined && folders.length > 0) {
        return path.join(folders[0].uri.fsPath, "untitled");
    }
    return "untitled";
}

/** Errors go to the output channel and the status bar, so saving never pops up dialogs. */
function reportError(message: string, showDialog: boolean): void {
    output.appendLine(new Date().toISOString() + " " + message);
    statusItem.text = "$(warning) AutoFormat";
    statusItem.tooltip = message;
    statusItem.show();
    if (showDialog) {
        void vscode.window.showErrorMessage(message);
    }
}
