//! `autoformat upgrade`: replaces the installed program with the newest release.
//!
//! How it works:
//! 1. Find the newest release by following GitHub's `.../releases/latest` link. This needs
//!    no API token and is not rate limited.
//! 2. If it is newer than this program, ask first (unless `--yes`), then download the
//!    archive for this computer and the release's `SHA256SUMS.txt`.
//! 3. Check the archive against its published checksum. Nothing is installed if it differs.
//! 4. Unpack the program and swap it in for the running one.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha256};

/// Where AutoFormat's releases are published.
const RELEASES_URL: &str = "https://github.com/JohnKesko/autoformat/releases";

/// Downloads larger than this are refused.
const MAX_DOWNLOAD_BYTES: u64 = 200 * 1024 * 1024;

pub struct UpgradeOptions {
    /// Only report whether a newer version exists.
    pub check_only: bool,
    /// Install without asking.
    pub yes: bool,
}

/// Runs `autoformat upgrade` and returns the exit code.
pub fn run(options: &UpgradeOptions) -> u8 {
    let program = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!(
                "error: cannot find the running autoformat program: {}",
                error
            );
            return 1;
        }
    };
    match upgrade(RELEASES_URL, &program, options) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("error: {}", message);
            1
        }
    }
}

/// The whole upgrade, with the release address and the program to replace passed in so
/// that tests can use a local server and a temporary file.
fn upgrade(releases_url: &str, program: &Path, options: &UpgradeOptions) -> Result<(), String> {
    if is_part_of_editor_extension(program) {
        return Err(
            "this autoformat came with the VS Code extension, and VS Code keeps it up to date. \
                    Update the extension in VS Code instead."
                .to_string(),
        );
    }
    let current = match Version::parse(env!("CARGO_PKG_VERSION")) {
        Some(version) => version,
        None => return Err("this program has an unreadable version number".to_string()),
    };

    println!("Checking for a newer version...");
    let latest = latest_release(releases_url)?;
    if latest.version <= current {
        println!("autoformat {} is up to date.", current);
        return Ok(());
    }
    println!(
        "A newer version is available: {} -> {}",
        current, latest.version
    );
    if options.check_only {
        println!("Run `autoformat upgrade` to install it.");
        return Ok(());
    }

    let asset = match release_asset() {
        Some(asset) => asset,
        None => {
            return Err("there is no ready-made autoformat for this kind of computer. Build it from source instead:\n  \
                        cargo install --git https://github.com/JohnKesko/autoformat autoformat-cli --force"
                .to_string())
        }
    };
    // Find out before downloading whether the program can be replaced at all.
    check_can_replace(program)?;

    if !options.yes
        && !confirm(&format!(
            "Download and install autoformat {}?",
            latest.version
        ))?
    {
        println!("Not upgrading.");
        return Ok(());
    }

    println!("Downloading {}...", asset);
    let release_files = format!("{}/download/{}", releases_url, latest.tag);
    let archive = download(&format!("{}/{}", release_files, asset))?;
    let sums = download(&format!("{}/SHA256SUMS.txt", release_files))?;
    let sums = match String::from_utf8(sums) {
        Ok(text) => text,
        Err(_) => return Err("SHA256SUMS.txt is not readable text".to_string()),
    };
    let expected = match checksum_for(&sums, asset) {
        Some(checksum) => checksum,
        None => return Err(format!("SHA256SUMS.txt has no checksum for {}", asset)),
    };
    if sha256_hex(&archive) != expected {
        return Err(
            "the download does not match its published checksum, so it was not installed. \
                    Try again later. If this keeps happening, something between you and GitHub is \
                    changing downloads."
                .to_string(),
        );
    }

    let new_program = extract_program(&archive)?;
    replace_program(program, &new_program)?;
    println!(
        "autoformat is now {} ({})",
        latest.version,
        program.display()
    );
    Ok(())
}

/// A release version such as `0.1.2`. Comparing versions compares major, then minor,
/// then patch, because that is the order of the fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    /// Reads `0.1.2` or `v0.1.2`. Anything else gives `None`.
    fn parse(text: &str) -> Option<Version> {
        let text = text.trim().trim_start_matches('v');
        let parts: Vec<&str> = text.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(Version {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A published release: its git tag (`v0.1.2`) and the version that tag names.
struct Release {
    tag: String,
    version: Version,
}

fn http_agent(follow_redirects: bool, timeout_seconds: u64) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .user_agent(concat!("autoformat/", env!("CARGO_PKG_VERSION")))
        .max_redirects(if follow_redirects { 10 } else { 0 })
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(timeout_seconds)))
        .build();
    ureq::Agent::new_with_config(config)
}

/// Finds the newest release. GitHub answers `.../releases/latest` with a redirect to
/// `.../releases/tag/<tag>`, which names it.
fn latest_release(releases_url: &str) -> Result<Release, String> {
    let agent = http_agent(false, 20);
    let url = format!("{}/latest", releases_url);
    let response = match agent.get(&url).call() {
        Ok(response) => response,
        Err(error) => return Err(format!("could not reach GitHub: {}", error)),
    };
    let location = response
        .headers()
        .get("location")
        .and_then(|value| value.to_str().ok());
    let location = match location {
        Some(location) => location,
        None => return Err("GitHub did not say which release is the newest".to_string()),
    };
    let tag = match tag_from_location(location) {
        Some(tag) => tag,
        None => return Err("GitHub has no published autoformat release yet".to_string()),
    };
    match Version::parse(&tag) {
        Some(version) => Ok(Release { tag, version }),
        None => Err(format!(
            "the newest release has an unexpected name: {}",
            tag
        )),
    }
}

fn tag_from_location(location: &str) -> Option<String> {
    let (_, tag) = location.rsplit_once("/tag/")?;
    let tag = tag.trim_matches('/');
    if tag.is_empty() {
        None
    } else {
        Some(tag.to_string())
    }
}

fn download(url: &str) -> Result<Vec<u8>, String> {
    let agent = http_agent(true, 300);
    let mut response = match agent.get(url).call() {
        Ok(response) => response,
        Err(error) => return Err(format!("could not download {}: {}", url, error)),
    };
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("downloading {} failed (HTTP {})", url, status));
    }
    match response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD_BYTES)
        .read_to_vec()
    {
        Ok(bytes) => Ok(bytes),
        Err(error) => Err(format!("could not download {}: {}", url, error)),
    }
}

/// The name of the release archive built for this computer, if there is one.
fn release_asset() -> Option<&'static str> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("autoformat-aarch64-apple-darwin.tar.gz")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("autoformat-x86_64-apple-darwin.tar.gz")
    } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
        Some("autoformat-aarch64-unknown-linux-musl.tar.gz")
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("autoformat-x86_64-unknown-linux-musl.tar.gz")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("autoformat-x86_64-pc-windows-msvc.zip")
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        Some("autoformat-aarch64-pc-windows-msvc.zip")
    } else {
        None
    }
}

/// The published checksum for `asset`, from `sha256sum` style lines: `<hex>  <name>`.
fn checksum_for(sums: &str, asset: &str) -> Option<String> {
    for line in sums.lines() {
        let mut parts = line.split_whitespace();
        let checksum = parts.next()?;
        let name = match parts.next() {
            Some(name) => name.trim_start_matches('*'),
            None => continue,
        };
        if name == asset {
            return Some(checksum.to_lowercase());
        }
    }
    None
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut text = String::new();
    for byte in digest.iter() {
        text.push_str(&format!("{:02x}", byte));
    }
    text
}

/// The copy of autoformat inside the VS Code extension (in a folder named like
/// `johnk.autoformat-0.1.2-darwin-arm64`) is updated by VS Code, not by this command.
fn is_part_of_editor_extension(program: &Path) -> bool {
    program
        .to_string_lossy()
        .to_lowercase()
        .contains("johnk.autoformat-")
}

/// Checks that the folder of the program can be written to, before downloading anything.
fn check_can_replace(program: &Path) -> Result<(), String> {
    let folder = match program.parent() {
        Some(folder) => folder,
        None => return Err("the autoformat program has no folder".to_string()),
    };
    let probe = folder.join(".autoformat-upgrade-check");
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            if cfg!(windows) {
                Err(format!(
                    "cannot write to {}. Run the upgrade in a terminal opened as administrator.",
                    folder.display()
                ))
            } else {
                Err(format!(
                    "cannot write to {}. Run it with administrator rights: sudo autoformat upgrade",
                    folder.display()
                ))
            }
        }
        Err(error) => Err(format!("cannot write to {}: {}", folder.display(), error)),
    }
}

/// Takes the program out of a downloaded release archive.
#[cfg(unix)]
fn extract_program(archive: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Read;

    let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(archive));
    let entries = match tar.entries() {
        Ok(entries) => entries,
        Err(_) => return Err("the download is not a .tar.gz archive".to_string()),
    };
    for entry in entries {
        let mut entry = match entry {
            Ok(entry) => entry,
            Err(_) => return Err("the downloaded archive is damaged".to_string()),
        };
        let is_program = match entry.path() {
            Ok(path) => path
                .file_name()
                .map(|name| name == "autoformat")
                .unwrap_or(false),
            Err(_) => false,
        };
        if is_program {
            let mut bytes = Vec::new();
            if entry.read_to_end(&mut bytes).is_err() {
                return Err("could not read autoformat from the downloaded archive".to_string());
            }
            return Ok(bytes);
        }
    }
    Err("the downloaded archive does not contain autoformat".to_string())
}

/// Takes the program out of a downloaded release archive.
#[cfg(windows)]
fn extract_program(archive: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Read;

    let mut zip = match zip::ZipArchive::new(std::io::Cursor::new(archive)) {
        Ok(zip) => zip,
        Err(_) => return Err("the download is not a .zip archive".to_string()),
    };
    for index in 0..zip.len() {
        let mut file = match zip.by_index(index) {
            Ok(file) => file,
            Err(_) => return Err("the downloaded archive is damaged".to_string()),
        };
        if file.name().ends_with("autoformat.exe") {
            let mut bytes = Vec::new();
            if file.read_to_end(&mut bytes).is_err() {
                return Err("could not read autoformat.exe from the downloaded archive".to_string());
            }
            return Ok(bytes);
        }
    }
    Err("the downloaded archive does not contain autoformat.exe".to_string())
}

/// Writes the new program next to the old one and swaps it in. A running program cannot
/// simply be overwritten on Windows, so the running copy is replaced with `self_replace`.
fn replace_program(program: &Path, new_program: &[u8]) -> Result<(), String> {
    let folder = match program.parent() {
        Some(folder) => folder,
        None => return Err("the autoformat program has no folder".to_string()),
    };
    let staged = folder.join(".autoformat-upgrade-new");
    if let Err(error) = std::fs::write(&staged, new_program) {
        return Err(format!("could not write {}: {}", staged.display(), error));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).is_err() {
            let _ = std::fs::remove_file(&staged);
            return Err("could not mark the new program as runnable".to_string());
        }
    }

    let is_running_program = match std::env::current_exe() {
        Ok(running) => same_file(&running, program),
        Err(_) => false,
    };
    let result = if is_running_program {
        self_replace::self_replace(&staged)
    } else {
        std::fs::rename(&staged, program)
    };
    let _ = std::fs::remove_file(&staged);
    match result {
        Ok(()) => Ok(()),
        Err(error) => Err(format!(
            "could not replace {}: {}",
            program.display(),
            error
        )),
    }
}

fn same_file(left: &Path, right: &Path) -> bool {
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn confirm(question: &str) -> Result<bool, String> {
    print!("{} [y/N] ", question);
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return Err("could not read the answer".to_string());
    }
    let answer = answer.trim().to_lowercase();
    Ok(answer == "y" || answer == "yes")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn versions_are_read_and_compared() {
        assert_eq!(version("v0.1.2").to_string(), "0.1.2");
        assert_eq!(version("0.10.0").to_string(), "0.10.0");
        assert!(Version::parse("v0.1").is_none());
        assert!(Version::parse("v0.1.2-beta.1").is_none());
        assert!(Version::parse("main").is_none());
        assert!(version("0.2.0") > version("0.1.9"));
        assert!(version("0.1.10") > version("0.1.2"));
        assert!(version("1.0.0") > version("0.99.99"));
        assert!(Version::parse(env!("CARGO_PKG_VERSION")).is_some());
    }

    #[test]
    fn the_release_tag_comes_from_the_redirect() {
        assert_eq!(
            tag_from_location("https://github.com/JohnKesko/autoformat/releases/tag/v0.1.2")
                .as_deref(),
            Some("v0.1.2")
        );
        assert_eq!(
            tag_from_location("https://github.com/JohnKesko/autoformat/releases"),
            None
        );
    }

    #[test]
    fn checksums_are_found_by_file_name() {
        let sums = "abc123  autoformat-aarch64-apple-darwin.tar.gz\nDEF456 *autoformat-x86_64-pc-windows-msvc.zip\n";
        assert_eq!(
            checksum_for(sums, "autoformat-aarch64-apple-darwin.tar.gz").as_deref(),
            Some("abc123")
        );
        assert_eq!(
            checksum_for(sums, "autoformat-x86_64-pc-windows-msvc.zip").as_deref(),
            Some("def456")
        );
        assert!(checksum_for(sums, "other.tar.gz").is_none());
    }

    #[test]
    fn sha256_is_written_as_lowercase_hex() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn this_computer_has_a_published_build() {
        // If this fails, the release workflow and release_asset() no longer match.
        assert!(release_asset().is_some());
    }

    #[test]
    fn the_copy_inside_the_vscode_extension_is_left_to_vscode() {
        let bundled = Path::new("/Users/me/.vscode/extensions/johnk.autoformat-0.1.2-darwin-arm64/bin/darwin-arm64/autoformat");
        assert!(is_part_of_editor_extension(bundled));
        assert!(!is_part_of_editor_extension(Path::new(
            "/Users/me/.local/bin/autoformat"
        )));
    }

    /// A fake GitHub on 127.0.0.1: `routes` maps a path to (status, extra header, body).
    #[cfg(unix)]
    fn serve(routes: Vec<(String, u16, String, Vec<u8>)>) -> String {
        use std::io::{BufRead, BufReader};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = match stream {
                    Ok(stream) => stream,
                    Err(_) => continue,
                };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                loop {
                    let mut header = String::new();
                    if reader.read_line(&mut header).unwrap() == 0 || header == "\r\n" {
                        break;
                    }
                }
                let path = request_line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("")
                    .to_string();
                let route = routes
                    .iter()
                    .find(|(route_path, _, _, _)| *route_path == path);
                let (status, extra_header, body) = match route {
                    Some((_, status, header, body)) => (*status, header.clone(), body.clone()),
                    None => (404, String::new(), b"not found".to_vec()),
                };
                let head = format!(
                    "HTTP/1.1 {} X\r\nContent-Length: {}\r\nConnection: close\r\n{}\r\n",
                    status,
                    body.len(),
                    extra_header
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        format!("http://{}/releases", address)
    }

    #[cfg(unix)]
    fn tar_gz_with_program(content: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let encoder = flate2::write::GzEncoder::new(&mut bytes, flate2::Compression::fast());
            let mut builder = tar::Builder::new(encoder);
            for (name, data) in [("LICENSE", b"MIT".as_slice()), ("autoformat", content)] {
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o755);
                header.set_cksum();
                builder.append_data(&mut header, name, data).unwrap();
            }
            builder.into_inner().unwrap().finish().unwrap();
        }
        bytes
    }

    #[cfg(unix)]
    fn fake_release(tag: &str, archive: Vec<u8>, checksum: String) -> String {
        let asset = release_asset().unwrap();
        let sums = format!("{}  {}\n", checksum, asset);
        serve(vec![
            (
                "/releases/latest".to_string(),
                302,
                format!(
                    "Location: https://github.com/JohnKesko/autoformat/releases/tag/{}\r\n",
                    tag
                ),
                Vec::new(),
            ),
            (
                format!("/releases/download/{}/{}", tag, asset),
                200,
                String::new(),
                archive,
            ),
            (
                format!("/releases/download/{}/SHA256SUMS.txt", tag),
                200,
                String::new(),
                sums.into_bytes(),
            ),
        ])
    }

    #[cfg(unix)]
    fn temporary_program(name: &str) -> std::path::PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "autoformat-upgrade-{}-{}",
            name,
            std::process::id()
        ));
        std::fs::create_dir_all(&folder).unwrap();
        let program = folder.join("autoformat");
        std::fs::write(&program, b"old program").unwrap();
        program
    }

    #[cfg(unix)]
    #[test]
    fn the_program_is_extracted_from_a_release_archive() {
        assert_eq!(
            extract_program(&tar_gz_with_program(b"new program")).unwrap(),
            b"new program"
        );
        assert!(extract_program(b"not an archive").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn upgrades_from_a_release() {
        use std::os::unix::fs::PermissionsExt;

        let archive = tar_gz_with_program(b"new program");
        let checksum = sha256_hex(&archive);
        let releases = fake_release("v99.0.0", archive, checksum);
        let program = temporary_program("ok");
        let options = UpgradeOptions {
            check_only: false,
            yes: true,
        };
        upgrade(&releases, &program, &options).unwrap();
        assert_eq!(std::fs::read(&program).unwrap(), b"new program");
        let mode = std::fs::metadata(&program).unwrap().permissions().mode();
        assert_eq!(mode & 0o111, 0o111, "the new program can be run");
    }

    #[cfg(unix)]
    #[test]
    fn a_wrong_checksum_installs_nothing() {
        let archive = tar_gz_with_program(b"new program");
        let releases = fake_release("v99.0.0", archive, "0".repeat(64));
        let program = temporary_program("bad");
        let options = UpgradeOptions {
            check_only: false,
            yes: true,
        };
        let error = upgrade(&releases, &program, &options).unwrap_err();
        assert!(error.contains("checksum"), "{}", error);
        assert_eq!(std::fs::read(&program).unwrap(), b"old program");
    }

    #[cfg(unix)]
    #[test]
    fn nothing_happens_when_up_to_date_or_only_checking() {
        let archive = tar_gz_with_program(b"new program");
        let checksum = sha256_hex(&archive);
        let older = fake_release("v0.0.1", archive.clone(), checksum.clone());
        let program = temporary_program("current");
        let options = UpgradeOptions {
            check_only: false,
            yes: true,
        };
        upgrade(&older, &program, &options).unwrap();
        assert_eq!(std::fs::read(&program).unwrap(), b"old program");

        let newer = fake_release("v99.0.0", archive, checksum);
        let check = UpgradeOptions {
            check_only: true,
            yes: true,
        };
        upgrade(&newer, &program, &check).unwrap();
        assert_eq!(std::fs::read(&program).unwrap(), b"old program");
    }
}
