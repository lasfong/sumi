# PowerShell Build, Test & Package Script for Windows 10/11 (x86_64-pc-windows-msvc) — Sumi v2.0 (Tauri v2 + React 19 + KLineChart v10)
# Usage:
#   powershell -ExecutionPolicy Bypass -File .\scripts\build_windows.ps1
#   powershell -ExecutionPolicy Bypass -File .\scripts\build_windows.ps1 -Run
#   powershell -ExecutionPolicy Bypass -File .\scripts\build_windows.ps1 -SkipTests -Run

param(
    [switch]$SkipTests,
    [switch]$Run
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

Write-Host "=== Sumi Supercharts v2.0 (Tauri v2 + Rust + KLineChart v10) — Windows Build ===" -ForegroundColor Cyan

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $CargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path (Join-Path $CargoBin "cargo.exe")) {
        $env:PATH = "$CargoBin;$env:PATH"
    } else {
        throw "Rust cargo not found. Install Rust via https://rustup.rs (with MSVC C++ Build Tools) and reopen PowerShell."
    }
}

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    throw "Node.js / npm not found. Install Node.js LTS (https://nodejs.org) and reopen PowerShell."
}

rustc --version
cargo --version
node --version
npm --version

Write-Host "`n[1/4] Installing & building Frontend UI (React 19 + KLineChart v10.1.1)..." -ForegroundColor Yellow
# Use `npm install --include=optional` rather than `npm ci` to avoid npm#4828 missing win32-x64-msvc native bindings
npm --prefix ui install --include=optional --no-audit --no-fund
npm --prefix ui run typecheck
npm --prefix ui run build

if (-not $SkipTests) {
    Write-Host "`n[2/4] Running Rust formatting, Clippy (MSVC), and automated workspace tests..." -ForegroundColor Yellow
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
} else {
    Write-Host "`n[2/4] Skipping Rust checks/tests (-SkipTests)." -ForegroundColor DarkGray
}

Write-Host "`n[3/4] Building Tauri v2 Windows Desktop Bundle (NSIS / MSI / WebView2)..." -ForegroundColor Yellow
$HasCargoTauri = $false
try {
    cargo tauri --version | Out-Null
    $HasCargoTauri = $true
} catch {
    $HasCargoTauri = $false
}

Push-Location (Join-Path $RepoRoot "crates\sumi-desktop")
try {
    if ($HasCargoTauri) {
        cargo tauri build
    } else {
        Write-Host "cargo-tauri not found globally; using local @tauri-apps/cli..." -ForegroundColor DarkYellow
        & (Join-Path $RepoRoot "ui\node_modules\.bin\tauri.cmd") build
    }
} finally {
    Pop-Location
}

$OutDir = Join-Path $RepoRoot "dist\SumiSupercharts-win64"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$ExeSrc = Join-Path $RepoRoot "target\release\sumi-desktop.exe"
$ExeDst = Join-Path $OutDir "sumi-desktop.exe"
if (Test-Path $ExeSrc) {
    Copy-Item -Force $ExeSrc $ExeDst
}

$BundleDir = Join-Path $RepoRoot "target\release\bundle"
if (Test-Path $BundleDir) {
    Copy-Item -Recurse -Force $BundleDir (Join-Path $OutDir "bundle")
}

Write-Host "`n[4/4] Packaged Windows Tauri v2 release at: $OutDir" -ForegroundColor Green

if ($Run -and (Test-Path $ExeDst)) {
    Write-Host "Launching Sumi Supercharts..." -ForegroundColor Cyan
    Start-Process -FilePath $ExeDst -WorkingDirectory $RepoRoot
}
