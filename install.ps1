# Installs the autoformat command line tool on Windows, from the latest GitHub release:
#
#   irm https://raw.githubusercontent.com/JohnKesko/autoformat/master/install.ps1 | iex
#
# Settings (environment variables, all optional):
#   AUTOFORMAT_INSTALL_DIR      where to put the program (default: %LOCALAPPDATA%\Programs\autoformat)
#   AUTOFORMAT_VERSION          install this version instead of the latest, for example 0.1.2
#   AUTOFORMAT_NO_PATH_UPDATE   set to 1 to leave your PATH alone
#
# The download is checked against the release's SHA256SUMS.txt before anything is
# installed. No administrator rights are needed.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$repo = 'JohnKesko/autoformat'

# Which build fits this computer.
$architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
if ($architecture -eq 'X64') {
    $platform = 'x86_64-pc-windows-msvc'
} elseif ($architecture -eq 'Arm64') {
    $platform = 'aarch64-pc-windows-msvc'
} else {
    throw "There is no autoformat build for a $architecture processor."
}
$asset = "autoformat-$platform.zip"

if ($env:AUTOFORMAT_DOWNLOAD_BASE) {
    # Only for testing: a URL that holds the release files.
    $base = $env:AUTOFORMAT_DOWNLOAD_BASE
} elseif ($env:AUTOFORMAT_VERSION) {
    $base = "https://github.com/$repo/releases/download/v$($env:AUTOFORMAT_VERSION.TrimStart('v'))"
} else {
    $base = "https://github.com/$repo/releases/latest/download"
}

if ($env:AUTOFORMAT_INSTALL_DIR) {
    $installDir = $env:AUTOFORMAT_INSTALL_DIR
} else {
    $installDir = Join-Path $env:LOCALAPPDATA 'Programs\autoformat'
}

$temp = Join-Path ([System.IO.Path]::GetTempPath()) ('autoformat-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $temp | Out-Null
try {
    Write-Host "Downloading $asset"
    $archive = Join-Path $temp $asset
    $sums = Join-Path $temp 'SHA256SUMS.txt'
    Invoke-WebRequest -Uri "$base/$asset" -OutFile $archive -UseBasicParsing
    Invoke-WebRequest -Uri "$base/SHA256SUMS.txt" -OutFile $sums -UseBasicParsing

    $expected = $null
    foreach ($line in Get-Content $sums) {
        $parts = $line.Trim() -split '\s+'
        if ($parts.Count -ge 2 -and $parts[1].TrimStart('*') -eq $asset) {
            $expected = $parts[0].ToLower()
        }
    }
    if (-not $expected) {
        throw "SHA256SUMS.txt has no checksum for $asset."
    }
    $actual = (Get-FileHash -Algorithm SHA256 -Path $archive).Hash.ToLower()
    if ($actual -ne $expected) {
        throw 'The download does not match its published checksum, so it was not installed.'
    }

    $unpacked = Join-Path $temp 'unpacked'
    Expand-Archive -Path $archive -DestinationPath $unpacked -Force
    $binary = Get-ChildItem -Path $unpacked -Recurse -Filter 'autoformat.exe' | Select-Object -First 1
    if (-not $binary) {
        throw 'The download does not contain autoformat.exe.'
    }

    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    $program = Join-Path $installDir 'autoformat.exe'
    $old = "$program.old"
    # A running autoformat.exe cannot be overwritten, but it can be renamed.
    Remove-Item $old -Force -ErrorAction SilentlyContinue
    if (Test-Path $program) {
        Move-Item $program $old -Force
    }
    Copy-Item $binary.FullName $program -Force
    Remove-Item $old -Force -ErrorAction SilentlyContinue

    $installed = & $program --version
    Write-Host "Installed $installed to $program"

    if ($env:AUTOFORMAT_NO_PATH_UPDATE -ne '1') {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        if (-not $userPath) {
            $userPath = ''
        }
        $folders = $userPath -split ';' | Where-Object { $_ -ne '' }
        if ($folders -notcontains $installDir) {
            $newPath = (@($folders) + $installDir) -join ';'
            [Environment]::SetEnvironmentVariable('Path', $newPath, 'User')
            Write-Host "Added $installDir to your PATH. Open a new terminal to use autoformat."
        } else {
            Write-Host "Run 'autoformat --help' to get started. Update later with 'autoformat upgrade'."
        }
    }
} finally {
    Remove-Item -Recurse -Force $temp -ErrorAction SilentlyContinue
}
