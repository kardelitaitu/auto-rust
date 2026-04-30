#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Auto-rust CI Checker - Runs full test suite like GitHub workflow

.DESCRIPTION
    Runs cargo test, clippy, fmt, and build checks with detailed reporting.
    Similar to .github/workflows/ci.yml for local Windows development.

.EXAMPLE
    .\check.ps1
    .\check.ps1 -Verbose
    .\check.ps1 -Quick    # Skip release builds
#>
[CmdletBinding()]
param(
    [switch]$Quick,
    [switch]$SkipTests,
    [switch]$SkipClippy,
    [switch]$SkipFormat,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$startTime = Get-Date

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
    Write-Host "$c$message$r"
}

function Write-Section($title) {
    Write-Host ""
    Write-Status "═══ $title ═══" "Cyan"
}

# Results tracking
$results = @{
    Tests = @{ Passed = $false; Duration = 0; Output = "" }
    Format = @{ Passed = $false; Duration = 0; Output = "" }
    Clippy = @{ Passed = $false; Duration = 0; Output = "" }
    BuildDebug = @{ Passed = $false; Duration = 0; Output = "" }
    BuildRelease = @{ Passed = $false; Duration = 0; Output = "" }
}

# Check if in correct directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Status "ERROR: Must run from project root (where Cargo.toml is)" "Red"
    exit 1
}

Write-Status "🦀 Auto-rust CI Checker" "Blue"
Write-Status "Working Directory: $(Get-Location)" "Yellow"

# ============ TESTS ============
if (-not $SkipTests) {
    Write-Section "Running Tests (cargo test --all-features)"
    $testStart = Get-Date
    try {
        $output = cargo test --all-features --lib 2>&1
        $results.Tests.Output = $output -join "`n"
        
        # Parse test results
        $testLine = $output | Select-String "test result:" | Select-Object -Last 1
        if ($testLine) {
            Write-Status "✓ $testLine" "Green"
            if ($testLine -match "FAILED") {
                $results.Tests.Passed = $false
                Write-Status "✗ Tests failed" "Red"
            } else {
                $results.Tests.Passed = $true
            }
        } else {
            $results.Tests.Passed = $false
            Write-Status "✗ No test results found" "Red"
        }
    } catch {
        $results.Tests.Passed = $false
        Write-Status "✗ Tests failed with error" "Red"
        Write-Host $_
    }
    $results.Tests.Duration = ((Get-Date) - $testStart).TotalSeconds
}

# ============ FORMAT ============
if (-not $SkipFormat) {
    Write-Section "Checking Format (cargo fmt --all -- --check)"
    $fmtStart = Get-Date
    try {
        $output = cargo fmt --all -- --check 2>&1
        $results.Format.Output = $output -join "`n"
        if ($LASTEXITCODE -eq 0) {
            Write-Status "✓ Format check passed" "Green"
            $results.Format.Passed = $true
        } else {
            Write-Status "✗ Format check failed - run 'cargo fmt --all' to fix" "Red"
            $results.Format.Passed = $false
        }
    } catch {
        # fmt returns non-zero if there are issues
        if ($output -match "Diff in") {
            Write-Status "✗ Format check failed - files need formatting" "Red"
            Write-Host $output
            $results.Format.Passed = $false
        } else {
            $results.Format.Passed = $true
        }
    }
    $results.Format.Duration = ((Get-Date) - $fmtStart).TotalSeconds
}

# ============ CLIPPY ============
if (-not $SkipClippy) {
    Write-Section "Running Clippy (cargo clippy --all-targets --all-features -- -D warnings)"
    $clipStart = Get-Date
    try {
        $output = cargo clippy --all-targets --all-features -- -D warnings 2>&1
        $results.Clippy.Output = $output -join "`n"
        
        if ($LASTEXITCODE -eq 0) {
            Write-Status "✓ Clippy passed" "Green"
            $results.Clippy.Passed = $true
        } else {
            Write-Status "✗ Clippy found warnings/errors" "Red"
            Write-Host ($output | Select-String "warning:|error:" | Select-Object -First 10)
            $results.Clippy.Passed = $false
        }
    } catch {
        $results.Clippy.Passed = $false
        Write-Status "✗ Clippy failed" "Red"
    }
    $results.Clippy.Duration = ((Get-Date) - $clipStart).TotalSeconds
}

# ============ BUILD DEBUG ============
if (-not $SkipBuild) {
    Write-Section "Checking Build (cargo check)"
    $buildStart = Get-Date
    try {
        $output = cargo check 2>&1
        $results.BuildDebug.Output = $output -join "`n"
        if ($LASTEXITCODE -eq 0) {
            Write-Status "✓ Debug build check passed" "Green"
            $results.BuildDebug.Passed = $true
        } else {
            Write-Status "✗ Debug build failed" "Red"
            $results.BuildDebug.Passed = $false
        }
    } catch {
        $results.BuildDebug.Passed = $false
    }
    $results.BuildDebug.Duration = ((Get-Date) - $buildStart).TotalSeconds

    # ============ BUILD RELEASE ============
    if (-not $Quick) {
        Write-Section "Checking Release Build (cargo check --release)"
        $relStart = Get-Date
        try {
            $output = cargo check --release 2>&1
            $results.BuildRelease.Output = $output -join "`n"
            if ($LASTEXITCODE -eq 0) {
                Write-Status "✓ Release build check passed" "Green"
                $results.BuildRelease.Passed = $true
            } else {
                Write-Status "✗ Release build failed" "Red"
                $results.BuildRelease.Passed = $false
            }
        } catch {
            $results.BuildRelease.Passed = $false
        }
        $results.BuildRelease.Duration = ((Get-Date) - $relStart).TotalSeconds
    } else {
        Write-Status "⚡ Skipping release build (--Quick mode)" "Yellow"
    }
}

# ============ SUMMARY REPORT ============
$totalDuration = ((Get-Date) - $startTime).TotalSeconds

Write-Host ""
Write-Status "═══════════════════════════════════════" "Cyan"
Write-Status "           CI CHECKER REPORT           " "Cyan"
Write-Status "═══════════════════════════════════════" "Cyan"
Write-Host ""

$passed = 0
$failed = 0

foreach ($check in $results.GetEnumerator() | Sort-Object Key) {
    $name = $check.Key
    $result = $check.Value
    
    if ($result.Duration -gt 0 -or $result.Passed) {
        $status = if ($result.Passed) { "✓ PASS" } else { "✗ FAIL" }
        $color = if ($result.Passed) { "Green" } else { "Red" }
        $duration = "{0:N2}s" -f $result.Duration
        
        Write-Status ($status.PadRight(8) + " $name".PadRight(25) + $duration.PadLeft(8)) $color
        
        if ($result.Passed) { $passed++ } else { $failed++ }
    }
}

Write-Host ""
Write-Status "────────────────────────────────────────" "Cyan"
Write-Status ("Passed: $passed  |  Failed: $failed  |  Total Time: {0:N2}s" -f $totalDuration) $(if ($failed -eq 0) { "Green" } else { "Red" })
Write-Status "────────────────────────────────────────" "Cyan"
Write-Host ""

# Exit code
if ($failed -eq 0) {
    Write-Status "🎉 All checks passed! Ready for commit." "Green"
    exit 0
} else {
    Write-Status "💥 Some checks failed. Fix before committing." "Red"
    exit 1
}
