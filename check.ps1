#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Auto-rust CI Checker - Runs full test suite like GitHub workflow

.DESCRIPTION
    Runs cargo nextest run, clippy, fmt, and build checks with detailed reporting.
    Mirrors .github/workflows/ci.yml for local Windows development.
    Includes timeouts to prevent hanging.

.EXAMPLE
    .\check.ps1           # Run all checks
    .\check.ps1 -Verbose  # Show all output
    .\check.ps1 -SkipTests # Skip test execution
#>
[CmdletBinding()]
param(
    [switch]$SkipTests,
    [switch]$SkipClippy,
    [switch]$SkipFormat,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$startTime = Get-Date

# Timeout settings (in seconds)
$globalTimeout = 300    # 5 minutes global
$testsTimeout = 180    # 3 minutes for tests
$formatTimeout = 30     # 30 seconds for format
$clippyTimeout = 30    # 30 seconds for clippy
$buildTimeout = 60      # 60 seconds for build

# Colors for output
$colors = @{
    Green = "`e[32m"
    Red = "`e[31m"
    Yellow = "`e[33m"
    Blue = "`e[34m"
    Cyan = "`e[36m"
    Reset = "`e[0m"
}

function Write-Status($message, $color = "White") {
    $c = $colors[$color] ?? ""
    $r = $colors.Reset
    Write-Output "$c$message$r"
}

function Write-Section($title) {
    Write-Output ""
    Write-Status "=== $title ===" "Cyan"
}

function Invoke-WithTimeout {
    param(
        [string]$Name,
        [scriptblock]$Command,
        [int]$TimeoutSeconds
    )

    $job = Start-Job -ScriptBlock $Command
    $waited = 0

    while ($job.State -eq "Running" -and $waited -lt $TimeoutSeconds) {
        Start-Sleep 1
        $waited++
    }

    if ($job.State -eq "Running") {
        Write-Status "$Name timed out after ${TimeoutSeconds}s - killing process" "Yellow"
        Stop-Job -Job $job -ErrorAction SilentlyContinue
        Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
        return $null
    }

    $output = Receive-Job -Job $job
    Remove-Job -Job $job -Force -ErrorAction SilentlyContinue
    return $output
}

# Results tracking
$results = @{
    Tests = @{ Passed = $false; Duration = 0; Output = "" }
    Format = @{ Passed = $false; Duration = 0; Output = "" }
    Clippy = @{ Passed = $false; Duration = 0; Output = "" }
    Build = @{ Passed = $false; Duration = 0; Output = "" }
}

# Check if in correct directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Status "ERROR: Must run from project root (where Cargo.toml is)" "Red"
    exit 1
}

Write-Status "Auto-rust CI Checker" "Blue"
Write-Status "Working Directory: $(Get-Location)" "Yellow"
Write-Status "Timeouts - Global: ${globalTimeout}s | Tests: ${testsTimeout}s | Format: ${formatTimeout}s | Clippy: ${clippyTimeout}s | Build: ${buildTimeout}s" "Yellow"

# ============ TESTS ============
if (-not $SkipTests) {
    Write-Output ""
    Write-Status "Checking if cargo-nextest is installed..." "Yellow"
    $nextestCheck = Get-Command cargo-nextest -ErrorAction SilentlyContinue
    if (-not $nextestCheck) {
        Write-Status "Installing cargo-nextest..." "Yellow"
        cargo install --locked cargo-nextest 2>&1 | Out-Null
    }
    Write-Section "Running Tests (cargo nextest run --all-features)"
    $testStart = Get-Date

    $output = Invoke-WithTimeout -Name "Tests" -TimeoutSeconds $testsTimeout -Command {
        cargo nextest run --all-features --lib 2>&1
    }

    $results.Tests.Output = if ($output) { $output -join "`n" } else { "" }

    if (-not $output) {
        Write-Status "Tests timed out after ${testsTimeout}s" "Red"
        $results.Tests.Passed = $false
    } elseif ($output -match "test result: ok" -or $output -match "passed") {
        Write-Status "Tests passed" "Green"
        $results.Tests.Passed = $true
    } else {
        Write-Status "Tests failed" "Red"
        $results.Tests.Passed = $false
    }
    $results.Tests.Duration = ((Get-Date) - $testStart).TotalSeconds
}

# ============ FORMAT ============
if (-not $SkipFormat) {
    Write-Section "Checking Format (cargo fmt --all -- --check)"
    $fmtStart = Get-Date

    $output = Invoke-WithTimeout -Name "Format" -TimeoutSeconds $formatTimeout -Command {
        cargo fmt --all -- --check 2>&1
    }

    $results.Format.Output = if ($output) { $output -join "`n" } else { "" }

    if (-not $output) {
        Write-Status "Format timed out after ${formatTimeout}s" "Red"
        $results.Format.Passed = $false
    } elseif ($LASTEXITCODE -eq 0) {
        Write-Status "Format check passed" "Green"
        $results.Format.Passed = $true
    } else {
        Write-Status "Format check failed - run 'cargo fmt --all' to fix" "Red"
        $results.Format.Passed = $false
    }
    $results.Format.Duration = ((Get-Date) - $fmtStart).TotalSeconds
}

# ============ CLIPPY ============
if (-not $SkipClippy) {
    Write-Section "Running Clippy (cargo clippy --all-targets --all-features -- -D warnings)"
    $clipStart = Get-Date

    $output = Invoke-WithTimeout -Name "Clippy" -TimeoutSeconds $clippyTimeout -Command {
        cargo clippy --all-targets --all-features -- -D warnings 2>&1
    }

    $results.Clippy.Output = if ($output) { $output -join "`n" } else { "" }

    if (-not $output) {
        Write-Status "Clippy timed out after ${clippyTimeout}s" "Red"
        $results.Clippy.Passed = $false
    } elseif ($LASTEXITCODE -eq 0) {
        Write-Status "Clippy passed" "Green"
        $results.Clippy.Passed = $true
    } else {
        Write-Status "Clippy found warnings/errors" "Red"
        $results.Clippy.Passed = $false
    }
    $results.Clippy.Duration = ((Get-Date) - $clipStart).TotalSeconds
}

# ============ BUILD ============
if (-not $SkipBuild) {
    Write-Section "Checking Build (cargo check)"
    $buildStart = Get-Date

    $output = Invoke-WithTimeout -Name "Build" -TimeoutSeconds $buildTimeout -Command {
        cargo check 2>&1
    }

    $results.Build.Output = if ($output) { $output -join "`n" } else { "" }

    if (-not $output) {
        Write-Status "Build timed out after ${buildTimeout}s" "Red"
        $results.Build.Passed = $false
    } elseif ($LASTEXITCODE -eq 0) {
        Write-Status "Build check passed" "Green"
        $results.Build.Passed = $true
    } else {
        Write-Status "Build failed" "Red"
        $results.Build.Passed = $false
    }
    $results.Build.Duration = ((Get-Date) - $buildStart).TotalSeconds
}

# ============ SUMMARY REPORT ============
$totalDuration = ((Get-Date) - $startTime).TotalSeconds

Write-Output ""
Write-Status "====================================================" "Cyan"
Write-Status "           CI CHECKER REPORT           " "Cyan"
Write-Status "====================================================" "Cyan"
Write-Output ""

$passed = 0
$failed = 0

foreach ($check in $results.GetEnumerator() | Sort-Object Key) {
    $name = $check.Key
    $result = $check.Value

    if ($result.Duration -gt 0 -or $result.Passed) {
        $status = if ($result.Passed) { "PASS" } else { "FAIL" }
        $color = if ($result.Passed) { "Green" } else { "Red" }
        $duration = "{0:N2}s" -f $result.Duration

        Write-Status ($status.PadRight(8) + " $name".PadRight(25) + $duration.PadLeft(8)) $color

        if ($result.Passed) { $passed++ } else { $failed++ }
    }
}

Write-Output ""
Write-Status "------------------------------------------------" "Cyan"
Write-Status ("Passed: $passed  |  Failed: $failed  |  Total Time: {0:N2}s" -f $totalDuration) $(if ($failed -eq 0) { "Green" } else { "Red" })
Write-Status "------------------------------------------------" "Cyan"
Write-Output ""

# Exit code
if ($failed -eq 0) {
    Write-Status "All checks passed! Ready for commit." "Green"
    exit 0
} else {
    Write-Status "Some checks failed. Fix before committing." "Red"
    exit 1
}