// Finds the autoformat executable that ships inside the extension.
//
// Binaries live in `bin/<platform>-<arch>/`, for example `bin/darwin-arm64/autoformat`
// or `bin/win32-x64/autoformat.exe`. Nothing is ever downloaded.

import * as fs from "fs";
import * as path from "path";

export interface ResolvedExecutable {
    path: string;
    source: "setting" | "bundled";
}

/** The folder name used for the current platform, for example `linux-x64`. */
export function platformFolder(platform: string, arch: string): string {
    return platform + "-" + arch;
}

export function executableName(platform: string): string {
    return platform === "win32" ? "autoformat.exe" : "autoformat";
}

/**
 * Picks the executable to run. A path set by the user wins; otherwise the bundled
 * binary for this platform is used. Returns an error message when nothing is found.
 */
export function resolveExecutable(
    extensionPath: string,
    configuredPath: string,
    platform: string,
    arch: string,
    fileExists: (file: string) => boolean
): ResolvedExecutable | string {
    if (configuredPath.trim() !== "") {
        if (fileExists(configuredPath)) {
            return { path: configuredPath, source: "setting" };
        }
        return "The autoformat.executablePath setting points to a file that does not exist: " + configuredPath;
    }
    const bundled = path.join(extensionPath, "bin", platformFolder(platform, arch), executableName(platform));
    if (fileExists(bundled)) {
        return { path: bundled, source: "bundled" };
    }
    return (
        "AutoFormat does not include a formatter for this platform (" +
            platformFolder(platform, arch) +
            "). Set autoformat.executablePath to an autoformat executable."
    );
}

/** Makes sure a bundled binary can be executed (zip files do not always keep the flag). */
export function ensureExecutable(file: string, platform: string): void {
    if (platform === "win32") {
        return;
    }
    try {
        fs.accessSync(file, fs.constants.X_OK);
    } catch {
        fs.chmodSync(file, 0o755);
    }
}
