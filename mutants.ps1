#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Layer 4: Mutation Testing — verifies tests catch real bugs.
.DESCRIPTION
    Runs cargo-mutants on core logic modules to measure test quality.
    Surviving mutants indicate gaps where tests don't distinguish correct from broken code.

    Targets (by priority):
      1. twitteractivity_limits.rs    — limit enforcement (baseline, fast)
      2. decision/engine.rs           — engagement decision engine
      3. decision/strategies/*.rs     — all decision strategies (llm, hybrid, persona, legacy, unified)
      4. twitteractivity_persona.rs   — persona weight selection + sentiment modulation
      5. twitteractivity_llm_validation.rs — emoji removal, LLM output sanitization
      6. twitteractivity_errors.rs    — error classification (transient vs permanent)
      7. session/duration.rs          — DurationMs newtype (has unsafe NonZeroU64)

    Threshold: aim for < 10% surviving mutants on core modules.
.EXAMPLE
    .\mutants.ps1                              # Run all targets (4 jobs x 8 build threads)
    .\mutants.ps1 -Target limits               # Only limits module
    .\mutants.ps1 -Target decision             # Only decision engine + strategies
    .\mutants.ps1 -Jobs 8 -BuildThreads 4      # More jobs, fewer threads per build
    .\mutants.ps1 -Jobs 2 -BuildThreads 16     # Fewer jobs, fastest per-job builds
    .\mutants.ps1 -NoNextest                   # Fall back to cargo test instead of nextest
    .\mutants.ps1 -SkipInstall                 # Skip cargo-mutants/nextest install check
    .\mutants.ps1 -AutoTune:$false             # Disable auto-tuning (use default 4 Jobs / 8 BuildThreads)
#>
[CmdletBinding()]
param(
    [ValidateSet("all", "limits", "decision", "persona", "llm", "errors", "duration")]
    [string]$Target = "all",
    [int]$Timeout = 180,
    [int]$Jobs = 4,
    [int]$BuildTimeout = 120,
    [int]$BuildThreads = 8,
    [switch]$SkipInstall,
    [switch]$JsonOutput,
    [switch]$NoNextest,
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

# Auto-tune only overrides default values, not explicitly passed parameters.
# $PSBoundParameters contains only the params the caller explicitly passed.
if ($AutoTune -and (-not $PSBoundParameters.ContainsKey('Jobs') -and -not $PSBoundParameters.ContainsKey('BuildThreads'))) {
    $Jobs = [math]::Min($cpuCores, 8)
    $BuildThreads = [math]::Min($cpuCores, 16)
    Write-Status "  Auto-tuned: Jobs=$Jobs, BuildThreads=$BuildThreads" "Green"
} elseif ($AutoTune -and ($PSBoundParameters.ContainsKey('Jobs') -or $PSBoundParameters.ContainsKey('BuildThreads'))) {
    Write-Status "  Auto-tune skipped: using explicit Jobs=$Jobs, BuildThreads=$BuildThreads" "Yellow"
}

# ---- Preflight ------------------------------------------------------
if (-not $SkipInstall) {
    if (-not (Get-Command cargo-mutants -EA SilentlyContinue)) {
        Write-Status "Installing cargo-mutants..." "Yellow"
        $env:RUSTC_WRAPPER = ""
        cargo install cargo-mutants
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Failed to install cargo-mutants"
            exit 1
        }
    }
    if (-not $NoNextest -and -not (Get-Command cargo-nextest -EA SilentlyContinue)) {
        Write-Status "Installing cargo-nextest (faster test runner)..." "Yellow"
        $env:RUSTC_WRAPPER = ""
        cargo install --locked cargo-nextest
        if ($LASTEXITCODE -ne 0) {
            Write-Status "  WARN: Failed to install nextest, falling back to cargo test" "Yellow"
            $NoNextest = $true
        }
    }
}

# ---- Target definitions ---------------------------------------------
# Each target: Name, File paths (relative), Description
$targets = @()

$targets += @{
    Name  = "limits"
    Files = @("src/utils/twitter/twitteractivity_limits.rs")
    Desc  = "Limit enforcement - counters, increments, summary"
}

$targets += @{
    Name  = "decision-engine"
    Files = @("src/utils/twitter/decision/engine.rs")
    Desc  = "Decision engine - handle_engagement_decision match arms"
}

$targets += @{
    Name  = "decision-strategies"
    Files = @(
        "src/utils/twitter/decision/strategies/llm.rs",
        "src/utils/twitter/decision/strategies/hybrid.rs",
        "src/utils/twitter/decision/strategies/persona.rs",
        "src/utils/twitter/decision/strategies/legacy.rs",
        "src/utils/twitter/decision/strategies/unified.rs"
    )
    Desc  = "Decision strategies - LLM, hybrid, persona, legacy, unified"
}

$targets += @{
    Name  = "persona"
    Files = @("src/utils/twitter/twitteractivity_persona.rs")
    Desc  = "Persona weights + sentiment modulation"
}

$targets += @{
    Name  = "llm"
    Files = @(
        "src/utils/twitter/twitteractivity_llm_validation.rs",
        "src/utils/twitter/twitteractivity_llm.rs"
    )
    Desc  = "LLM validation + response handling"
}

$targets += @{
    Name  = "errors"
    Files = @("src/utils/twitter/twitteractivity_errors.rs")
    Desc  = "Error classification - transient vs permanent"
}

$targets += @{
    Name  = "duration"
    Files = @("src/session/duration.rs")
    Desc  = "DurationMs newtype (unsafe NonZeroU64)"
}

# ---- Filter targets -------------------------------------------------
if ($Target -ne "all") {
    $targets = $targets | Where-Object { $_.Name -eq $Target -or $_.Name -like "$Target*" }
    if ($targets.Count -eq 0) {
        Write-Error "No targets matched '$Target'"
        exit 1
    }
}

# ---- Run mutation testing -------------------------------------------
$results = @()
$totalMutants = 0
$totalCaught = 0
$totalMissed = 0
$totalTimeout = 0
$anyFailed = $false

foreach ($t in $targets) {
    Write-Header "$($t.Name): $($t.Desc)"

    # Build file args
    $fileArgs = @()
    foreach ($f in $t.Files) {
        if (Test-Path $f) {
            $fileArgs += "--file"
            $fileArgs += $f
        } else {
            Write-Status "  SKIP: $f not found" "Yellow"
        }
    }

    if ($fileArgs.Count -eq 0) {
        Write-Status "  SKIP: no source files found" "Yellow"
        continue
    }

    # Run cargo-mutants
    $outputDir = "mutants-out/$($t.Name)"
    if (-not (Test-Path $outputDir)) {
        New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
    }
    $mutantsTimeout = $Timeout
    $mutantsArgs = @("mutants") + $fileArgs + @(
        "--timeout", $mutantsTimeout.ToString(),
        "--build-timeout", $BuildTimeout.ToString(),
        "--jobs", $Jobs.ToString(),
        "--cargo-arg=-j$BuildThreads",
        "--output", $outputDir
    )

    if (-not $NoNextest) {
        $mutantsArgs += "--test-tool"
        $mutantsArgs += "nextest"
    }

    if ($JsonOutput) {
        $mutantsArgs += "--json"
    }

    Write-Status "  Running: cargo $($mutantsArgs -join ' ')" "Yellow"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()

    $env:RUSTC_WRAPPER = ""
    & cargo @mutantsArgs
    $exitCode = $LASTEXITCODE
    $elapsed = $sw.Elapsed.TotalSeconds

    # Parse results from cargo-mutants output
    $summaryFile = Join-Path $outputDir "outcomes.json"
    $caught = 0; $missed = 0; $timedOut = 0; $total = 0

    if (Test-Path $summaryFile) {
        $outcomes = Get-Content $summaryFile -Raw | ConvertFrom-Json
        foreach ($o in $outcomes) {
            $total++
            switch ($o.outcome) {
                "Caught"   { $caught++ }
                "Missed"   { $missed++ }
                "Timeout"  { $timedOut++ }
            }
        }
    } else {
        Write-Status "  WARN: No outcomes.json found - check output manually" "Yellow"
    }

    $totalMutants += $total
    $totalCaught += $caught
    $totalMissed += $missed
    $totalTimeout += $timedOut

    $surviveRate = if ($total -gt 0) { [math]::Round(($missed / $total) * 100, 1) } else { 0 }
    $color = if ($surviveRate -lt 10) { "Green" } elseif ($surviveRate -lt 20) { "Yellow" } else { "Red" }

    Write-Status ("  Results: {0} mutants | {1} caught | {2} missed | {3} timeout | {4}% survived | {5:N0}s" -f `
        $total, $caught, $missed, $timedOut, $surviveRate, $elapsed) $color

    $results += @{
        Name     = $t.Name
        Desc     = $t.Desc
        Total    = $total
        Caught   = $caught
        Missed   = $missed
        Timeout  = $timedOut
        Rate     = $surviveRate
        Duration = $elapsed
        Passed   = ($surviveRate -lt 10)
    }

    if (-not ($surviveRate -lt 10)) { $anyFailed = $true }
}

# ---- Summary report -------------------------------------------------
$totalTime = ((Get-Date) - $startTime).TotalSeconds
Write-Header "MUTATION TESTING REPORT"

Write-Status ("{0,-25} {1,8} {2,8} {3,8} {4,8} {5,10} {6,8}" -f `
    "Module", "Total", "Caught", "Missed", "Timeout", "Survive%", "Time") "Cyan"
Write-Status ("{0,-25} {1,8} {2,8} {3,8} {4,8} {5,10} {6,8}" -f `
    ("-" * 25), ("-" * 8), ("-" * 8), ("-" * 8), ("-" * 8), ("-" * 10), ("-" * 8)) "Cyan"

foreach ($r in $results) {
    $col = if ($r.Passed) { "Green" } else { "Red" }
    Write-Status ("{0,-25} {1,8} {2,8} {3,8} {4,8} {5,9:N1}% {6,7:N0}s" -f `
        $r.Name, $r.Total, $r.Caught, $r.Missed, $r.Timeout, $r.Rate, $r.Duration) $col
}

$overallRate = if ($totalMutants -gt 0) { [math]::Round(($totalMissed / $totalMutants) * 100, 1) } else { 0 }
$overallColor = if ($overallRate -lt 10) { "Green" } elseif ($overallRate -lt 20) { "Yellow" } else { "Red" }

Write-Output ""
Write-Status ("Total: {0} mutants | {1} caught | {2} missed | {3}% survival rate | {4:N0}s" -f `
    $totalMutants, $totalCaught, $totalMissed, $overallRate, $totalTime) $overallColor

if ($overallRate -ge 10) {
    Write-Output ""
    Write-Status "ACTION NEEDED: Surviving mutants > 10% threshold." "Red"
    Write-Output "  Surviving mutants indicate test gaps. For each missed mutant:"
    Write-Output "    1. Check mutants-out/<module>/missed/*.txt for the mutation diff"
    Write-Output "    2. Add a test that would catch that specific mutation"
    Write-Output "    3. Re-run the module: .\mutants.ps1 -Target <module>"
    Write-Output ""
}

if ($anyFailed) {
    Write-Status "Some modules exceed the 10% threshold. See above." "Red"
    exit 1
} else {
    Write-Status "All modules under 10% survival rate. Tests are mutation-resistant." "Green"
    exit 0
}
