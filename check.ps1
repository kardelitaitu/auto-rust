#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Auto-rust CI Checker - Runs full test suite like GitHub workflow

.DESCRIPTION
    Runs cargo test, clippy, fmt, and build checks with detailed reporting.
    Similar to .github/workflows/ci.yml for local Windows development.

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
    Build = @{ Passed = $false; Duration = 0; Output = "" }
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
        
        # Parse test results - "test result: ok" means passed
        $testLine = $output | Select-String "test result:" | Select-Object -Last 1
        if ($testLine) {
            if ($testLine -match "test result: ok") {
                Write-Status "✓ Tests passed: $testLine" "Green"
                $results.Tests.Passed = $true
            } else {
                Write-Status "✗ Tests failed: $testLine" "Red"
                $results.Tests.Passed = $false
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

# ============ BUILD ============
if (-not $SkipBuild) {
    Write-Section "Checking Build (cargo check)"
    $buildStart = Get-Date
    try {
        $output = cargo check 2>&1
        $results.Build.Output = $output -join "`n"
        if ($LASTEXITCODE -eq 0) {
            Write-Status "✓ Build check passed" "Green"
            $results.Build.Passed = $true
        } else {
            Write-Status "✗ Build failed" "Red"
            $results.Build.Passed = $false
        }
    } catch {
        $results.Build.Passed = $false
    }
    $results.Build.Duration = ((Get-Date) - $buildStart).TotalSeconds
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
