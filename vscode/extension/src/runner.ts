// Runs the autoformat command line tool and collects its output.

import { spawn } from "child_process";

export interface RunResult {
    exitCode: number;
    stdout: string;
    stderr: string;
}

/** Runs `executable` with `args`, writes `input` to its standard input and waits. */
export function runAutoformat(executable: string, args: string[], input: string, timeoutMs: number): Promise<RunResult> {
    return new Promise((resolve, reject) => {
        const child = spawn(executable, args, { stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
        const stdout: Buffer[] = [];
        const stderr: Buffer[] = [];
        let finished = false;

        const timer = setTimeout(() => {
            if (!finished) {
                finished = true;
                child.kill();
                reject(new Error("autoformat did not finish within " + timeoutMs + " ms"));
            }
        }, timeoutMs);

        child.stdout.on("data", (chunk: Buffer) => stdout.push(chunk));
        child.stderr.on("data", (chunk: Buffer) => stderr.push(chunk));
        child.on("error", (error) => {
            if (!finished) {
                finished = true;
                clearTimeout(timer);
                reject(error);
            }
        });
        child.on("close", (code) => {
            if (!finished) {
                finished = true;
                clearTimeout(timer);
                resolve({
                    exitCode: code === null ? -1 : code,
                    stdout: Buffer.concat(stdout).toString("utf8"),
                    stderr: Buffer.concat(stderr).toString("utf8"),
                });
            }
        });
        child.stdin.on("error", () => {
            // The process may exit before reading everything (for example on a usage error).
        });
        child.stdin.end(input, "utf8");
    });
}
