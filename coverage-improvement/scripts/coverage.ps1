#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Generate Rust coverage reports and integrate with coverage improvement workflow.
.DESCRIPTION
    Runs cargo tarpaulin with JSON and HTML output formats. Analyzes coverage gaps
    and creates prioritized improvement items for systematic test coverage improvement.
.EXAMPLE
    .\coverage.ps1
    .\coverage.ps1 -OutputDir "custom/coverage"
    .\coverage.ps1 -Analyze
    .\coverage.ps1 -Loop
#>
[CmdletBinding()]
param(
    [string]$OutputDir,
    [switch]$Analyze,
    [switch]$Loop,
    [switch]$ShowOutput  # show all cargo output
)

$sw = [System.Diagnostics.Stopwatch]::StartNew()
$ErrorActionPreference = "Stop"

if (-not $OutputDir) {
    $OutputDir = Join-Path $PSScriptRoot "target/reports/coverage"
}

# Walk up from the script directory to find the project root with Cargo.toml
$projectRoot = $PSScriptRoot
while ($projectRoot -and -not (Test-Path (Join-Path $projectRoot "Cargo.toml"))) {
    $projectRoot = Split-Path $projectRoot -Parent
}
if (-not $projectRoot -or -not (Test-Path (Join-Path $projectRoot "Cargo.toml"))) {
    Write-Error "Could not find Cargo.toml."
    exit 1
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

Write-Host "Running cargo tarpaulin (this takes a minute)..."

# Build tarpaulin args
$tarpaulinArgs = @(
    "tarpaulin",
    "--out", "Json",
    "--out", "Html",
    "--output-dir", $OutputDir,
    "--all-features",
    "-l"
)

# Run tarpaulin from project root, suppress output by default
Push-Location $projectRoot
try {
    $prevEap = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    if ($ShowOutput) {
        & cargo @tarpaulinArgs
    } else {
        & cargo @tarpaulinArgs 2>&1 | Out-Null
    }
    $ErrorActionPreference = $prevEap
    if ($LASTEXITCODE -ne 0) {
        Write-Error "cargo tarpaulin failed (exit $LASTEXITCODE)"
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
}

# Copy JSON to scripts directory for analysis
$jsonReport = Join-Path $OutputDir "tarpaulin-report.json"
$analysisJson = Join-Path $PSScriptRoot "coverage.json"
if (Test-Path $jsonReport) {
    Copy-Item -Path $jsonReport -Destination $analysisJson -Force
}

$needsAnalysis = $Analyze -or $Loop
if ($needsAnalysis) {
    if (-not (Test-Path $analysisJson)) {
        Write-Error "Coverage JSON report not found: $analysisJson"
        exit 1
    }

    $mgr = Join-Path $PSScriptRoot "coverage_improvement_manager.ps1"
    if (-not (Test-Path $mgr)) {
        Write-Error "Coverage improvement manager not found: $mgr"
        exit 1
    }

    Write-Host "Analyzing coverage gaps..."
    & $mgr Generate -CoverageFile $analysisJson -Quiet
    Write-Host "Coverage improvement data written."
}

if ($Loop) {
    Write-Host "`n=== Coverage Improvement Loop ==="
    Write-Host "1. Write tests for identified gaps"
    Write-Host "2. Run: cargo test"
    Write-Host "3. Mark items: .\coverage_improvement_manager.ps1 Complete 'ID'"
    Write-Host "4. Repeat: .\coverage.ps1 -Loop"
    Write-Host "`nPending items:"
    & (Join-Path $PSScriptRoot "coverage_improvement_manager.ps1") List
}

Write-Host "Done in $($sw.Elapsed.TotalSeconds.ToString('F1'))s" -ForegroundColor Cyan
