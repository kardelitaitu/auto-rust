#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Twitter Activity TDD Test Runner - supports Red-Green-Refactor cycle.
.DESCRIPTION
    Runs twitteractivity tests with various modes:
    - Default: Run all twitter tests once
    - Watch: Watch mode (re-runs on file changes)
    - Coverage: Run with coverage instrumentation
    - Profile: List slow tests for optimization
    - Red: Run only RED-marked tests (expected failures)
    - Green: Run only GREEN-marked tests (passing validation)
    - Refactor: Run all tests (verify green after refactoring)
.EXAMPLE
    .\run-twitter-tests.ps1
    .\run-twitter-tests.ps1 -Watch
    .\run-twitter-tests.ps1 -Coverage
    .\run-twitter-tests.ps1 -Profile
    .\run-twitter-tests.ps1 -Red
    .\run-twitter-tests.ps1 -Green
    .\run-twitter-tests.ps1 -Refactor
.PARAMETER Watch
    Watch mode - uses cargo watch to re-run tests on file changes
.PARAMETER Coverage
    Run with LLVM coverage instrumentation via CARGO_INCREMENTAL=0
.PARAMETER Profile
    List slowest tests for performance optimization
.PARAMETER Red
    Run tests tagged with RED markers (expected to fail)
.PARAMETER Green
    Run tests tagged with GREEN markers (passing validation)
.PARAMETER Refactor
    Run all tests to verify green after refactoring
.PARAMETER Fast
    Only run unit tests (skip integration tests)
.PARAMETER Integration
    Only run integration tests (skip unit tests)
.PARAMETER Filter
    Additional test filter string passed to cargo test
.PARAMETER NoLog
    Suppress output to log file
#>

[CmdletBinding()]
param(
    [switch]$Watch,
    [switch]$Coverage,
    [switch]$Profile,
    [switch]$Red,
    [switch]$Green,
    [switch]$Refactor,
    [switch]$Fast,
    [switch]$Integration,
    [string]$Filter = "",
    [switch]$NoLog
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

$LogDir = "bacon-logs"
$Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"

# ============================================================================
# Helper Functions
# ============================================================================

function Write-Banner {
    param([string]$Mode)
    Write-Host ""
    Write-Host "========================================================" -ForegroundColor Cyan
    Write-Host "  Twitter Activity TDD Test Runner" -ForegroundColor Cyan
    Write-Host "  Mode: $Mode" -ForegroundColor Cyan
    Write-Host "  Time: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" -ForegroundColor Cyan
    Write-Host "========================================================" -ForegroundColor Cyan
    Write-Host ""
}

function Invoke-CargoTest {
    param(
        [string[]]$TestArgs,
        [string]$ModeLabel,
        [switch]$TddProfile
    )

    $cmdArgs = @("test")

    if ($TddProfile) {
        $cmdArgs += "--profile", "tdd"
    }

    if ($Fast) {
        $cmdArgs += "--lib"
    } elseif ($Integration) {
        $cmdArgs += "--test", "twitteractivity_integration"
    } else {
        $cmdArgs += "--lib"
        $cmdArgs += "--test", "twitteractivity_integration"
    }

    # Single `--` separator before all test-binary arguments
    $cmdArgs += "--"
    if ($TddProfile) {
        $cmdArgs += "--test-threads", "1"
    }
    if ($TestArgs) {
        $cmdArgs += $TestArgs
    }
    if ($Filter) {
        $cmdArgs += $Filter
    }

    if ($Coverage) {
        $env:CARGO_INCREMENTAL = "0"
        $env:RUSTFLAGS = "-Cinstrument-coverage"
        $env:LLVM_PROFILE_FILE = "twitter-test-%p-%m.profraw"
    }

    if ($Profile) {
        $cmdArgs = @("test", "--", "-Zunstable-options", "--report-time")
        if ($Filter) {
            $cmdArgs += $Filter
        }
    }

    if ($ModeLabel) {
        Write-Host "Running: cargo $($cmdArgs -join ' ')" -ForegroundColor Gray
    }

    $logFile = if (-not $NoLog) {
        $lf = Join-Path $LogDir "twitter-tests_${Timestamp}_${ModeLabel}.log"
        New-Item -ItemType Directory -Force -Path $LogDir | Out-Null
        $lf
    } else {
        $null
    }

    if ($logFile) {
        Write-Host "Log: $logFile" -ForegroundColor Gray
    }

    # Run cargo, merge stderr into stdout so output is visible in the terminal
    $output = & cargo @cmdArgs 2>&1
    $exitCode = $LASTEXITCODE
    # Send to terminal via host output (works reliably via pipe/redirection)
    $output | Out-String | Write-Host
    # Persist to log file
    if ($logFile) {
        $output | Out-File -FilePath $logFile -Encoding utf8
    }

    if ($Coverage) {
        Remove-Item -Path "twitter-test-*.profraw" -Force -ErrorAction SilentlyContinue
    }

    return $exitCode
}

# ============================================================================
# Mode Dispatch
# ============================================================================

if ($Red) {
    Write-Banner "RED (Expected Failures)"
    Write-Host "Running RED-marked tests to verify they fail as expected..." -ForegroundColor Yellow
    Write-Host "These tests demonstrate desired but unimplemented behavior." -ForegroundColor Yellow
    Write-Host ""
    $exitCode = Invoke-CargoTest -TestArgs @("--", "tdd_red") -ModeLabel "RED" -TddProfile
    if ($exitCode -eq 0) {
        Write-Host ""
        Write-Host "⚠  RED tests PASSED — behavior already implemented." -ForegroundColor Yellow
        Write-Host "   These tests describe existing behavior. Move them to" -ForegroundColor Yellow
        Write-Host "   the GREEN category or write a true RED test for" -ForegroundColor Yellow
        Write-Host "   unimplemented behavior." -ForegroundColor Yellow
        exit 1
    } else {
        Write-Host ""
        Write-Host "✓  RED tests failed as expected." -ForegroundColor Green
        Write-Host "   Ready to implement GREEN." -ForegroundColor Green
        Write-Host ""
        Write-Host "Failing tests (expected):" -ForegroundColor Gray
        # Show which tests failed
        $LASTEXITCODE = 0
        exit 0
    }
}

if ($Green) {
    Write-Banner "GREEN (Passing Validation)"
    Write-Host "Running GREEN-marked tests for validation..." -ForegroundColor Green
    $exitCode = Invoke-CargoTest -TestArgs @("--", "tdd_green") -ModeLabel "GREEN" -TddProfile
    if ($exitCode -ne 0) {
        Write-Host ""
        Write-Host "✗  Some GREEN tests failed." -ForegroundColor Red
        Write-Host "   Fix implementation before refactoring." -ForegroundColor Red
    } else {
        Write-Host ""
        Write-Host "✓  All GREEN tests pass." -ForegroundColor Green
    }
    exit $exitCode
}

if ($Refactor) {
    Write-Banner "REFACTOR (Full Test Suite)"
    Write-Host "Running ALL twitter tests to verify green after refactoring..." -ForegroundColor Magenta
    $exitCode = Invoke-CargoTest -ModeLabel "REFACTOR" -TddProfile
    if ($exitCode -ne 0) {
        Write-Host ""
        Write-Host "✗  REFACTOR: Some tests failed after refactoring." -ForegroundColor Red
        Write-Host "   Rollback or fix before proceeding." -ForegroundColor Red
    } else {
        Write-Host ""
        Write-Host "✓  REFACTOR: All tests green. Code is clean!" -ForegroundColor Green
    }
    exit $exitCode
}

if ($Watch) {
    Write-Banner "WATCH"
    Write-Host "Watching for changes in src/task/twitteractivity.rs and src/utils/twitter/..." -ForegroundColor Cyan
    Write-Host "Tests will re-run automatically on file changes." -ForegroundColor Cyan
    Write-Host "Press Ctrl+C to stop." -ForegroundColor Gray
    Write-Host ""

    # Check if cargo-watch is installed
    $watchAvailable = & cargo install --list 2>$null | Select-String -Pattern "cargo-watch" -SimpleMatch
    if (-not $watchAvailable) {
        Write-Host "cargo-watch not found. Installing..." -ForegroundColor Yellow
        & cargo install cargo-watch
    }

    $watchArgs = @(
        "watch",
        "-w", "src/task/twitteractivity.rs",
        "-w", "src/utils/twitter",
        "-x", "test --lib --test twitteractivity_integration"
    )

    if ($Filter) {
        $watchArgs += "--", "-x", "test --lib --test twitteractivity_integration -- $Filter"
    }

    & cargo @watchArgs
    exit $LASTEXITCODE
}

if ($Coverage) {
    Write-Banner "COVERAGE"
    Write-Host "Running tests with LLVM coverage instrumentation..." -ForegroundColor Cyan
    $exitCode = Invoke-CargoTest -ModeLabel "COVERAGE"
    if ($exitCode -eq 0) {
        Write-Host ""
        Write-Host "✓  All tests passed with coverage instrumentation." -ForegroundColor Green
        Write-Host ""
        Write-Host "To generate coverage report:" -ForegroundColor Gray
        Write-Host "  grcov . --binary-path ./target/debug/ -s . -t html --branch --ignore-not-existing -o ./coverage/" -ForegroundColor Gray
    }
    exit $exitCode
}

if ($Profile) {
    Write-Banner "PROFILE"
    Write-Host "Running tests with timing instrumentation..." -ForegroundColor Cyan
    $exitCode = Invoke-CargoTest -ModeLabel "PROFILE"
    exit $exitCode
}

# ============================================================================
# Default Mode: Run All Tests
# ============================================================================

Write-Banner "DEFAULT"
Write-Host "Test categories available:" -ForegroundColor Cyan
Write-Host "  -Red       Run RED (expected failing) tests" -ForegroundColor Yellow
Write-Host "  -Green     Run GREEN (passing) tests only" -ForegroundColor Green
Write-Host "  -Refactor  Run ALL tests for refactoring verification" -ForegroundColor Magenta
Write-Host "  -Watch     Watch mode for continuous testing" -ForegroundColor Cyan
Write-Host "  -Coverage  Run with coverage instrumentation" -ForegroundColor Cyan
Write-Host "  -Profile   List slowest tests" -ForegroundColor Gray
Write-Host "  -Fast      Unit tests only (skip integration)" -ForegroundColor Gray
Write-Host "  -Filter    Add extra filter string" -ForegroundColor Gray
Write-Host ""

$exitCode = Invoke-CargoTest -ModeLabel "ALL"

if ($exitCode -ne 0) {
    Write-Host ""
    Write-Host "╔════════════════════════════════════════════════╗" -ForegroundColor Red
    Write-Host "║  ⚠  Some tests FAILED. TDD cycle:            ║" -ForegroundColor Red
    Write-Host "║  1. Fix failing tests                        ║" -ForegroundColor Red
    Write-Host "║  2. Run .\run-twitter-tests.ps1 -Green       ║" -ForegroundColor Red
    Write-Host "║  3. Run .\run-twitter-tests.ps1 -Refactor    ║" -ForegroundColor Red
    Write-Host "╚════════════════════════════════════════════════╝" -ForegroundColor Red
} else {
    Write-Host ""
    Write-Host "╔════════════════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║  ✓  All tests PASSING. TDD cycle:            ║" -ForegroundColor Green
    Write-Host "║  1. Write RED test → run -Red to verify fail  ║" -ForegroundColor Green
    Write-Host "║  2. Implement → run -Green to verify pass     ║" -ForegroundColor Green
    Write-Host "║  3. Refactor → run -Refactor to verify green  ║" -ForegroundColor Green
    Write-Host "╚════════════════════════════════════════════════╝" -ForegroundColor Green
}

exit $exitCode
