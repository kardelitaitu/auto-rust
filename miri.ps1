#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Layer 6: Dynamic Analysis — detect undefined behavior with Miri.
.DESCRIPTION
    Runs cargo-miri on the test suite to detect undefined behavior, including:
      - Out-of-bounds pointer arithmetic
      - Use-after-free
      - Invalid boolean/enum values
      - Unaligned pointer access

    Primary targets:
      1. session/duration.rs — uses unsafe NonZeroU64::new_unchecked (2 call sites)
      2. Full test suite — catch any UB from dependencies or transitive unsafe

    Miri requires the nightly toolchain. This script installs it if needed.

    Optimized for high-core-count machines (32+ threads, 64GB+ RAM).
.EXAMPLE
    .\miri.ps1                             # Run all targets (duration + full)
    .\miri.ps1 -Target duration            # Only session/duration.rs tests (fast)
    .\miri.ps1 -Target full                # Full suite under miri (slow, thorough)
    .\miri.ps1 -MiriCpus 8                # Simulate 8 CPUs in miri (default: 4)
    .\miri.ps1 -TestThreads 8             # Run 8 tests in parallel (default: 4)
    .\miri.ps1 -FastMode                  # Skip slow checks (stacked borrows, leaks)
    .\miri.ps1 -SkipInstall               # Skip nightly + miri install check
    .\miri.ps1 -AutoTune:$false           # Disable auto-tuning (use default 4/4)
#>
[CmdletBinding()]
param(
    [ValidateSet("both", "full", "duration")]
    [string]$Target = "both",
    [int]$MiriCpus = 4,
    [int]$TestThreads = 4,
    [switch]$FastMode,
    [switch]$SkipInstall,
    [bool]$AutoTune = $true
)

$ErrorActionPreference = "Stop"
$startTime = Get-Date
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $root) { $root = (Get-Location).Path }
Set-Location $root

if (-not (Test-Path "Cargo.toml")) {
    Write-Error "ERROR: Must run from project root (where Cargo.toml is)"
    exit 1
}

# ---- Helpers --------------------------------------------------------
$colors = @{
    Green  = "`e[32m"
    Red    = "`e[31m"
    Yellow = "`e[33m"
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

# ---- System resource detection ----------------------------------------
Write-Header "System Resources"
$cpuCores = [Environment]::ProcessorCount
$ramBytes = (Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory
$ramGB = [math]::Round($ramBytes / 1GB, 1)
Write-Status "  Logical CPUs: $cpuCores" "Cyan"
Write-Status "  Total RAM:    $ramGB GB" "Cyan"

if ($AutoTune) {
    $MiriCpus = [math]::Min($cpuCores, 8)
    $TestThreads = [math]::Min([math]::Max(1, [int]($ramGB / 8)), 8)
    Write-Status "  Auto-tuned: MiriCpus=$MiriCpus, TestThreads=$TestThreads" "Green"
}

# ---- Preflight: install nightly + miri ------------------------------
if (-not $SkipInstall) {
    $nightlyInstalled = rustup toolchain list | Select-String "nightly"
    if (-not $nightlyInstalled) {
        Write-Status "Installing nightly toolchain..." "Yellow"
        rustup toolchain install nightly
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Failed to install nightly toolchain"
            exit 1
        }
    }

    $miriInstalled = rustup component list --toolchain nightly | Select-String "miri.*installed"
    if (-not $miriInstalled) {
        Write-Status "Installing miri component..." "Yellow"
        rustup component add miri --toolchain nightly
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Failed to install miri component"
            exit 1
        }
    }
}

# ---- Verify miri works ----------------------------------------------
Write-Header "Pre-flight check"
$miriVersion = cargo +nightly miri --version 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Error "cargo +nightly miri not available. Run: rustup component add miri --toolchain nightly"
    exit 1
}
Write-Status "  Using: $miriVersion"

# ---- Miri setup (build deps with MIR instrumentation) ----------------
Write-Status "Running miri setup for dependency instrumentation..." "Yellow"
cargo +nightly miri setup 2>&1 | ForEach-Object { $_ }
if ($LASTEXITCODE -ne 0) {
    Write-Error "miri setup failed"
    exit 1
}

# ---- Build MIRI flags for speed ------------------------------------
$miriFlags = @()
$miriFlags += "-Zmiri-num-cpus=$MiriCpus"
$miriFlags += "-Zmiri-disable-isolation"

if ($FastMode) {
    Write-Status "  Fast mode: skipping stacked borrows + leak checks" "Yellow"
    $miriFlags += "-Zmiri-disable-stacked-borrows"
    $miriFlags += "-Zmiri-ignore-leaks"
}

$env:MIRIFLAGS = $miriFlags -join " "
Write-Status "  MIRIFLAGS: $env:MIRIFLAGS"

# ---- Define targets ------------------------------------------------
$results = @()

function Invoke-MiriTest {
    param(
        [string]$Name,
        [string]$Desc,
        [string[]]$MiriArgs,
        [string[]]$SkipTests = @()
    )

    Write-Header "$Name`: $Desc"
    Write-Status "  Running: cargo $($MiriArgs -join ' ')" "Yellow"

    $sw = [System.Diagnostics.Stopwatch]::StartNew()

    & cargo @MiriArgs 2>&1 | ForEach-Object { $_ }
    $exitCode = $LASTEXITCODE
    $elapsed = $sw.Elapsed.TotalSeconds

    $passed = $exitCode -eq 0
    $col = if ($passed) { "Green" } else { "Red" }
    Write-Status ("  {0} in {1:N0}s" -f $(if ($passed) { "PASS" } else { "FAIL (exit $exitCode)" }), $elapsed) $col

    return @{
        Name     = $Name
        Desc     = $Desc
        Passed   = $passed
        ExitCode = $exitCode
        Duration = $elapsed
    }
}

# ---- Run targets ---------------------------------------------------
Write-Header "Running targets"
Write-Status "  Target: $Target" "Cyan"
# Note: RUSTC_WRAPPER is intentionally NOT set because Miri warns about it

if ($Target -eq "duration" -or $Target -eq "both") {
    Write-Status "  Running duration target..." "Yellow"
    # Fast focused run on the only real unsafe code
    $args = @("+nightly", "miri", "test", "--lib", "-j", $TestThreads.ToString(), "--", "session::duration")
    $results += Invoke-MiriTest -Name "duration" -Desc "session::duration tests (unsafe NonZeroU64)" -MiriArgs $args
}

if ($Target -eq "full" -or $Target -eq "both") {
    Write-Status "  Running full target..." "Yellow"
    # Full suite — slow but thorough
    # Skip tests that are known to be slow or use FFI/async that miri can't handle
    $skipPatterns = @(
        "api_client",           # HTTP/wiremock tests
        "twitteractivity_",     # Browser automation (FFI)
        "integration",          # Integration tests with external deps
        "retry",                # Retry tests with real timeouts
        "session::io",          # Session I/O with browser
        "concurrent"            # Concurrency tests (miri is single-threaded)
    )

    $skipArgs = @("--")
    foreach ($pattern in $skipPatterns) {
        $skipArgs += "--skip"
        $skipArgs += $pattern
    }

    Write-Status "  Skipping FFI/async tests: $($skipPatterns -join ', ')" "Yellow"
    $args = @("+nightly", "miri", "test", "--lib", "-j", $TestThreads.ToString()) + $skipArgs
    $results += Invoke-MiriTest -Name "full" -Desc "Full test suite (excluding FFI/async)" -MiriArgs $args
}

# ---- Report ---------------------------------------------------------
Write-Header "MIRI REPORT"

if ($results.Count -eq 0) {
    Write-Status "  No tests were run." "Yellow"
    exit 1
}

Write-Status ("{0,-20} {1,8} {2,12}" -f "Target", "Status", "Time") "Cyan"
Write-Status ("{0,-20} {1,8} {2,12}" -f ("-" * 20), ("-" * 8), ("-" * 12)) "Cyan"

$allPassed = $true
foreach ($r in $results) {
    # Skip empty results
    if (-not $r -or -not $r.Name) { continue }
    
    $status = if ($r.Passed) { "PASS" } else { "FAIL" }
    $col = if ($r.Passed) { "Green" } else { "Red" }
    Write-Status ("{0,-20} {1,8} {2,11:N0}s" -f $r.Name, $status, $r.Duration) $col
    if (-not $r.Passed) { $allPassed = $false }
}

if ($allPassed -and $results.Count -gt 0) {
    Write-Status "No undefined behavior detected." "Green"
    exit 0
} else {
    Write-Status "MIRI DETECTED ISSUES — see output above." "Red"
    exit 1
}

