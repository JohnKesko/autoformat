//! The `autoformat` command line tool.
//!
//! ```text
//! autoformat format [options] [paths...]
//! autoformat format --stdin --stdin-filepath src/main.rs < input > output
//! autoformat config [path]
//! autoformat languages
//! autoformat tree <file>
//! autoformat lsp
//! autoformat upgrade [--check] [--yes]
//! ```

mod config_help;
mod lsp;
mod upgrade;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use autoformat_config::{discover_config, load_config_file, Config};
use autoformat_core::{format_source, LanguageAdapter};
use autoformat_languages::Languages;

const HELP: &str = "\
AutoFormat - one formatter for Rust, TOML, JSON, TypeScript, JavaScript, C#, Java, Kotlin and PowerShell

USAGE:
    autoformat format [OPTIONS] [PATHS...]   Format files or directories in place
    autoformat config [PATH]                 Show the configuration used for a path
    autoformat languages                     List supported languages and extensions
    autoformat tree <FILE>                   Print the syntax tree of a file (for debugging)
    autoformat lsp                           Run as a language server, for editors such as Zed
    autoformat upgrade [--check] [--yes]     Update autoformat to the newest version

FORMAT OPTIONS:
    --check                  Do not write files; exit with code 1 if any file would change
    --stdin                  Read code from standard input and write the result to standard output
    --stdin-filepath <PATH>  With --stdin: the file name used to pick the language and find autoformat.toml
    --language <NAME>        Use this language instead of detecting it from the file extension
    --config <FILE>          Use this configuration file instead of searching for autoformat.toml
    --verbose                Print extra information

UPGRADE OPTIONS:
    --check                  Only tell whether a newer version exists
    -y, --yes                Install without asking first

OTHER OPTIONS:
    -h, --help               Show this help
    -V, --version            Show the version

EXIT CODES:
    0  success
    1  a file could not be formatted, or (with --check) a file is not formatted
    2  invalid command line or configuration
";

/// Options for `autoformat format`.
#[derive(Default)]
struct FormatOptions {
    paths: Vec<PathBuf>,
    check: bool,
    stdin: bool,
    stdin_filepath: Option<PathBuf>,
    language: Option<String>,
    config: Option<PathBuf>,
    verbose: bool,
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&arguments);
    ExitCode::from(code)
}

fn run(arguments: &[String]) -> u8 {
    let command = match arguments.first() {
        Some(command) => command.as_str(),
        None => {
            print!("{}", HELP);
            return 2;
        }
    };
    let rest = &arguments[1..];
    match command {
        "format" | "fmt" => match parse_format_options(rest) {
            Ok(options) => run_format(&options),
            Err(message) => usage_error(&message),
        },
        "config" => run_config(rest),
        "languages" => run_languages(),
        "tree" => run_tree(rest),
        "lsp" => run_lsp(rest),
        "upgrade" | "update" => match parse_upgrade_options(rest) {
            Ok(options) => upgrade::run(&options),
            Err(message) => usage_error(&message),
        },
        "-h" | "--help" | "help" => {
            print!("{}", HELP);
            0
        }
        "-V" | "--version" | "version" => {
            println!("autoformat {}", env!("CARGO_PKG_VERSION"));
            0
        }
        other => usage_error(&format!("unknown command `{}`", other)),
    }
}

fn usage_error(message: &str) -> u8 {
    eprintln!("error: {}", message);
    eprintln!("Run `autoformat --help` for usage.");
    2
}

/// `autoformat lsp` talks over standard input and output. `--stdio` says the same thing
/// and is accepted because some editors always add it.
fn run_lsp(arguments: &[String]) -> u8 {
    for argument in arguments {
        if argument != "--stdio" {
            return usage_error(&format!("unknown option `{}` for lsp", argument));
        }
    }
    lsp::run()
}

fn parse_upgrade_options(arguments: &[String]) -> Result<upgrade::UpgradeOptions, String> {
    let mut options = upgrade::UpgradeOptions {
        check_only: false,
        yes: false,
    };
    for argument in arguments {
        match argument.as_str() {
            "--check" => options.check_only = true,
            "-y" | "--yes" => options.yes = true,
            other => return Err(format!("unknown option `{}` for upgrade", other)),
        }
    }
    Ok(options)
}

fn parse_format_options(arguments: &[String]) -> Result<FormatOptions, String> {
    let mut options = FormatOptions::default();
    let mut index = 0;
    while index < arguments.len() {
        let argument = arguments[index].as_str();
        match argument {
            "--check" => options.check = true,
            "--stdin" => options.stdin = true,
            "--verbose" => options.verbose = true,
            "--stdin-filepath" | "--language" | "--config" => {
                index += 1;
                let value = match arguments.get(index) {
                    Some(value) => value.clone(),
                    None => return Err(format!("`{}` needs a value", argument)),
                };
                match argument {
                    "--stdin-filepath" => options.stdin_filepath = Some(PathBuf::from(value)),
                    "--language" => options.language = Some(value),
                    _ => options.config = Some(PathBuf::from(value)),
                }
            }
            _ if argument.starts_with('-') && argument != "-" => {
                return Err(format!("unknown option `{}`", argument));
            }
            _ => options.paths.push(PathBuf::from(argument)),
        }
        index += 1;
    }
    if options.stdin && !options.paths.is_empty() {
        return Err("--stdin cannot be combined with file paths".to_string());
    }
    if !options.stdin && options.paths.is_empty() {
        return Err("no files given (use `autoformat format <paths...>` or `--stdin`)".to_string());
    }
    if options.stdin_filepath.is_some() && !options.stdin {
        return Err("--stdin-filepath only works together with --stdin".to_string());
    }
    Ok(options)
}

/// Loads the configuration from `--config`, or searches upward from `near`.
fn load_config(explicit: &Option<PathBuf>, near: &Path) -> Result<Config, String> {
    match explicit {
        Some(path) => load_config_file(path).map_err(|error| error.to_string()),
        None => match discover_config(near) {
            Ok((config, _)) => Ok(config),
            Err(error) => Err(error.to_string()),
        },
    }
}

fn run_format(options: &FormatOptions) -> u8 {
    let languages = Languages::new();
    if let Some(name) = &options.language {
        if languages.by_name(name).is_none() {
            return usage_error(&format!(
                "unknown language `{}` (see `autoformat languages`)",
                name
            ));
        }
    }
    if options.stdin {
        return format_stdin(options, &languages);
    }

    let mut files = Vec::new();
    for path in &options.paths {
        if path.is_dir() {
            collect_files(path, &languages, &mut files);
        } else if path.is_file() {
            files.push(path.clone());
        } else {
            eprintln!("error: {} does not exist", path.display());
            return 1;
        }
    }

    let mut failed = false;
    let mut unformatted = 0;
    let mut changed = 0;
    for file in &files {
        match format_file(file, options, &languages) {
            Ok(FileResult::Unchanged) => {}
            Ok(FileResult::Changed) => {
                if options.check {
                    println!("would reformat {}", file.display());
                    unformatted += 1;
                } else {
                    if options.verbose {
                        println!("formatted {}", file.display());
                    }
                    changed += 1;
                }
            }
            Err(message) => {
                eprintln!("error: {}: {}", file.display(), message);
                failed = true;
            }
        }
    }

    if options.verbose || options.check {
        if options.check {
            println!(
                "{} file(s) checked, {} would be reformatted",
                files.len(),
                unformatted
            );
        } else {
            println!("{} file(s) checked, {} reformatted", files.len(), changed);
        }
    }
    if failed || unformatted > 0 {
        1
    } else {
        0
    }
}

enum FileResult {
    Changed,
    Unchanged,
}

fn format_file(
    path: &Path,
    options: &FormatOptions,
    languages: &Languages,
) -> Result<FileResult, String> {
    let adapter = pick_language(path, options, languages)?;
    let config = load_config(&options.config, path)?;
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read the file: {}", error))?;
    let formatted = format_source(adapter, &source, &config).map_err(|error| error.to_string())?;
    if options.verbose {
        for note in &formatted.notes {
            eprintln!("note: {}: {}", path.display(), note);
        }
    }
    if formatted.text == source {
        return Ok(FileResult::Unchanged);
    }
    if !options.check {
        std::fs::write(path, &formatted.text)
            .map_err(|error| format!("cannot write the file: {}", error))?;
    }
    Ok(FileResult::Changed)
}

fn format_stdin(options: &FormatOptions, languages: &Languages) -> u8 {
    let name_path = options.stdin_filepath.clone().unwrap_or_default();
    let adapter = match pick_language(&name_path, options, languages) {
        Ok(adapter) => adapter,
        Err(message) => return usage_error(&message),
    };
    let config_start = if name_path.as_os_str().is_empty() {
        std::env::current_dir().unwrap_or_default()
    } else {
        name_path.clone()
    };
    let config = match load_config(&options.config, &config_start) {
        Ok(config) => config,
        Err(message) => {
            eprintln!("error: {}", message);
            return 2;
        }
    };

    let mut source = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut source) {
        eprintln!("error: cannot read standard input: {}", error);
        return 1;
    }
    match format_source(adapter, &source, &config) {
        Ok(formatted) => {
            if options.verbose {
                for note in &formatted.notes {
                    eprintln!("note: {}", note);
                }
            }
            if options.check {
                return if formatted.text == source { 0 } else { 1 };
            }
            let mut stdout = std::io::stdout();
            if stdout.write_all(formatted.text.as_bytes()).is_err() {
                return 1;
            }
            0
        }
        Err(error) => {
            eprintln!("error: {}", error);
            1
        }
    }
}

fn pick_language<'a>(
    path: &Path,
    options: &FormatOptions,
    languages: &'a Languages,
) -> Result<&'a dyn LanguageAdapter, String> {
    if let Some(name) = &options.language {
        return languages
            .by_name(name)
            .ok_or_else(|| format!("unknown language `{}`", name));
    }
    match languages.for_path(path) {
        Some(adapter) => Ok(adapter),
        None => Err(format!(
            "cannot tell the language of `{}` from its extension (use --language)",
            path.display()
        )),
    }
}

/// Finds supported files under a directory. Hidden directories, `target` and
/// `node_modules` are skipped.
fn collect_files(directory: &Path, languages: &Languages, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("warning: cannot read {}: {}", directory.display(), error);
            return;
        }
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    for path in paths {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default();
        if path.is_dir() {
            let skip = name.starts_with('.') || name == "target" || name == "node_modules";
            if !skip {
                collect_files(&path, languages, files);
            }
        } else if languages.for_path(&path).is_some() {
            files.push(path);
        }
    }
}

fn run_config(arguments: &[String]) -> u8 {
    let start = match arguments.first() {
        Some(path) => PathBuf::from(path),
        None => std::env::current_dir().unwrap_or_default(),
    };
    match discover_config(&start) {
        Ok((config, path)) => {
            match path {
                Some(path) => println!("# from {}", path.display()),
                None => println!("# no autoformat.toml found; using defaults"),
            }
            print!("{}", config);
            0
        }
        Err(error) => {
            eprintln!("error: {}", error);
            2
        }
    }
}

fn run_languages() -> u8 {
    let languages = Languages::new();
    for adapter in languages.all() {
        let extensions: Vec<String> = adapter
            .extensions()
            .iter()
            .map(|extension| format!(".{}", extension))
            .collect();
        println!("{:<12} {}", adapter.name(), extensions.join(" "));
    }
    0
}

fn run_tree(arguments: &[String]) -> u8 {
    let mut options = FormatOptions::default();
    let mut file = None;
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == "--language" {
            index += 1;
            options.language = arguments.get(index).cloned();
        } else {
            file = Some(PathBuf::from(&arguments[index]));
        }
        index += 1;
    }
    let file = match file {
        Some(file) => file,
        None => return usage_error("`tree` needs a file"),
    };
    let languages = Languages::new();
    let adapter = match pick_language(&file, &options, &languages) {
        Ok(adapter) => adapter,
        Err(message) => return usage_error(&message),
    };
    let source = match std::fs::read_to_string(&file) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("error: cannot read {}: {}", file.display(), error);
            return 1;
        }
    };
    match adapter.grammar().parse(&source) {
        Ok(tree) => {
            print!("{}", tree.dump());
            if let Some(error) = tree.first_error() {
                println!("syntax error: {}", error);
            }
            0
        }
        Err(message) => {
            eprintln!("error: {}", message);
            1
        }
    }
}
