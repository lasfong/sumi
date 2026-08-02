param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$PythonBin = Join-Path $Root "backend\.venv\Scripts\python.exe"
$BackendPort = 18000
$FrontendPort = 15173
$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("sumi-uat-" + [Guid]::NewGuid().ToString('N').Substring(0,8))
New-Item -ItemType Directory -Path $TempDir -Force | Out-Null
$DatabasePath = (Join-Path $TempDir "sumi-uat.db").Replace('\', '/')
$ArtifactDir = Join-Path $Root "test-results\product-uat"

Write-Host "== Preparing temporary DB at $DatabasePath =="
$env:DATABASE_URL = "sqlite:///$DatabasePath"
Push-Location (Join-Path $Root "backend")
try {
  & $PythonBin -m alembic upgrade head
  & $PythonBin scripts/seed_demo.py
} finally {
  Pop-Location
}

Write-Host "== Starting Backend on port $BackendPort =="
$backendEnv = @{
  "DATABASE_URL" = "sqlite:///$DatabasePath"
  "CORS_ALLOWED_ORIGINS" = "http://127.0.0.1:$FrontendPort"
}

# Start backend process
$backendProcess = New-Object System.Diagnostics.Process
$backendProcess.StartInfo.FileName = $PythonBin
$backendProcess.StartInfo.Arguments = "-m uvicorn app.main:app --host 127.0.0.1 --port $BackendPort"
$backendProcess.StartInfo.WorkingDirectory = Join-Path $Root "backend"
$backendProcess.StartInfo.UseShellExecute = $false
$backendProcess.StartInfo.EnvironmentVariables["DATABASE_URL"] = "sqlite:///$DatabasePath"
$backendProcess.StartInfo.EnvironmentVariables["CORS_ALLOWED_ORIGINS"] = "http://127.0.0.1:$FrontendPort"
$backendProcess.Start() | Out-Null

Write-Host "== Starting Frontend on port $FrontendPort =="
$frontendProcess = New-Object System.Diagnostics.Process
$frontendProcess.StartInfo.FileName = "cmd.exe"
$frontendProcess.StartInfo.Arguments = "/c npm run dev -- --host 127.0.0.1 --port $FrontendPort"
$frontendProcess.StartInfo.WorkingDirectory = Join-Path $Root "frontend"
$frontendProcess.StartInfo.UseShellExecute = $false
$frontendProcess.StartInfo.EnvironmentVariables["SUMI_API_TARGET"] = "http://127.0.0.1:$BackendPort"
$frontendProcess.Start() | Out-Null

try {
  Write-Host "== Waiting for services to become healthy =="
  $healthy = $false
  for ($i = 0; $i -lt 40; $i++) {
    Start-Sleep -Milliseconds 500
    try {
      $r1 = Invoke-WebRequest -Uri "http://127.0.0.1:$BackendPort/api/health" -UseBasicParsing -TimeoutSec 1
      $r2 = Invoke-WebRequest -Uri "http://127.0.0.1:$FrontendPort" -UseBasicParsing -TimeoutSec 1
      if ($r1.StatusCode -eq 200 -and $r2.StatusCode -eq 200) {
        $healthy = $true
        break
      }
    } catch {}
  }

  if (-not $healthy) {
    throw "Backend or Frontend failed to start"
  }

  Write-Host "== Running Product UAT script =="
  $env:SUMI_FRONTEND_URL = "http://127.0.0.1:$FrontendPort"
  $env:SUMI_BACKEND_URL = "http://127.0.0.1:$BackendPort"
  $env:SUMI_PRODUCT_UAT_ARTIFACT_DIR = $ArtifactDir
  $env:SUMI_UAT_DATABASE_PATH = $DatabasePath
  $env:SUMI_PRODUCTION_DATABASE_PATH = Join-Path $Root "backend\sumi.db"
  node scripts/product-uat.mjs
  if ($LASTEXITCODE -ne 0) {
    throw "Product UAT failed with exit code $LASTEXITCODE"
  }
} finally {
  Write-Host "== Cleaning up background processes =="
  if ($backendProcess -and -not $backendProcess.HasExited) {
    & taskkill.exe /PID $backendProcess.Id /T /F | Out-Null
  }
  if ($frontendProcess -and -not $frontendProcess.HasExited) {
    & taskkill.exe /PID $frontendProcess.Id /T /F | Out-Null
  }
  Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}
