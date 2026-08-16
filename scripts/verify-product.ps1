param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot

& (Join-Path $PSScriptRoot "verify-v2.ps1")
& (Join-Path $PSScriptRoot "run-product-uat.ps1")

Write-Host "Sumi product verification passed."
