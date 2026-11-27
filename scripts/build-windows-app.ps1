<#
.SYNOPSIS
    Sumi Supercharts v3.0 — Windows 10/11 (x64 WebView2) Portable + Installer Build Script
.DESCRIPTION
    Mirrors Yumeko's `scripts/build-windows-app.ps1` architecture:
    - Clean-slate compatible (requires only Rust MSVC toolchain + Node.js >= 20; zero hardcoded paths)
    - Produces portable package in `dist-windows\Sumi\` + `Sumi-Supercharts-v3.0.0-windows-x64.zip`
    - Generates `sumi.cmd` launcher & optional desktop shortcut / `%LOCALAPPDATA%\Programs\Sumi` install
    - Registers `HKCU\Software\Classes\sumi` URI scheme (`sumi://inspect?...`) without admin rights
    - Optional `-BuildInstaller` invokes `tauri build` for NSIS / MSI installers.
.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\scripts\build-windows-app.ps1 -InstallToLocalAppData -CreateDesktopShortcut -RegisterUriScheme
#>
[CmdletBinding()]
param(
    [switch]$InstallToLocalAppData,
    [switch]$CreateDesktopShortcut,
    [switch]$RegisterUriScheme,
    [switch]$BuildInstaller,
    [switch]$SkipZip
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$DistRoot = Join-Path $RepoRoot "dist-windows"
$StageDir = Join-Path $DistRoot "Sumi"

$CargoToml = Get-Content (Join-Path $RepoRoot "Cargo.toml") -Raw
$VersionMatch = [regex]::Match($CargoToml, '(?m)^version\s*=\s*"([^"]+)"')
$Version = if ($VersionMatch.Success) { $VersionMatch.Groups[1].Value } else { "3.0.0" }

Write-Host "==> Building Sumi Supercharts v$Version for Windows x64" -ForegroundColor Cyan

# Check WebView2 runtime presence (advisory)
$Wv2Reg = @(
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKCU:\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
)
$HasWv2 = $false
foreach ($k in $Wv2Reg) {
    if (Test-Path $k) { $HasWv2 = $true; break }
}
if (-not $HasWv2) {
    Write-Warning "Microsoft Edge WebView2 Runtime not detected in registry. Install Evergreen Bootstrapper if the app fails to open."
}

Write-Host "==> [1/4] Building React UI bundle..." -ForegroundColor Cyan
Push-Location (Join-Path $RepoRoot "ui")
try {
    npm ci --no-audit --no-fund
    if ($LASTEXITCODE -ne 0) { npm install --no-audit --no-fund }
    npm run build
    if ($LASTEXITCODE -ne 0) { throw "UI build failed" }
} finally {
    Pop-Location
}

Write-Host "==> [2/4] Building Rust release binary (sumi-desktop)..." -ForegroundColor Cyan
Push-Location $RepoRoot
try {
    cargo build --release --package sumi-desktop
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    if (Test-Path (Join-Path $RepoRoot "crates\sumi-cli\Cargo.toml")) {
        cargo build --release --package sumi-cli
        if ($LASTEXITCODE -ne 0) { throw "cargo build sumi-cli failed" }
    }
} finally {
    Pop-Location
}

Write-Host "==> [3/4] Staging portable package in $StageDir..." -ForegroundColor Cyan
if (Test-Path $StageDir) { Remove-Item $StageDir -Recurse -Force }
New-Item -ItemType Directory -Path $StageDir -Force | Out-Null

$DesktopExe = Join-Path $RepoRoot "target\release\sumi-desktop.exe"
if (-not (Test-Path $DesktopExe)) { throw "Missing release executable: $DesktopExe" }
Copy-Item $DesktopExe (Join-Path $StageDir "Sumi Supercharts.exe")

$CliExe = Join-Path $RepoRoot "target\release\sumi.exe"
if (Test-Path $CliExe) {
    Copy-Item $CliExe (Join-Path $StageDir "sumi.exe")
}

$LauncherCmd = @"
@echo off
setlocal
start "" "%~dp0Sumi Supercharts.exe" %*
"@
Set-Content -Path (Join-Path $StageDir "sumi.cmd") -Value $LauncherCmd -Encoding ASCII

$ReadmeWin = @"
Sumi Supercharts v$Version (Windows x64 Portable)
=================================================
- Double-click "Sumi Supercharts.exe" (or run sumi.cmd).
- Local data directory: %LOCALAPPDATA%\sumi\data
- Local config directory: %APPDATA%\sumi
- Cross-device sync: Open Database icon -> "Đồng Bộ Đa Máy" -> select OneDrive/iCloud/Syncthing folder or import/export .sumi.json.
"@
Set-Content -Path (Join-Path $StageDir "README-WINDOWS.txt") -Value $ReadmeWin -Encoding UTF8

if (-not $SkipZip) {
    $ZipPath = Join-Path $DistRoot "Sumi-Supercharts-v$Version-windows-x64.zip"
    if (Test-Path $ZipPath) { Remove-Item $ZipPath -Force }
    Compress-Archive -Path (Join-Path $StageDir "*") -DestinationPath $ZipPath -CompressionLevel Optimal
    Write-Host "==> Created portable archive: $ZipPath" -ForegroundColor Green
}

$ActiveExe = Join-Path $StageDir "Sumi Supercharts.exe"
if ($InstallToLocalAppData) {
    $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\Sumi"
    if (Test-Path $InstallDir) { Remove-Item $InstallDir -Recurse -Force }
    Copy-Item $StageDir $InstallDir -Recurse -Force
    $ActiveExe = Join-Path $InstallDir "Sumi Supercharts.exe"
    Write-Host "==> Installed to $InstallDir" -ForegroundColor Green
}

if ($CreateDesktopShortcut) {
    $Desktop = [Environment]::GetFolderPath("Desktop")
    $LnkPath = Join-Path $Desktop "Sumi Supercharts.lnk"
    $WshShell = New-Object -ComObject WScript.Shell
    $Shortcut = $WshShell.CreateShortcut($LnkPath)
    $Shortcut.TargetPath = $ActiveExe
    $Shortcut.WorkingDirectory = Split-Path $ActiveExe -Parent
    $Shortcut.Description = "Sumi Supercharts v$Version — QMV KT01 Workstation"
    $Shortcut.Save()
    Write-Host "==> Created Desktop shortcut: $LnkPath" -ForegroundColor Green
}

if ($RegisterUriScheme) {
    $BaseKey = "HKCU:\Software\Classes\sumi"
    New-Item -Path "$BaseKey\shell\open\command" -Force | Out-Null
    Set-ItemProperty -Path $BaseKey -Name "(Default)" -Value "URL:Sumi Supercharts Protocol"
    Set-ItemProperty -Path $BaseKey -Name "URL Protocol" -Value ""
    Set-ItemProperty -Path "$BaseKey\shell\open\command" -Name "(Default)" -Value "`"$ActiveExe`" `"%1`""
    Write-Host "==> Registered sumi:// URI handler in HKCU" -ForegroundColor Green
}

if ($BuildInstaller) {
    Write-Host "==> [4/4] Building NSIS/MSI installer via Tauri CLI..." -ForegroundColor Cyan
    Push-Location (Join-Path $RepoRoot "crates\sumi-desktop")
    try {
        npx --prefix (Join-Path $RepoRoot "ui") tauri build --bundles nsis,msi
        if ($LASTEXITCODE -ne 0) { throw "tauri build installer failed" }
    } finally {
        Pop-Location
    }
}

Write-Host "==> Windows build complete!" -ForegroundColor Green
