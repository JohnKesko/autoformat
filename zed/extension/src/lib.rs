//! The AutoFormat extension for the Zed editor.
//!
//! Zed formats through language servers, so this extension does only one thing: it gets
//! the `autoformat` program and tells Zed to run it as `autoformat lsp`. All formatting
//! happens in that program.
//!
//! Zed does not let extensions carry programs with them. The program is downloaded from
//! AutoFormat's GitHub releases the first time it is needed, and again when a newer
//! release exists. Downloads are kept in the folder Zed gives this extension:
//!
//! ```text
//! autoformat-v0.1.3/autoformat      the program
//! autoformat-v0.1.3/downloaded      written last, so a half finished download is never used
//! ```

use std::fs;

use zed_extension_api::{self as zed, LanguageServerId, Result};

/// Where AutoFormat's releases are published.
const REPOSITORY: &str = "JohnKesko/autoformat";

/// Download folders are named `autoformat-<release>`.
const FOLDER_PREFIX: &str = "autoformat-";

/// An empty file written into a download folder after everything else succeeded.
const DOWNLOADED_MARKER: &str = "downloaded";

struct AutoFormatExtension {
    /// The program found earlier, so GitHub is only asked once each time Zed runs.
    program: Option<String>,
}

impl zed::Extension for AutoFormatExtension {
    fn new() -> Self {
        AutoFormatExtension { program: None }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // When people set their own program in Zed's settings ("lsp": { "autoformat":
        // { "binary": { "path": ... } } }), Zed starts that program itself and never
        // asks this extension.
        let command = self.downloaded_program(language_server_id)?;
        Ok(zed::Command {
            command,
            args: vec!["lsp".to_string()],
            env: Vec::new(),
        })
    }
}

impl AutoFormatExtension {
    /// Returns the path of the downloaded program, downloading it first when needed.
    fn downloaded_program(&mut self, language_server_id: &LanguageServerId) -> Result<String> {
        if let Some(program) = &self.program {
            if is_file(program) {
                return Ok(program.clone());
            }
        }

        let (os, architecture) = zed::current_platform();
        let archive = match archive_name(os, architecture) {
            Some(archive) => archive,
            None => {
                return Err(
                    "AutoFormat has no ready-made program for this kind of computer. \
                     Build autoformat yourself and point Zed to it. The AutoFormat \
                     README shows how."
                        .to_string(),
                )
            }
        };
        let program_name = program_name(os);

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let options = zed::GithubReleaseOptions {
            require_assets: true,
            pre_release: false,
        };
        let release = match zed::latest_github_release(REPOSITORY, options) {
            Ok(release) => release,
            Err(error) => {
                // GitHub cannot be reached, for example without an internet connection.
                // Keep working with the program that was downloaded earlier.
                return match find_downloaded_program(program_name) {
                    Some(program) => {
                        self.program = Some(program.clone());
                        Ok(program)
                    }
                    None => Err(format!(
                        "could not find the newest AutoFormat release: {}",
                        error
                    )),
                };
            }
        };

        let folder = format!("{}{}", FOLDER_PREFIX, release.version);
        let program = format!("{}/{}", folder, program_name);
        if !is_downloaded(&folder, program_name) {
            let asset = match release.assets.iter().find(|asset| asset.name == archive) {
                Some(asset) => asset,
                None => {
                    return Err(format!(
                        "AutoFormat release {} has no file named {}",
                        release.version, archive
                    ))
                }
            };
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );
            // Start clean, in case an earlier download stopped half way.
            let _ = fs::remove_dir_all(&folder);
            if let Err(error) = zed::download_file(&asset.download_url, &folder, archive_type(os)) {
                return Err(format!("could not download {}: {}", archive, error));
            }
            zed::make_file_executable(&program)?;
            if let Err(error) = fs::write(format!("{}/{}", folder, DOWNLOADED_MARKER), "") {
                return Err(format!("could not finish the download: {}", error));
            }
            remove_other_downloads(&folder);
        }

        self.program = Some(program.clone());
        Ok(program)
    }
}

/// The name of the release file built for a kind of computer. These are the names the
/// release workflow (`.github/workflows/release.yml`) gives its archives.
fn archive_name(os: zed::Os, architecture: zed::Architecture) -> Option<&'static str> {
    match (os, architecture) {
        (zed::Os::Mac, zed::Architecture::Aarch64) => {
            Some("autoformat-aarch64-apple-darwin.tar.gz")
        }
        (zed::Os::Mac, zed::Architecture::X8664) => Some("autoformat-x86_64-apple-darwin.tar.gz"),
        (zed::Os::Linux, zed::Architecture::Aarch64) => {
            Some("autoformat-aarch64-unknown-linux-musl.tar.gz")
        }
        (zed::Os::Linux, zed::Architecture::X8664) => {
            Some("autoformat-x86_64-unknown-linux-musl.tar.gz")
        }
        (zed::Os::Windows, zed::Architecture::Aarch64) => {
            Some("autoformat-aarch64-pc-windows-msvc.zip")
        }
        (zed::Os::Windows, zed::Architecture::X8664) => {
            Some("autoformat-x86_64-pc-windows-msvc.zip")
        }
        // There are no 32-bit builds.
        (_, zed::Architecture::X86) => None,
    }
}

fn archive_type(os: zed::Os) -> zed::DownloadedFileType {
    match os {
        zed::Os::Windows => zed::DownloadedFileType::Zip,
        zed::Os::Mac | zed::Os::Linux => zed::DownloadedFileType::GzipTar,
    }
}

fn program_name(os: zed::Os) -> &'static str {
    match os {
        zed::Os::Windows => "autoformat.exe",
        zed::Os::Mac | zed::Os::Linux => "autoformat",
    }
}

fn is_file(path: &str) -> bool {
    match fs::metadata(path) {
        Ok(metadata) => metadata.is_file(),
        Err(_) => false,
    }
}

/// A download folder is only used when the program is there and the download finished.
fn is_downloaded(folder: &str, program_name: &str) -> bool {
    is_file(&format!("{}/{}", folder, program_name))
        && is_file(&format!("{}/{}", folder, DOWNLOADED_MARKER))
}

/// The names of all download folders in the extension's own folder.
fn download_folders() -> Vec<String> {
    let mut folders = Vec::new();
    if let Ok(entries) = fs::read_dir(".") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(FOLDER_PREFIX) {
                folders.push(name);
            }
        }
    }
    folders
}

/// A program from an earlier, finished download, if there is one.
fn find_downloaded_program(program_name: &str) -> Option<String> {
    for folder in download_folders() {
        if is_downloaded(&folder, program_name) {
            return Some(format!("{}/{}", folder, program_name));
        }
    }
    None
}

/// Removes older downloads, so they do not pile up.
fn remove_other_downloads(keep: &str) {
    for folder in download_folders() {
        if folder != keep {
            let _ = fs::remove_dir_all(&folder);
        }
    }
}

zed::register_extension!(AutoFormatExtension);

#[cfg(test)]
mod tests {
    use super::*;

    const EVERY_COMPUTER: [(zed::Os, zed::Architecture); 6] = [
        (zed::Os::Mac, zed::Architecture::Aarch64),
        (zed::Os::Mac, zed::Architecture::X8664),
        (zed::Os::Linux, zed::Architecture::Aarch64),
        (zed::Os::Linux, zed::Architecture::X8664),
        (zed::Os::Windows, zed::Architecture::Aarch64),
        (zed::Os::Windows, zed::Architecture::X8664),
    ];

    fn read_repository_file(path: &str) -> String {
        let full = format!("{}/../../{}", env!("CARGO_MANIFEST_DIR"), path);
        match fs::read_to_string(&full) {
            Ok(text) => text,
            Err(error) => panic!("cannot read {}: {}", full, error),
        }
    }

    /// The extension asks for the same file names that the release workflow publishes
    /// and that `autoformat upgrade` downloads.
    #[test]
    fn asks_for_files_the_release_workflow_publishes() {
        let workflow = read_repository_file(".github/workflows/release.yml");
        let upgrade = read_repository_file("crates/autoformat-cli/src/upgrade.rs");
        let mut names = Vec::new();
        for (os, architecture) in EVERY_COMPUTER {
            let name = archive_name(os, architecture).unwrap();
            let extension = match os {
                zed::Os::Windows => ".zip",
                zed::Os::Mac | zed::Os::Linux => ".tar.gz",
            };
            let target = name
                .strip_prefix("autoformat-")
                .unwrap()
                .strip_suffix(extension)
                .unwrap();
            let built = format!("rust_target: {},", target);
            assert!(
                workflow.contains(&built),
                "{} is not built by release.yml",
                target
            );
            assert!(
                upgrade.contains(name),
                "{} is not known to autoformat upgrade",
                name
            );
            assert!(!names.contains(&name), "{} is used twice", name);
            names.push(name);
        }
    }

    #[test]
    fn there_is_no_32_bit_program() {
        for os in [zed::Os::Mac, zed::Os::Linux, zed::Os::Windows] {
            assert_eq!(archive_name(os, zed::Architecture::X86), None);
        }
    }

    #[test]
    fn windows_uses_zip_and_exe() {
        assert_eq!(program_name(zed::Os::Windows), "autoformat.exe");
        assert_eq!(program_name(zed::Os::Mac), "autoformat");
        assert_eq!(program_name(zed::Os::Linux), "autoformat");
        assert!(matches!(
            archive_type(zed::Os::Windows),
            zed::DownloadedFileType::Zip
        ));
        assert!(matches!(
            archive_type(zed::Os::Mac),
            zed::DownloadedFileType::GzipTar
        ));
        assert!(matches!(
            archive_type(zed::Os::Linux),
            zed::DownloadedFileType::GzipTar
        ));
    }

    /// `extension.toml` and this file must agree on the server's name, and every language
    /// must have a name that autoformat understands.
    #[test]
    fn the_manifest_matches_the_code() {
        let manifest = read_repository_file("zed/extension/extension.toml");
        // This is the name people write in their Zed settings, so it must never change.
        assert!(manifest.contains("[language_servers.autoformat]"));
        let languages_line = manifest
            .lines()
            .find(|line| line.starts_with("languages = "))
            .unwrap();
        let lsp = read_repository_file("crates/autoformat-cli/src/lsp.rs");
        let mut count = 0;
        for line in manifest.lines() {
            // Lines such as: "TSX" = "typescriptreact"
            if !line.starts_with('"') {
                continue;
            }
            let parts: Vec<&str> = line.split('"').collect();
            let zed_language = parts[1];
            let language_id = parts[3];
            assert!(
                languages_line.contains(&format!("\"{}\"", zed_language)),
                "{} has a name but is not in `languages`",
                zed_language
            );
            assert!(
                lsp.contains(&format!("\"{}\"", language_id)),
                "autoformat lsp does not know the name {}",
                language_id
            );
            count += 1;
        }
        assert_eq!(count, languages_line.matches('"').count() / 2);
    }

    #[test]
    fn the_version_is_the_same_in_both_files() {
        let manifest = read_repository_file("zed/extension/extension.toml");
        let version_line = format!("version = \"{}\"", env!("CARGO_PKG_VERSION"));
        assert!(
            manifest.contains(&version_line),
            "extension.toml and Cargo.toml disagree"
        );
    }
}
