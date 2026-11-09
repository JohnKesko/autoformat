// Starts a real, isolated VS Code instance with the extension and runs `suite.ts` in it.
//
// Uses the VS Code installation given by VSCODE_EXECUTABLE, or the standard macOS
// location. If neither exists, @vscode/test-electron downloads a test copy of VS Code.
// A fresh user-data and extensions directory is used, so the developer's own VS Code
// setup is never touched.

import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import { runTests } from "@vscode/test-electron";

import { executableName, platformFolder } from "../binary";

/** Copies the locally built formatter into `bin/` so the extension can find it. */
function stageBinary(extensionRoot: string): void {
    const name = executableName(process.platform);
    const target = path.join(extensionRoot, "bin", platformFolder(process.platform, process.arch), name);
    const built = path.resolve(extensionRoot, "..", "..", "target", "release", name);
    if (!fs.existsSync(built)) {
        throw new Error("Build the formatter first: cargo build --release (missing " + built + ")");
    }
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.copyFileSync(built, target);
    fs.chmodSync(target, 0o755);
}

function findVsCode(): string | undefined {
    if (process.env.VSCODE_EXECUTABLE) {
        return process.env.VSCODE_EXECUTABLE;
    }
    const mac = "/Applications/Visual Studio Code.app/Contents/MacOS/Code";
    if (fs.existsSync(mac)) {
        return mac;
    }
    return undefined;
}

async function main(): Promise<void> {
    const extensionDevelopmentPath = path.resolve(__dirname, "..", "..");
    const extensionTestsPath = path.resolve(__dirname, "suite");
    stageBinary(extensionDevelopmentPath);

    const temp = fs.mkdtempSync(path.join(os.tmpdir(), "autoformat-vscode-"));
    const workspace = path.join(temp, "workspace");
    fs.mkdirSync(workspace);
    fs.writeFileSync(path.join(workspace, "autoformat.toml"), "[formatting]\nindent_size = 2\n");

    await runTests({
        vscodeExecutablePath: findVsCode(),
        extensionDevelopmentPath,
        extensionTestsPath,
        launchArgs: [
            workspace,
            "--disable-extensions",
            "--user-data-dir",
            path.join(temp, "user-data"),
            "--extensions-dir",
            path.join(temp, "extensions"),
            "--skip-welcome",
            "--skip-release-notes",
            "--disable-workspace-trust",
        ],
    });
}

main().catch((error) => {
    console.error(error);
    process.exit(1);
});
