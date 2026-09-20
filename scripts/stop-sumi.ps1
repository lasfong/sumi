<#
.SYNOPSIS
    Stops all Sumi Local Replay Workstation processes (ports 8000 and 5173).
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "SilentlyContinue"

Write-Host "========================================================" -ForegroundColor Yellow
Write-Host "       Stopping Sumi Local Replay Workstation           " -ForegroundColor Yellow
Write-Host "========================================================" -ForegroundColor Yellow

$stopped = 0

# Check Port 8000
$Port8000 = Get-NetTCPConnection -LocalPort 8000 -ErrorAction SilentlyContinue
if ($Port8000) {
    foreach ($conn in $Port8000) {
        $proc = Get-Process -Id $conn.OwningProcess -ErrorAction SilentlyContinue
        if ($proc) {
            Write-Host "Stopping Backend process: $($proc.ProcessName) (PID: $($proc.Id))" -ForegroundColor Cyan
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
            $stopped++
        }
    }
}

# Check Port 5173
$Port5173 = Get-NetTCPConnection -LocalPort 5173 -ErrorAction SilentlyContinue
if ($Port5173) {
    foreach ($conn in $Port5173) {
        $proc = Get-Process -Id $conn.OwningProcess -ErrorAction SilentlyContinue
        if ($proc) {
            Write-Host "Stopping Frontend process: $($proc.ProcessName) (PID: $($proc.Id))" -ForegroundColor Cyan
            Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
            $stopped++
        }
    }
}

Write-Host ""
if ($stopped -gt 0) {
    Write-Host "[OK] Stopped $stopped Sumi server processes." -ForegroundColor Green
} else {
    Write-Host "[INFO] No running Sumi server processes found on port 8000 or 5173." -ForegroundColor Yellow
}
