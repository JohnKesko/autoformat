// Help while `autoformat.toml` is edited: marks on mistakes, suggestions while typing and
// an explanation for each setting.
//
// This file does none of the thinking. For every question it starts `autoformat lsp` (the
// small server that Zed also uses), sends the text of the file and the question, and
// reads the answer. The program is the only place that knows the settings, so VS Code and
// Zed always say the same thing, and no other extension or library is needed.
//
// The program is started again for every question. That takes a few hundredths of a
// second, and it means there is no long-running process to start, watch and restart.

import * as path from "path";

import { runAutoformat } from "./runner";

/** Only files with this exact name get help. */
export const CONFIG_FILE_NAME = "autoformat.toml";

/** The characters that make VS Code ask for suggestions right away. */
export const SUGGESTION_TRIGGERS: string[] = ["[", ".", "=", '"', "'"];

export interface Position {
    line: number;
    character: number;
}

export interface Range {
    start: Position;
    end: Position;
}

/** A mistake and where it is. */
export interface Mark {
    range: Range;
    message: string;
}

export interface Suggestion {
    label: string;
    /** The protocol's number for the kind of suggestion (table, setting or value). */
    kind: number;
    detail?: string;
    documentation?: { value: string };
    sortText?: string;
    filterText?: string;
    /** What picking the suggestion writes, and which text it replaces. */
    textEdit: { range: Range; newText: string };
}

export interface Explanation {
    contents: { value: string };
    range: Range;
}

/** One message to or from `autoformat lsp`. */
export interface ServerMessage {
    id?: number;
    method?: string;
    params?: any;
    result?: any;
}

export function isConfigFile(filePath: string): boolean {
    return path.basename(filePath) === CONFIG_FILE_NAME;
}

/** Writes messages the way the server reads them: each one after a `Content-Length` line. */
export function frame(messages: ServerMessage[]): string {
    let text = "";
    for (const message of messages) {
        const body = JSON.stringify({ jsonrpc: "2.0", ...message });
        text += "Content-Length: " + Buffer.byteLength(body, "utf8") + "\r\n\r\n" + body;
    }
    return text;
}

/** Splits what the server wrote back into messages. */
export function unframe(output: string): ServerMessage[] {
    // The lengths are counted in bytes, not in characters.
    const bytes = Buffer.from(output, "utf8");
    const messages: ServerMessage[] = [];
    let position = 0;
    while (position < bytes.length) {
        const headerEnd = bytes.indexOf("\r\n\r\n", position);
        if (headerEnd < 0) {
            break;
        }
        const header = bytes.toString("utf8", position, headerEnd);
        const match = /Content-Length: (\d+)/i.exec(header);
        if (match === null) {
            break;
        }
        const bodyStart = headerEnd + 4;
        const bodyEnd = bodyStart + Number(match[1]);
        messages.push(JSON.parse(bytes.toString("utf8", bodyStart, bodyEnd)));
        position = bodyEnd;
    }
    return messages;
}

/**
 * One short conversation with `autoformat lsp`: open the file, ask one question (or none),
 * say goodbye. Returns everything the server said.
 */
export async function converse(
    executable: string,
    uri: string,
    text: string,
    question: { method: string; position: Position } | undefined,
    timeoutMs: number
): Promise<ServerMessage[]> {
    const messages: ServerMessage[] = [
        { method: "textDocument/didOpen", params: { textDocument: { uri, languageId: "toml", version: 1, text } } },
    ];
    if (question !== undefined) {
        messages.push({ id: 1, method: question.method, params: { textDocument: { uri }, position: question.position } });
    }
    messages.push({ id: 2, method: "shutdown" });
    messages.push({ method: "exit" });
    const result = await runAutoformat(executable, ["lsp"], frame(messages), timeoutMs);
    return unframe(result.stdout);
}

/** The mistakes in the text of an `autoformat.toml`. */
export async function findMarks(executable: string, uri: string, text: string, timeoutMs: number): Promise<Mark[]> {
    const answers = await converse(executable, uri, text, undefined, timeoutMs);
    for (const answer of answers) {
        if (answer.method === "textDocument/publishDiagnostics") {
            return answer.params.diagnostics;
        }
    }
    return [];
}

/** What to suggest at this place in an `autoformat.toml`. */
export async function findSuggestions(
    executable: string,
    uri: string,
    text: string,
    position: Position,
    timeoutMs: number
): Promise<Suggestion[]> {
    const answers = await converse(executable, uri, text, { method: "textDocument/completion", position }, timeoutMs);
    const answer = answers.find((message) => message.id === 1);
    return answer !== undefined && Array.isArray(answer.result) ? answer.result : [];
}

/** The explanation of the setting at this place, if the place is on a setting name. */
export async function findExplanation(
    executable: string,
    uri: string,
    text: string,
    position: Position,
    timeoutMs: number
): Promise<Explanation | undefined> {
    const answers = await converse(executable, uri, text, { method: "textDocument/hover", position }, timeoutMs);
    const answer = answers.find((message) => message.id === 1);
    return answer !== undefined && answer.result ? answer.result : undefined;
}
