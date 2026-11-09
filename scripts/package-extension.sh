#!/usr/bin/env bash
# Builds a platform-specific VS Code extension package (.vsix).
#
# Usage: scripts/package-extension.sh <vscode-target> <path-to-autoformat-binary>
#
# <vscode-target> is one of: win32-x64, win32-arm64, linux-x64, linux-arm64,
# alpine-x64, alpine-arm64, darwin-x64, darwin-arm64.
#
# The binary is copied to vscode/extension/bin/<platform>-<arch>/ and the package is
# written to vscode/extension/autoformat-<target>-<version>.vsix.
set -euo pipefail

target="$1"
binary="$2"

case "$target" in
  win32-x64|win32-arm64|linux-x64|linux-arm64|darwin-x64|darwin-arm64) folder="$target" ;;
  alpine-x64) folder="linux-x64" ;;
  alpine-arm64) folder="linux-arm64" ;;
  *) echo "unknown target: $target" >&2; exit 2 ;;
esac

root="$(cd "$(dirname "$0")/.." && pwd)"
extension="$root/vscode/extension"
name="autoformat"
if [[ "$target" == win32-* ]]; then
  name="autoformat.exe"
fi

rm -rf "$extension/bin"
mkdir -p "$extension/bin/$folder"
cp "$binary" "$extension/bin/$folder/$name"
chmod 755 "$extension/bin/$folder/$name"
cp "$root/LICENSE" "$extension/LICENSE"
# The marketplace page shows the project README; vsce only reads files inside the
# extension folder, so a copy is placed there (it is ignored by git).
cp "$root/README.md" "$extension/README.md"

cd "$extension"
npm ci --no-audit --no-fund
npm run compile
version="$(node -p "require('./package.json').version")"
npx vsce package --target "$target" --out "autoformat-$target-$version.vsix"
