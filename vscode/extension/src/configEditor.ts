// Connects the help for `autoformat.toml` (see configHelp.ts) to VS Code: marks on
// mistakes, suggestions while typing and an explanation when pointing at a setting.

import * as vscode from "vscode";

import {
    findExplanation,
    findMarks,
    findSuggestions,
    isConfigFile,
    Range as HelpRange,
    SUGGESTION_TRIGGERS,
} from "./configHelp";

const TIMEOUT_MS = 5000;

/**
 * Turns the help on. `findExecutable` gives the path of the autoformat program, or
 * undefined when there is none.
 */
export function registerConfigHelp(
    context: vscode.ExtensionContext,
    findExecutable: (document: vscode.TextDocument) => string | undefined
): void {
    // Only files named autoformat.toml. Other TOML files, such as Cargo.toml, get nothing.
    const selector: vscode.DocumentFilter = { scheme: "file", pattern: "**/autoformat.toml" };
    const marks = vscode.languages.createDiagnosticCollection("autoformat");

    async function refreshMarks(document: vscode.TextDocument): Promise<void> {
        if (!isConfigDocument(document)) {
            return;
        }
        const executable = findExecutable(document);
        if (executable === undefined) {
            return;
        }
        const version = document.version;
        let found;
        try {
            found = await findMarks(executable, document.uri.toString(), document.getText(), TIMEOUT_MS);
        } catch {
            return;
        }
        // The user typed on while the program was working. A newer answer is on its way.
        if (document.isClosed || document.version !== version) {
            return;
        }
        const diagnostics = found.map((mark) => {
            const diagnostic = new vscode.Diagnostic(toRange(mark.range), mark.message, vscode.DiagnosticSeverity.Error);
            diagnostic.source = "autoformat";
            return diagnostic;
        });
        marks.set(document.uri, diagnostics);
    }

    for (const document of vscode.workspace.textDocuments) {
        void refreshMarks(document);
    }

    context.subscriptions.push(
        marks,
        vscode.workspace.onDidOpenTextDocument((document) => void refreshMarks(document)),
        vscode.workspace.onDidChangeTextDocument((event) => void refreshMarks(event.document)),
        vscode.workspace.onDidCloseTextDocument((document) => marks.delete(document.uri)),

        vscode.languages.registerCompletionItemProvider(
            selector,
            {
                async provideCompletionItems(document, position) {
                    const executable = findExecutable(document);
                    if (executable === undefined) {
                        return [];
                    }
                    const place = { line: position.line, character: position.character };
                    const found = await findSuggestions(executable, document.uri.toString(), document.getText(), place, TIMEOUT_MS);
                    return found.map((suggestion) => {
                        // VS Code numbers the kinds one lower than the protocol does.
                        const item = new vscode.CompletionItem(suggestion.label, suggestion.kind - 1);
                        item.detail = suggestion.detail;
                        if (suggestion.documentation !== undefined) {
                            item.documentation = new vscode.MarkdownString(suggestion.documentation.value);
                        }
                        item.sortText = suggestion.sortText;
                        item.filterText = suggestion.filterText;
                        item.range = toRange(suggestion.textEdit.range);
                        item.insertText = suggestion.textEdit.newText;
                        // After a setting name and its `=`, show the allowed values right away.
                        if (suggestion.textEdit.newText.endsWith("= ")) {
                            item.command = { command: "editor.action.triggerSuggest", title: "Suggest values" };
                        }
                        return item;
                    });
                },
            },
            ...SUGGESTION_TRIGGERS
        ),

        vscode.languages.registerHoverProvider(selector, {
            async provideHover(document, position) {
                const executable = findExecutable(document);
                if (executable === undefined) {
                    return undefined;
                }
                const place = { line: position.line, character: position.character };
                const found = await findExplanation(executable, document.uri.toString(), document.getText(), place, TIMEOUT_MS);
                if (found === undefined) {
                    return undefined;
                }
                return new vscode.Hover(new vscode.MarkdownString(found.contents.value), toRange(found.range));
            },
        })
    );
}

function isConfigDocument(document: vscode.TextDocument): boolean {
    return document.uri.scheme === "file" && isConfigFile(document.uri.fsPath);
}

function toRange(range: HelpRange): vscode.Range {
    return new vscode.Range(range.start.line, range.start.character, range.end.line, range.end.character);
}
