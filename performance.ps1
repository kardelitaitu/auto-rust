#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Generate local nextest performance reports.
.DESCRIPTION
    Runs nextest inventory and slow-test review commands, then writes the
    captured output to target/reports/performance for later inspection.
.EXAMPLE
    .\performance.ps1
    .\performance.ps1 -Inventory
    .\performance.ps1 -Slow
#>
[CmdletBinding()]
param(
    [switch]$Inventory,
    [switch]$Slow,
    [string]$ReportDir
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path "Cargo.toml")) {
    Write-Error "Must run from project root (where Cargo.toml is)"
    exit 1
}

if (-not $Inventory -and -not $Slow) {
    $Inventory = $true
    $Slow = $true
}

if (-not $ReportDir) {
    $ReportDir = Join-Path $PSScriptRoot "target\reports\performance"
}

New-Item -ItemType Directory -Force -Path $ReportDir | Out-Null

function Invoke-NextestReport {
    param(
        [Parameter(Mandatory = $true)][string]$Label,
        [Parameter(Mandatory = $true)][string[]]$Args,
        [Parameter(Mandatory = $true)][string]$OutputFile
    )

    Write-Host "Running $Label..."
    Write-Host "Command: cargo $($Args -join ' ')"

    $stdoutFile = Join-Path $ReportDir "$Label.stdout.txt"
    $stderrFile = Join-Path $ReportDir "$Label.stderr.txt"

    $proc = Start-Process cargo -ArgumentList $Args -NoNewWindow -Wait -PassThru `
        -RedirectStandardOutput $stdoutFile -RedirectStandardError $stderrFile

    $lines = @()
    if (Test-Path $stdoutFile) {
        $lines += Get-Content $stdoutFile
    }
    if (Test-Path $stderrFile) {
        $lines += Get-Content $stderrFile
    }
    $lines | Set-Content $OutputFile

    if ($proc.ExitCode -ne 0) {
        Write-Error "cargo nextest $Label failed"
        exit $proc.ExitCode
    }

    $lineArray = @($lines)
    $slowLines = @($lineArray | Where-Object { $_ -match '\bSLOW\b' })
    $summaryLines = @($lineArray | Where-Object { $_ -match 'Summary \[' })

    if ($slowLines.Count -gt 0) {
        Write-Host "$Label slow lines: $($slowLines.Count)"
    }

    if ($Label -eq "inventory") {
        Write-Host "Inventory lines captured: $($lineArray.Count)"
    }
    elseif ($summaryLines.Count -gt 0) {
        Write-Host "Summary: $($summaryLines[-1])"
    }

    Write-Host "Report written to $OutputFile"

    Remove-Item $stdoutFile, $stderrFile -ErrorAction SilentlyContinue
}

if ($Inventory) {
    Invoke-NextestReport `
        -Label "inventory" `
        -Args @("nextest", "list", "--all-features", "--verbose") `
        -OutputFile (Join-Path $ReportDir "nextest-inventory.txt")
}

if ($Slow) {
    Invoke-NextestReport `
        -Label "slow-test review" `
        -Args @("nextest", "run", "--all-features", "--profile", "ci", "--status-level", "slow", "--final-status-level", "slow") `
        -OutputFile (Join-Path $ReportDir "nextest-slow-review.txt")
}
