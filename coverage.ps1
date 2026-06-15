#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Layer 5: Coverage-Guided Gap Analysis — find untested branches.
.DESCRIPTION
    Runs cargo-tarpaulin to identify which branches, match arms, and error paths
    are never exercised by tests. Focuses on areas where coverage gaps hide bugs:

    Focus areas:
      1. handle_engagement_decision — which match arms are dead? (tweet_age branches, edge cases)
      2. js_* verification functions — are both querySelector success/fallback paths tested?
      3. Error propagation — anyhow::bail!, context(), unwrap_or_else — which paths never fire?

    Produces an HTML report and a line-level gap summary for each target module.
.EXAMPLE
    .\coverage.ps1                             # Full project coverage
    .\coverage.ps1 -Target decision             # Only decision engine
    .\coverage.ps1 -Target errors               # Only error paths
    .\coverage.ps1 -Target js                   # Only JS verification functions
    .\coverage.ps1 -Html                        # Open HTML report after run
    .\coverage.ps1 -SkipInstall                 # Skip tarpaulin install check
    .\coverage.ps1 -AutoTune:$false             # Disable auto-tuning (use default settings)
#>
[CmdletBinding()]
param(
    [ValidateSet("all", "decision", "errors", "js", "engagement", "llm")]
    [string]$Target = "all",
    [switch]$Html,
    [switch]$SkipInstall,
    [int]$Timeout = 300,
    [bool]$AutoTune = $true
)

$ErrorActionPreference = "Stop"
$startTime = Get-Date
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $root) { $root = (Get-Location).Path }
Set-Location -LiteralPath $root

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
    $testThreads = [math]::Min($cpuCores, 8)
    Write-Status "  Auto-tuned: test-threads=$testThreads" "Green"
}

# ---- Preflight ------------------------------------------------------
if (-not $SkipInstall) {
    if (-not (Get-Command cargo-tarpaulin -EA SilentlyContinue)) {
        Write-Status "Installing cargo-tarpaulin..." "Yellow"
        $env:RUSTC_WRAPPER = ""
        cargo install cargo-tarpaulin
        if ($LASTEXITCODE -ne 0) {
            Write-Error "Failed to install cargo-tarpaulin"
            exit 1
        }
    }
}

# ---- Target filter definitions --------------------------------------
$packages = @()
$ignorePatterns = @()

switch ($Target) {
    "all" {
        # Full project — no package filter
    }
    "decision" {
        # Focus on decision engine + strategies
        # tarpaulin --packages doesn't exist, use --skip-clean + cover specific tests
        $testFilter = "decision"
    }
    "errors" {
        $testFilter = "errors"
    }
    "js" {
        $testFilter = "js_"
    }
    "engagement" {
        $testFilter = "engagement"
    }
    "llm" {
        $testFilter = "llm"
    }
}

# ---- Output directory -----------------------------------------------
$outputDir = "coverage-data"
if (-not (Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

# ---- Step 1: Run tarpaulin with branch coverage --------------------
Write-Header "Running cargo-tarpaulin (branch coverage)"

$tarpaulinArgs = @(
    "tarpaulin",
    "--lib",
    "--all-features",
    "--timeout", $Timeout.ToString(),
    "--branch",
    "--out", "Html",
    "--out", "Json",
    "--output-dir", $outputDir,
    "--skip-clean"
)

if ($Target -ne "all" -and $testFilter) {
    # Run specific test subset for focused coverage
    $tarpaulinArgs += "--test-threads"
    $tarpaulinArgs += $testThreads.ToString()
}

Write-Status "  Running: cargo $($tarpaulinArgs -join ' ')" "Yellow"
$sw = [System.Diagnostics.Stopwatch]::StartNew()

$env:RUSTC_WRAPPER = ""
& cargo @tarpaulinArgs
$tarpExitCode = $LASTEXITCODE
$elapsed = $sw.Elapsed.TotalSeconds

Write-Status ("  Completed in {0:N0}s (exit code: {1})" -f $elapsed, $tarpExitCode)

# ---- Step 2: Parse coverage data -----------------------------------
Write-Header "Coverage Analysis"

$jsonFile = Join-Path $outputDir "tarpaulin-report.json"
$htmlFile = Join-Path $outputDir "tarpaulin-report.html"

if (Test-Path $jsonFile) {
    $report = Get-Content $jsonFile -Raw | ConvertFrom-Json

    # Overall stats
    $totalLines = 0
    $coveredLines = 0
    $totalBranches = 0
    $coveredBranches = 0

    foreach ($file in $report.files.PSObject.Properties) {
        $fileData = $file.Value
        $totalLines += $fileData.coverable_lines
        $coveredLines += $fileData.covered_lines
        if ($null -ne $fileData.branches) {
            $totalBranches += $fileData.branches.total
            $coveredBranches += $fileData.branches.covered
        }
    }

    $lineRate = if ($totalLines -gt 0) { [math]::Round(($coveredLines / $totalLines) * 100, 1) } else { 0 }
    $branchRate = if ($totalBranches -gt 0) { [math]::Round(($coveredBranches / $totalBranches) * 100, 1) } else { 0 }

    Write-Status ("  Line coverage:   {0}/{1} ({2}%)" -f $coveredLines, $totalLines, $lineRate) `
        $(if ($lineRate -ge 70) { "Green" } elseif ($lineRate -ge 50) { "Yellow" } else { "Red" })
    Write-Status ("  Branch coverage: {0}/{1} ({2}%)" -f $coveredBranches, $totalBranches, $branchRate) `
        $(if ($branchRate -ge 60) { "Green" } elseif ($branchRate -ge 40) { "Yellow" } else { "Red" })
} else {
    Write-Status "  WARN: No JSON report found at $jsonFile" "Yellow"
    Write-Status "  Check tarpaulin output above for errors." "Yellow"
}

# ---- Step 3: Identify gap modules ----------------------------------
Write-Header "Coverage Gap Report (target modules)"

# These are the specific files TODO.md asks us to check
$gapTargets = @(
    @{
        Name = "handle_engagement_decision"
        File = "src/utils/twitter/decision/engine.rs"
        Desc = "Match arms for engagement decisions — tweet_age branches, edge cases"
    },
    @{
        Name = "js_verification"
        File = "src/utils/twitter/twitteractivity_llm.rs"
        Desc = "querySelector success vs null/document fallback paths"
    },
    @{
        Name = "error_propagation"
        File = "src/utils/twitter/twitteractivity_errors.rs"
        Desc = "anyhow::bail!, context(), unwrap_or_else — which error paths never fire?"
    },
    @{
        Name = "engagement_state"
        File = "src/utils/twitter/twitteractivity_state.rs"
        Desc = "State transitions — are all branches of handle_state_change tested?"
    },
    @{
        Name = "persona_weights"
        File = "src/utils/twitter/twitteractivity_persona.rs"
        Desc = "select_persona_weights — override paths, boundary conditions"
    },
    @{
        Name = "limits_enforcement"
        File = "src/utils/twitter/twitteractivity_limits.rs"
        Desc = "Limit checks — which action types hit limits, overflow guards"
    }
)

# Check if each target file has coverage data
if (Test-Path $jsonFile) {
    $report = Get-Content $jsonFile -Raw | ConvertFrom-Json

    Write-Status ("{0,-30} {1,12} {2,12} {3,10}" -f "Module", "Lines", "Covered", "Rate") "Cyan"
    Write-Status ("{0,-30} {1,12} {2,12} {3,10}" -f ("-" * 30), ("-" * 12), ("-" * 12), ("-" * 10)) "Cyan"

    foreach ($gt in $gapTargets) {
        $normPath = $gt.File -replace '/', '\'
        $found = $false

        foreach ($file in $report.files.PSObject.Properties) {
            $filePath = $file.Name -replace '/', '\'
            if ($filePath -like "*$normPath*" -or $normPath -like "*$filePath*") {
                $fileData = $file.Value
                $total = $fileData.coverable_lines
                $covered = $fileData.covered_lines
                $rate = if ($total -gt 0) { [math]::Round(($covered / $total) * 100, 1) } else { 0 }
                $col = if ($rate -ge 70) { "Green" } elseif ($rate -ge 50) { "Yellow" } else { "Red" }

                Write-Status ("{0,-30} {1,12} {2,12} {3,9:N1}%" -f $gt.Name, $total, $covered, $rate) $col
                $found = $true
                break
            }
        }

        if (-not $found) {
            Write-Status ("{0,-30} {1,12} {2,12} {3,10}" -f $gt.Name, "-", "-", "N/A") "Yellow"
        }
    }
}

# ---- Step 4: Low-coverage file list --------------------------------
if (Test-Path $jsonFile) {
    Write-Header "Files Below 50% Line Coverage"

    $lowCoverage = @()
    foreach ($file in $report.files.PSObject.Properties) {
        $fileData = $file.Value
        $total = $fileData.coverable_lines
        $covered = $fileData.covered_lines
        if ($total -gt 0) {
            $rate = ($covered / $total) * 100
            if ($rate -lt 50 -and $total -ge 10) {
                $lowCoverage += @{
                    Path   = $file.Name
                    Total  = $total
                    Covered = $covered
                    Rate   = $rate
                }
            }
        }
    }

    if ($lowCoverage.Count -eq 0) {
        Write-Status "  No source files below 50% coverage (with >= 10 coverable lines)." "Green"
    } else {
        $lowCoverage = $lowCoverage | Sort-Object { $_.Rate }
        Write-Status ("{0,-60} {1,8} {2,8} {3,8}" -f "File", "Lines", "Covered", "Rate") "Cyan"
        foreach ($lc in $lowCoverage) {
            $shortPath = $lc.Path -replace '.*[/\\]src[/\\]', 'src/'
            Write-Status ("{0,-60} {1,8} {2,8} {3,7:N1}%" -f $shortPath, $lc.Total, $lc.Covered, $lc.Rate) "Red"
        }
    }
}

# ---- Step 5: Uncovered error paths ---------------------------------
Write-Header "Error Path Analysis (manual grep)"

$errorFiles = @(
    "src/utils/twitter/twitteractivity_errors.rs",
    "src/utils/twitter/twitteractivity_engagement.rs",
    "src/utils/twitter/twitteractivity_navigation.rs",
    "src/utils/twitter/twitteractivity_state.rs"
)

$bailCount = 0
$contextCount = 0

foreach ($ef in $errorFiles) {
    if (Test-Path $ef) {
        $content = Get-Content $ef -Raw
        $bails = ([regex]::Matches($content, 'bail!\(')).Count
        $contexts = ([regex]::Matches($content, '\.context\(')).Count
        $unwrapOrs = ([regex]::Matches($content, 'unwrap_or_else')).Count
        $bailCount += $bails
        $contextCount += $contexts
        $shortPath = $ef -replace '.*[/\\]src[/\\]', 'src/'
        Write-Status ("  {0}: {1} bail!, {2} .context(), {3} unwrap_or_else" -f $shortPath, $bails, $contexts, $unwrapOrs)
    }
}

Write-Output ""
Write-Status ("  Total across target files: {0} bail!, {1} .context()" -f $bailCount, $contextCount)
Write-Status "  Check HTML report for exact uncovered lines." "Yellow"

# ---- Report ---------------------------------------------------------
$totalTime = ((Get-Date) - $startTime).TotalSeconds
Write-Header "COVERAGE REPORT SUMMARY"

Write-Status ("  Total time: {0:N0}s" -f $totalTime)
Write-Status ("  HTML report: $outputDir/tarpaulin-report.html")

if ($Html -and (Test-Path $htmlFile)) {
    Write-Status "  Opening HTML report..." "Cyan"
    Start-Process $htmlFile
}

if ($tarpExitCode -ne 0) {
    Write-Status "  tarpaulin exited with code $tarpExitCode — check output above" "Yellow"
}

Write-Output ""
Write-Status "NEXT STEPS:" "Yellow"
Write-Output "  1. Open the HTML report to see exact uncovered lines"
Write-Output "  2. Focus on red files (below 50% coverage)"
Write-Output "  3. For decision engine: ensure all match arms have test cases"
Write-Output "  4. For error paths: verify bail!/context() paths with negative test cases"
Write-Output "  5. For JS verification: test both querySelector success and null fallback"

exit $tarpExitCode
