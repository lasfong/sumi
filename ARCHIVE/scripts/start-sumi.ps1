<#
.SYNOPSIS
    Starts the Sumi Local Replay Workstation (Backend + Frontend) and launches the browser.
#>
param(
    [switch]$NoBrowser
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot

Write-Host "========================================================" -ForegroundColor Cyan
Write-Host "       Starting Sumi Local Replay Workstation           " -ForegroundColor Cyan
Write-Host "========================================================" -ForegroundColor Cyan

# 1. Resolve Python executable
$PythonExe = Join-Path $Root "backend\.venv\Scripts\python.exe"
if (-not (Test-Path $PythonExe)) {
    $PythonExe = "python"
    Write-Host "[NOTE] Using system python" -ForegroundColor Yellow
} else {
    Write-Host "[OK] Using Python venv at backend\.venv" -ForegroundColor Green
}

# 2. Check if ports are already in use
$Port8000 = Get-NetTCPConnection -LocalPort 8000 -ErrorAction SilentlyContinue
if ($Port8000) {
    Write-Host "[WARN] Port 8000 is already in use by PID $($Port8000.OwningProcess). Stopping it..." -ForegroundColor Yellow
    Stop-Process -Id $Port8000.OwningProcess -Force -ErrorAction SilentlyContinue
}

$Port5173 = Get-NetTCPConnection -LocalPort 5173 -ErrorAction SilentlyContinue
if ($Port5173) {
    Write-Host "[WARN] Port 5173 is already in use by PID $($Port5173.OwningProcess). Stopping it..." -ForegroundColor Yellow
    Stop-Process -Id $Port5173.OwningProcess -Force -ErrorAction SilentlyContinue
}

# 3. Start Backend
Write-Host "[1/2] Starting Backend (FastAPI on http://localhost:8000)..." -ForegroundColor Cyan
$BackendDir = Join-Path $Root "backend"
$BackendProcess = Start-Process -FilePath "cmd.exe" `
    -ArgumentList "/k cd /d `"$BackendDir`" && set PYTHONPATH=. && `"$PythonExe`" -m uvicorn app.main:app --port 8000" `
    -PassThru

# 4. Start Frontend
Write-Host "[2/2] Starting Frontend (Vite on http://localhost:5173)..." -ForegroundColor Cyan
$FrontendDir = Join-Path $Root "frontend"
$FrontendProcess = Start-Process -FilePath "cmd.exe" `
    -ArgumentList "/k cd /d `"$FrontendDir`" && npm run dev" `
    -PassThru

Start-Sleep -Seconds 4

if (-not $NoBrowser) {
    Write-Host "Opening browser at http://localhost:5173..." -ForegroundColor Green
    Start-Process "http://localhost:5173"
}

Write-Host ""
Write-Host "========================================================" -ForegroundColor Green
Write-Host " Sumi is running!" -ForegroundColor Green
Write-Host " - Frontend Web App: http://localhost:5173"
Write-Host " - Backend API:      http://localhost:8000"
Write-Host " - API Docs:         http://localhost:8000/docs"
Write-Host ""
Write-Host " To stop Sumi, run .\scripts\stop-sumi.ps1 or .\stop-sumi.bat"
Write-Host "========================================================" -ForegroundColor Green
