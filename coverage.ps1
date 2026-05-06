#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Generate Rust coverage reports into target/reports/coverage.
.DESCRIPTION
    Runs cargo llvm-cov and writes HTML and machine-readable reports under target/reports/coverage
    so coverage artifacts stay out of the repo root.
.EXAMPLE
    .\coverage.ps1
    .\coverage.ps1 -OutputDir "custom/coverage"
#>
[CmdletBinding()]
param(
    [string]$OutputDir,
    [switch]$Json,
    [switch]$Lcov,
    [switch]$Html = $true
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

Write-Host "Running cargo llvm-cov..."
Write-Host "Output dir: $OutputDir"

# Build the cargo llvm-cov command with appropriate output formats
$covArgs = @("llvm-cov", "--workspace", "--all-features")

# Determine output format (mutually exclusive)
if ($Html -and ($Json -or $Lcov)) {
    Write-Error "Cannot specify --Html together with --Json or --Lcov"
    exit 1
}

if ($Json -and $Lcov) {
    Write-Error "Cannot specify --Json together with --Lcov"
    exit 1
}

if ($Html) {
    $htmlDir = Join-Path $OutputDir "html"
    $covArgs += "--html"
    $covArgs += "--output-dir", $htmlDir
    $summaryPath = Join-Path $OutputDir "coverage.json"
}
elseif ($Json) {
    $jsonPath = Join-Path $OutputDir "coverage.json"
    $covArgs += "--json"
    $covArgs += "--output-path", $jsonPath
    $summaryPath = $jsonPath
}
elseif ($Lcov) {
    $lcovPath = Join-Path $OutputDir "coverage.lcov"
    $covArgs += "--lcov"
    $covArgs += "--output-path", $lcovPath
    $summaryPath = $lcovPath
}
else {
    # Default to HTML if no format specified
    $htmlDir = Join-Path $OutputDir "html"
    $covArgs += "--html"
    $covArgs += "--output-dir", $htmlDir
    $summaryPath = Join-Path $OutputDir "coverage.json"
}

# Run coverage command first
Write-Host "Command: cargo $($covArgs -join ' ')"
& cargo @covArgs
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo llvm-cov failed"
    exit $LASTEXITCODE
}

# Generate JSON summary separately if HTML was requested
if ($Html) {
    Write-Host "Generating separate JSON summary..."
    $jsonArgs = @("llvm-cov", "--workspace", "--all-features", "--json", "--output-path", $summaryPath)
    & cargo @jsonArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "Failed to generate JSON summary, but HTML report was created"
    }
}

# Report generated files
if ($Html) {
    $reportPath = Join-Path $OutputDir "html\index.html"
    Write-Host "HTML coverage report written to $reportPath"
}

if ($Json) {
    Write-Host "JSON coverage report written to $jsonPath"
}

if ($Lcov) {
    Write-Host "LCOV coverage report written to $lcovPath"
}

Write-Host "Coverage summary written to $summaryPath"
