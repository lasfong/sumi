param(
  [switch]$BrowserSmoke
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot

Write-Host "== Sumi V2 backend tests =="
Push-Location (Join-Path $Root "backend")
try {
  & .\.venv\Scripts\python.exe -m pytest app\tests -q

  Write-Host "== Sumi V2 Alembic upgrade =="
  & .\.venv\Scripts\alembic.exe upgrade head
}
finally {
  Pop-Location
}

Write-Host "== Sumi V2 frontend lint/test/build =="
Push-Location (Join-Path $Root "frontend")
try {
  npm.cmd run lint
  npm.cmd run test
  npm.cmd run build
  if ($BrowserSmoke) {
    Write-Host "== Sumi V2 browser smoke =="
    npm.cmd run smoke:browser
  }
}
finally {
  Pop-Location
}

Write-Host "== Sumi V2 verification complete =="
