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
       2. Full test suite (include-list) — only pure-computation modules that don't
          use tokio, temp dirs, or FFI. On Windows, Miri can't intercept syscalls
          like CreateIoCompletionPort or GetTempPathW, so async/filesystem modules
          are excluded. See the $includePatterns list for the current set.

    Miri requires the nightly toolchain. This script installs it if needed.

    Optimized for high-core-count machines (32+ threads, 64GB+ RAM).
.EXAMPLE
    .\scripts\miri.ps1                    # Run all targets (duration + full)
    .\scripts\miri.ps1 -Target duration   # Only session/duration.rs tests (fast)
    .\scripts\miri.ps1 -Target full       # Full suite under miri (slow, thorough)
    .\scripts\miri.ps1 -MiriCpus 8       # Simulate 8 CPUs in miri (default: 4)
    .\scripts\miri.ps1 -TestThreads 8    # Run 8 tests in parallel (default: 4)
    .\scripts\miri.ps1 -FastMode         # Skip slow checks (stacked borrows, leaks)
    .\scripts\miri.ps1 -SkipInstall      # Skip nightly + miri install check
    .\scripts\miri.ps1 -AutoTune:$false  # Disable auto-tuning (use default 4/4)
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
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $scriptDir) { $scriptDir = (Get-Location).Path }
# Walk up from script directory to find project root (Cargo.toml)
$root = $scriptDir
while ($root -and -not (Test-Path (Join-Path $root "Cargo.toml"))) {
    $root = Split-Path -Parent $root
}
if (-not $root -or -not (Test-Path (Join-Path $root "Cargo.toml"))) {
    Write-Error "ERROR: Must run from project root (where Cargo.toml is)"
    exit 1
}
Set-Location $root

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

# ---- Clean old logs ------------------------------------------------
$logDir = $scriptDir
Remove-Item -Path (Join-Path $logDir "miri-*.log") -Force -ErrorAction SilentlyContinue

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
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $logFile = Join-Path $logDir "miri-$Name-$timestamp.log"

    # Tee output to both console and log file
    & cargo @MiriArgs 2>&1 | Tee-Object -FilePath $logFile
    $exitCode = $LASTEXITCODE
    $elapsed = $sw.Elapsed.TotalSeconds

    # Extract failure summary from log
    $failures = @()
    if ($exitCode -ne 0) {
        $failures = Select-String -Path $logFile -Pattern "^test .+ FAILED" | ForEach-Object { $_.Line }
    }

    $passed = $exitCode -eq 0
    $col = if ($passed) { "Green" } else { "Red" }
    Write-Status ("  {0} in {1:N0}s" -f $(if ($passed) { "PASS" } else { "FAIL (exit $exitCode)" }), $elapsed) $col
    if ($failures.Count -gt 0) {
        Write-Status "  Failed tests ($($failures.Count)):" "Red"
        foreach ($f in $failures) { Write-Status "    $f" "Red" }
    }
    Write-Status "  Log: $logFile" "Cyan"

    return @{
        Name     = $Name
        Desc     = $Desc
        Passed   = $passed
        ExitCode = $exitCode
        Duration = $elapsed
        LogFile  = $logFile
    }
}

# ---- Run targets ---------------------------------------------------
Write-Header "Running targets"
Write-Status "  Target: $Target" "Cyan"
# Note: RUSTC_WRAPPER is intentionally NOT set because Miri warns about it

if ($Target -eq "duration" -or $Target -eq "both") {
    Write-Status "  Running duration target..." "Yellow"
    # Fast focused run on the only real unsafe code
    $miriArgs = @("+nightly", "miri", "test", "--lib", "-j", $TestThreads.ToString(), "--", "session::duration")
    $results += Invoke-MiriTest -Name "duration" -Desc "session::duration tests (unsafe NonZeroU64)" -MiriArgs $miriArgs
}

if ($Target -eq "full" -or $Target -eq "both") {
    Write-Status "  Running full target..." "Yellow"
    # Only pure-computation modules — no async (tokio), no filesystem FFI, no IO.
    # On Windows, ANY tokio runtime or filesystem op triggers FFI that Miri
    # cannot intercept (CreateIoCompletionPort, GetTempPathW, etc.).
    #
    # Uses include-list (positional filters) for pure modules, plus --skip
    # for any impure module that collides with an include pattern via
    # substring matching. For example "logger::" matches health_logger::*,
    # so health_logger:: is explicitly skipped.
    $includePatterns = @(
        "capabilities::"
        "cli::"
        "error::"
        "logger::"          # also matches health_logger::* (skipped below)
        "result::"
        "state::"           # also matches adaptive::*state, orchestrator::state::*, session::state::* (all skipped)
        "validation::"
    )

    # Collision skips: modules that collide with an include pattern via
    # substring match AND use tokio/filesystem FFI.
    $collisionSkips = @(
        "adaptive::"        # collides with state::; filesystem FFI
        "health_logger::"   # collides with logger::; tokio
        "logger::tests::test_file_logger_"  # collides with logger::; tempfile (GetTempPathW)
        "orchestrator::"    # collides with state::; tokio
        "session::"         # collides with state::; tokio
        "state::overlay::"  # MSVC CRT triggers GetModuleHandleA on Windows
    )

    # Miri-incompatible skips: tests that fail under Miri's simulated clock
    # (global clock is shared across threads — sleep in one test advances
    # the clock for all, breaking Instant::elapsed() timing assertions).
    $miriSkips = @(
        "utils::twitter::state::tracking::tdd_tests::tdd_green_action_tracker_cooldown_expires"
    )

    $filterArgs = @("--") + $includePatterns
    foreach ($pattern in ($collisionSkips + $miriSkips)) {
        $filterArgs += "--skip"
        $filterArgs += $pattern
    }

    Write-Status "  Including: $($includePatterns -join ', ')" "Yellow"
    Write-Status "  Collision skips: $($collisionSkips -join ', ')" "Yellow"
    Write-Status "  Miri-incompatible skips: $($miriSkips -join ', ')" "Yellow"
    $miriArgs = @("+nightly", "miri", "test", "--lib", "-j", $TestThreads.ToString()) + $filterArgs
    $results += Invoke-MiriTest -Name "full" -Desc "Pure-computation modules (~502 tests)" -MiriArgs $miriArgs
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
    Write-Status "MIRI DETECTED ISSUES" "Red"
    foreach ($r in $results) {
        if (-not $r.Passed) {
            Write-Status "  Target '$($r.Name)' failed -- log: $($r.LogFile)" "Red"
        }
    }
    exit 1
}

