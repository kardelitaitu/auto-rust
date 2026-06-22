#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Auto-rust CI Checker - Runs full test suite like GitHub workflow
.DESCRIPTION
    Runs cargo check, fmt, clippy, then nextest with detailed reporting.
    Short-circuits on first failure for fast feedback.
    Mirrors .github//workflows/ci.ym1 for local Windows development.
.EXAMPLE
    .\check.PS1           # Run all checks
    .\check.PS1 -SkipTests # Skip test execution
#>
[CmdletBinding()]
param(
    [switch]$SkipTests,
    [switch]$SkipClippy,
    [switch]$SkipFormat,
    [switch]$SkipBuild,
    [switch]$SkipSpecLint
)

$ErrorActionPreference = "Stop"
$startTime = Get-Date

$colors = @{
    Green  = "`e[32m"
    Red    = "`e[31m"
    Yellow = "`e[33m"
    Blue   = "`e[34m"
    Cyan   = "`e[36m"
    Reset  = "`e[0m"
}

function Write-Status($msg, $color = "White") {
    $c = if ($colors[$color]) { $colors[$color] } else { "" }
    Write-Output "$c$msg$($colors.Reset)"
}

function Write-Header($title) {
    Write-Output ""
    Write-Status "=== $title ===" "Cyan"
}

$results = @{
    SpecLint = @{ Passed = $false; Duration = 0 }
    Build  = @{ Passed = $false; Duration = 0 }
    Format = @{ Passed = $false; Duration = 0 }
    Clippy = @{ Passed = $false; Duration = 0 }
    Tests  = @{ Passed = $false; Duration = 0 }
}

if (-not (Test-Path "Cargo.toml")) {
    Write-Status "ERROR: Must run from project root (where Cargo.toml is)" "Red"
    exit 1
}

$failed = $false
$stepNum = 1

function Write-StepHeader($num, $desc) {
    Write-Output "$num. $desc"
}

function Write-StepResult($passed) {
    if ($passed) {
        Write-Status "PASS" "Green"
    } else {
        Write-Status "FAIL" "Red"
    }
}

# ---- SPEC LINT -------------------------------------------------------
if (-not $SkipSpecLint) {
    Write-StepHeader $stepNum "Spec lint (.\spec-lint.ps1)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & pwsh -NoProfile -NonInteractive -File .\spec-lint.ps1
    $elapsed = $sw.Elapsed.TotalSeconds
    $passed = $LASTEXITCODE -eq 0
    $results.SpecLint = @{ Passed = $passed; Duration = $elapsed }
    Write-StepResult $passed
    if (-not $passed) { $failed = $true }
    $stepNum++
}

# ---- BUILD -----------------------------------------------------------
if (-not $SkipBuild) {
    Write-StepHeader $stepNum "Build check (cargo check)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & cargo check
    $elapsed = $sw.Elapsed.TotalSeconds
    $passed = $LASTEXITCODE -eq 0
    $results.Build = @{ Passed = $passed; Duration = $elapsed }
    Write-StepResult $passed
    if (-not $passed) { $failed = $true }
    $stepNum++
}

# ---- FORMAT -----------------------------------------------------------
if (-not $SkipFormat -and -not $failed) {
    Write-StepHeader $stepNum "Format check (cargo fmt --all -- --check)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & cargo fmt --all -- --check
    $elapsed = $sw.Elapsed.TotalSeconds
    $passed = $LASTEXITCODE -eq 0
    $results.Format = @{ Passed = $passed; Duration = $elapsed }
    Write-StepResult $passed
    if (-not $passed) { $failed = $true }
    $stepNum++
}

# ---- CLIPPY ----------------------------------------------------------
if (-not $SkipClippy -and -not $failed) {
    Write-StepHeader $stepNum "Clippy check (cargo clippy --all-targets --all-features -- -D warnings)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & cargo clippy --all-targets --all-features -- -D warnings
    $elapsed = $sw.Elapsed.TotalSeconds
    $passed = $LASTEXITCODE -eq 0
    $results.Clippy = @{ Passed = $passed; Duration = $elapsed }
    Write-StepResult $passed
    if (-not $passed) { $failed = $true }
    $stepNum++
}

# ---- CLIPPY: unwrap_used --------------------------------------------
if (-not $SkipClippy -and -not $failed) {
    Write-StepHeader $stepNum "Clippy - ban .unwrap() in production code (cargo clippy --lib -- -D clippy::unwrap_used)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & cargo clippy --lib -- -D warnings -D clippy::unwrap_used
    $elapsed = $sw.Elapsed.TotalSeconds
    $passed = $LASTEXITCODE -eq 0
    $results.UnwrapClippy = @{ Passed = $passed; Duration = $elapsed }
    Write-StepResult $passed
    if (-not $passed) { $failed = $true }
    $stepNum++
}

# ---- CLIPPY: expect_used --------------------------------------------
if (-not $SkipClippy -and -not $failed) {
    Write-StepHeader $stepNum "Clippy - ban .expect() in production code (cargo clippy --lib -- -D clippy::expect_used)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & cargo clippy --lib -- -D warnings -D clippy::expect_used
    $elapsed = $sw.Elapsed.TotalSeconds
    $passed = $LASTEXITCODE -eq 0
    $results.ExpectClippy = @{ Passed = $passed; Duration = $elapsed }
    Write-StepResult $passed
    if (-not $passed) { $failed = $true }
    $stepNum++
}

# ---- CLIPPY: binary targets (unwrap + expect) ----------------------
if (-not $SkipClippy -and -not $failed) {
    Write-StepHeader $stepNum "Clippy - ban unwrap/expect in binary targets (cargo clippy --bins -- -D clippy::unwrap_used -D clippy::expect_used)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & cargo clippy --bins -- -D warnings -D clippy::unwrap_used -D clippy::expect_used
    $elapsed = $sw.Elapsed.TotalSeconds
    $passed = $LASTEXITCODE -eq 0
    $results.BinsClippy = @{ Passed = $passed; Duration = $elapsed }
    Write-StepResult $passed
    if (-not $passed) { $failed = $true }
    $stepNum++
}

# ---- TESTS ----------------------------------------------------------
if (-not $SkipTests -and -not $failed) {
    Write-StepHeader $stepNum "Nextest check (cargo nextest run --all-features --lib)"

    if (-not (Get-Command cargo-nextest -EA SilentlyContinue)) {
        cargo install --locked cargo-nextest 2>&1 | Out-Null
    }

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & cargo nextest run --all-features --lib 2>&1 | ForEach-Object { $_ }
    $exitCode = $LASTEXITCODE
    $elapsed = $sw.Elapsed.TotalSeconds

    $passed = ($exitCode -eq 0) -or ($exitCode -eq 3)
    $results.Tests = @{ Passed = $passed; Duration = $elapsed }
    if (-not $passed) { $failed = $true }
    Write-StepResult $results.Tests.Passed
}

# ---- REPORT ----------------------------------------------------------
$total = ((Get-Date) - $startTime).TotalSeconds
Write-Status "CI CHECKER REPORT:" "Yellow"
$p = 0; $f = 0
$runOrder = @("SpecLint", "Build", "Format", "Clippy", "UnwrapClippy", "ExpectClippy", "BinsClippy", "Tests")
foreach ($name in $runOrder) {
    $r = $results.$name
    if ($r.Duration -gt 0 -or $r.Passed) {
        $s = if ($r.Passed) { "PASS" } else { "FAIL" }
        $col = if ($r.Passed) { "Green" } else { "Red" }
        Write-Status ("{0,-8}  {1,-25}  {2,8}" -f $s, $name, "{0:N2}s" -f $r.Duration) $col
        if ($r.Passed) { $p++ } else { $f++ }
    }
}
Write-Status ("Passed: $p  |  Failed: $f  |  Total Time: {0:N2}s" -f $total) $(if ($f -eq 0) { "Green" } else { "Red" })
Write-Status "----------------------------------------------" "Cyan"

# ---- FAILURE HELP --------------------------------------------------
if ($f -gt 0) {
    Write-Status "FAILURE TUTORIALS:" "Yellow"
    Write-Output ""

    if (-not $results.SpecLint.Passed -and $results.SpecLint.Duration -gt 0) {
        Write-Status "Spec Lint Failed:" "Red"
        Write-Output "  Run '.\spec-lint.ps1' to see detailed validation errors."
        Write-Output "  Common fixes: Check spec.yaml format, ensure required files exist, fix status mismatches."
        Write-Output ""
    }

    if (-not $results.Build.Passed -and $results.Build.Duration -gt 0) {
        Write-Status "Build Failed:" "Red"
        Write-Output "  Run 'cargo check' to see compilation errors."
        Write-Output "  Common fixes: Fix syntax errors, resolve type mismatches, add missing dependencies."
        Write-Output ""
    }

    if (-not $results.Format.Passed -and $results.Format.Duration -gt 0) {
        Write-Status "Format Failed:" "Red"
        Write-Output "  Run 'cargo fmt --all' to auto-fix formatting issues."
        Write-Output "  This will automatically format your code according to Rust standards."
        Write-Output ""
    }

    if (-not $results.Clippy.Passed -and $results.Clippy.Duration -gt 0) {
        Write-Status "Clippy Failed:" "Red"
        Write-Output "  Run 'cargo clippy --all-targets --all-features' to see all warnings."
        Write-Output "  Common fixes: Address performance suggestions, fix unsafe code warnings, improve code quality."
        Write-Output "  Use 'cargo clippy --fix' to auto-fix some issues."
        Write-Output ""
    }

    if (-not $results.UnwrapClippy.Passed -and $results.UnwrapClippy.Duration -gt 0) {
        Write-Status "Unwrap Clippy Failed:" "Red"
        Write-Output "  .unwrap() calls are banned in production code."
        Write-Output "  Use '?' for error propagation or '.expect("...")' with a message."
        Write-Output "  Run 'cargo clippy --lib -- -D clippy::unwrap_used' to see violations."
        Write-Output ""
    }

    if (-not $results.ExpectClippy.Passed -and $results.ExpectClippy.Duration -gt 0) {
        Write-Status "Expect Clippy Failed:" "Red"
        Write-Output "  .expect() calls are banned in production code."
        Write-Output "  Use '?' for error propagation or add '#[allow(clippy::expect_used)]' for justified invariants."
        Write-Output "  Run 'cargo clippy --lib -- -D clippy::expect_used' to see violations."
        Write-Output ""
    }

    if (-not $results.BinsClippy.Passed -and $results.BinsClippy.Duration -gt 0) {
        Write-Status "Binary Clippy Failed:" "Red"
        Write-Output "  .unwrap()/.expect() calls are banned in binary targets."
        Write-Output "  Use '?' for error propagation or '.unwrap_or_default()' for safe fallbacks."
        Write-Output "  Run 'cargo clippy --bins -- -D clippy::unwrap_used -D clippy::expect_used' to see violations."
        Write-Output ""
    }

    if (-not $results.Tests.Passed -and $results.Tests.Duration -gt 0) {
        Write-Status "Tests Failed:" "Red"
        Write-Output "  Run 'cargo nextest run --all-features --lib' to see detailed test output."
        Write-Output "  Common fixes: Check test assertions, fix panics, ensure test setup is correct."
        Write-Output "  For integration tests: 'cargo nextest run --all-features'"
        Write-Output ""
    }

    Write-Status "Need help? Check docs/ or ask in the project channel." "Cyan"
    Write-Output ""
}

# ---- EXIT -----------------------------------------------------------
if ($f -eq 0) {
    Write-Status "All checks passed! Ready for commit." "Green"
    Write-Status "COMMIT REMINDER:" "Yellow"
    Write-Output "  - Say why the change matters, not just what changed"
    Write-Output "  - Use: 'type: short summary (reason/impact)'"
    Write-Output "  - Keep it specific and scoped to one concern"
    Write-Output "  - Good examples:"
    Write-Output "      'feat: add twitterquote task (reuse LLM reply flow)'"
    Write-Output "      'fix: handle rate limits in twitterfollow (retry stability)'"
    Write-Output "      'docs: trim README TOC (faster first read)'"
    Write-Output "  - Avoid generic commits like: 'update', 'fix', 'changes'"
    exit 0
} else {
    Write-Status "Some checks failed. Fix before committing." "Red"
    exit 1
}
