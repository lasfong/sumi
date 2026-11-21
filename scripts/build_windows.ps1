# PowerShell Build, Test & Package Script for Windows 10/11 (x86_64-pc-windows-msvc)
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

Write-Host "=== Sumi Workstation (Pure Rust Desktop) — Windows Build ===" -ForegroundColor Cyan

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $CargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path (Join-Path $CargoBin "cargo.exe")) {
        $env:PATH = "$CargoBin;$env:PATH"
    } else {
        throw "Rust cargo not found. Install Rust via https://rustup.rs (with MSVC C++ Build Tools) and reopen PowerShell."
    }
}

rustc --version
cargo --version

if (-not $SkipTests) {
    Write-Host "`n[1/3] Running automated test suite..." -ForegroundColor Yellow
    cargo test
} else {
    Write-Host "`n[1/3] Skipping unit tests (-SkipTests)." -ForegroundColor DarkGray
}

Write-Host "`n[2/3] Building optimized release executable (DX12 / Vulkan wgpu)..." -ForegroundColor Yellow
cargo build --release

$OutDir = Join-Path $RepoRoot "dist\SumiWorkstation-win64"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

$ExeSrc = Join-Path $RepoRoot "target\release\sumi-workstation.exe"
$ExeDst = Join-Path $OutDir "sumi-workstation.exe"
Copy-Item -Force $ExeSrc $ExeDst

if (Test-Path (Join-Path $RepoRoot "assets")) {
    Copy-Item -Recurse -Force (Join-Path $RepoRoot "assets") (Join-Path $OutDir "assets")
}

Write-Host "`n[3/3] Packaged Windows release at: $ExeDst" -ForegroundColor Green

if ($Run) {
    Write-Host "Launching Sumi Workstation..." -ForegroundColor Cyan
    Start-Process -FilePath $ExeDst -WorkingDirectory $RepoRoot
}
