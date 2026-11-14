// Maps VS Code language identifiers to AutoFormat language names.

export const LANGUAGE_NAMES: { [languageId: string]: string } = {
    rust: "rust",
    toml: "toml",
    json: "json",
    jsonc: "json",
    javascript: "javascript",
    javascriptreact: "javascript",
    typescript: "typescript",
    typescriptreact: "tsx",
    csharp: "csharp",
    java: "java",
    kotlin: "kotlin",
    powershell: "powershell",
};

export const SUPPORTED_LANGUAGE_IDS: string[] = Object.keys(LANGUAGE_NAMES);

export function autoformatLanguage(languageId: string): string | undefined {
    return LANGUAGE_NAMES[languageId];
}

/** Builds the command line arguments for formatting one document. */
export function formatArguments(language: string, filePath: string): string[] {
    return ["format", "--stdin", "--language", language, "--stdin-filepath", filePath];
}
