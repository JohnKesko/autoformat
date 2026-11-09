#!/bin/sh
# Installs the autoformat command line tool on macOS or Linux, from the latest
# GitHub release:
#
#   curl -fsSL https://raw.githubusercontent.com/JohnKesko/autoformat/master/install.sh | sh
#
# Settings (environment variables, all optional):
#   AUTOFORMAT_INSTALL_DIR   where to put the program (default: $HOME/.local/bin)
#   AUTOFORMAT_VERSION       install this version instead of the latest, for example 0.1.2
#
# The download is checked against the release's SHA256SUMS.txt before anything is
# installed. No administrator rights are needed for the default folder.
set -eu

repo="JohnKesko/autoformat"
install_dir="${AUTOFORMAT_INSTALL_DIR:-$HOME/.local/bin}"
version="${AUTOFORMAT_VERSION:-}"
# Only for testing: a URL (or file:// path) that holds the release files.
base="${AUTOFORMAT_DOWNLOAD_BASE:-}"

say() {
    printf '%s\n' "$*"
}

fail() {
    printf 'error: %s\n' "$*" >&2
    exit 1
}

# Which build fits this computer.
case "$(uname -s)" in
    Darwin) system="apple-darwin" ;;
    Linux) system="unknown-linux-musl" ;;
    *) fail "this script is for macOS and Linux. On Windows, use install.ps1." ;;
esac
case "$(uname -m)" in
    x86_64 | amd64) chip="x86_64" ;;
    arm64 | aarch64) chip="aarch64" ;;
    *) fail "there is no autoformat build for a $(uname -m) processor" ;;
esac
# A terminal running under Rosetta on Apple Silicon reports x86_64; use the native build.
if [ "$system" = "apple-darwin" ] && [ "$chip" = "x86_64" ] &&
    [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || echo 0)" = "1" ]; then
    chip="aarch64"
fi
asset="autoformat-$chip-$system.tar.gz"

if [ -z "$base" ]; then
    if [ -n "$version" ]; then
        base="https://github.com/$repo/releases/download/v${version#v}"
    else
        base="https://github.com/$repo/releases/latest/download"
    fi
fi

download() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$1" -o "$2"
    elif command -v wget >/dev/null 2>&1; then
        wget -q "$1" -O "$2"
    else
        fail "curl or wget is needed to download autoformat"
    fi
}

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    elif command -v shasum >/dev/null 2>&1; then
        shasum -a 256 "$1" | awk '{print $1}'
    else
        fail "sha256sum or shasum is needed to check the download"
    fi
}

temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT

say "Downloading $asset"
download "$base/$asset" "$temp/$asset" || fail "could not download $base/$asset"
download "$base/SHA256SUMS.txt" "$temp/SHA256SUMS.txt" || fail "could not download $base/SHA256SUMS.txt"

expected=$(awk -v name="$asset" '$2 == name || $2 == "*" name {print tolower($1)}' "$temp/SHA256SUMS.txt")
[ -n "$expected" ] || fail "SHA256SUMS.txt has no checksum for $asset"
actual=$(sha256_of "$temp/$asset" | tr 'A-F' 'a-f')
[ "$actual" = "$expected" ] || fail "the download does not match its published checksum, so it was not installed"

mkdir -p "$temp/unpacked"
tar -xzf "$temp/$asset" -C "$temp/unpacked"
binary=$(find "$temp/unpacked" -type f -name autoformat | head -n 1)
[ -n "$binary" ] || fail "the download does not contain the autoformat program"

mkdir -p "$install_dir" 2>/dev/null || true
if [ ! -w "$install_dir" ]; then
    fail "cannot write to $install_dir. Choose another folder, or run: curl -fsSL https://raw.githubusercontent.com/$repo/master/install.sh | sudo AUTOFORMAT_INSTALL_DIR=$install_dir sh"
fi

# Copy next to the old program, then move it over in one step, so a half-written
# file is never left behind.
cp "$binary" "$install_dir/.autoformat.new"
chmod 755 "$install_dir/.autoformat.new"
mv -f "$install_dir/.autoformat.new" "$install_dir/autoformat"

say "Installed $("$install_dir/autoformat" --version) to $install_dir/autoformat"

case ":$PATH:" in
    *":$install_dir:"*)
        say "Run 'autoformat --help' to get started. Update later with 'autoformat upgrade'."
        ;;
    *)
        say ""
        say "$install_dir is not in your PATH yet. Add this line to your shell's startup file"
        say "(~/.zshrc on macOS, ~/.bashrc on most Linux systems), then open a new terminal:"
        say ""
        say "    export PATH=\"$install_dir:\$PATH\""
        ;;
esac
