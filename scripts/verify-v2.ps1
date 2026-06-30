param(
  [switch]$BrowserSmoke
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot

function Invoke-Checked {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Name,

    [Parameter(Mandatory = $true)]
    [scriptblock]$Command
  )

  & $Command
  $succeeded = $?
  $exitCode = $LASTEXITCODE
  if ((-not $succeeded) -or ($null -ne $exitCode -and $exitCode -ne 0)) {
    throw "$Name failed with exit code $exitCode"
  }
}

Write-Host "== Sumi V2 backend tests =="
Push-Location (Join-Path $Root "backend")
try {
  Invoke-Checked "Backend pytest" { & .\.venv\Scripts\python.exe -m pytest app\tests -q }

  Write-Host "== Sumi V2 Alembic upgrade =="
  Invoke-Checked "Alembic upgrade" { & .\.venv\Scripts\alembic.exe upgrade head }
}
finally {
  Pop-Location
}

Write-Host "== Sumi V2 frontend lint/test/build =="
Push-Location (Join-Path $Root "frontend")
try {
  Invoke-Checked "Frontend lint" { npm.cmd run lint }
  Invoke-Checked "Frontend tests" { npm.cmd run test }
  Invoke-Checked "Frontend build" { npm.cmd run build }
  if ($BrowserSmoke) {
    Write-Host "== Sumi V2 browser smoke =="
    Invoke-Checked "Browser smoke" { npm.cmd run smoke:browser }
  }
}
finally {
  Pop-Location
}

Write-Host "== Sumi V2 verification complete =="
