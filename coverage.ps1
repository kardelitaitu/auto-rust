#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Generate Rust coverage reports into target/reports/coverage.
.DESCRIPTION
    Runs cargo tarpaulin and writes the HTML report under target/reports/coverage
    so coverage artifacts stay out of the repo root.
.EXAMPLE
    .\coverage.ps1
#>
[CmdletBinding()]
param(
    [string]$OutputDir
)

$ErrorActionPreference = "Stop"

if (-not $OutputDir) {
    $OutputDir = Join-Path $PSScriptRoot "target\reports\coverage"
}

if (-not (Test-Path "Cargo.toml")) {
    Write-Error "Must run from project root (where Cargo.toml is)"
    exit 1
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

Write-Host "Running cargo tarpaulin..."
Write-Host "Output dir: $OutputDir"

cargo tarpaulin --out Html --output-dir $OutputDir
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo tarpaulin failed"
    exit $LASTEXITCODE
}

$reportPath = Join-Path $OutputDir "tarpaulin-report.html"
Write-Host "Coverage report written to $reportPath"
