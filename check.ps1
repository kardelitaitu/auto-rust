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
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$startTime = Get-Date
# Timeout settings (in seconds)
$globalTimeout = 300
$buildTimeout = 60
$formatTimeout = 30
$clippyTimeout = 30
$testsTimeout = 180

$colors = @{
    Green  = "`e[32m"
    Red    = "`e[31m"
    Yellow = "`e[33m"
    Blue   = "`e[34m"
    Cyan   = "`e[36m"
    Reset  = "`e[0m"
}

function Write-Status($msg, $color = "White") {
    Write-Output "$($colors[$color] ?? '')$msg$($colors.Reset)"
}

function Write-Header($title) {
    Write-Output ""
    Write-Status "=== $title ===" "Cyan"
}

function Start-CheckProcess {
    param([string]$Cmd, [int]$Secs)

    $tmp = "$env:APPDATA\ci_$( [guid]::NewGuid().ToString('N') ).txt"
    $job = Start-Process pwsh -ArgumentList "-NoProfile", "-NonI", "-Command",
        "$Cmd | Out-File '$tmp'; exit `$LASTEXITCODE" `
        -NoNewWindow -PassThru

    $waited = 0
    while ($job.HasExited -eq $false -and $waited -lt $Secs) {
        Start-Sleep -Milliseconds 100
        $waited += 0.1
    }

    if ($job.HasExited -eq $false) {
        Stop-Process $job.Id -Force -EA SilentlyContinue
        if (Test-Path $tmp) { Remove-Item $tmp -EA SilentlyContinue }
        return @{ Output = $null; ExitCode = 124 }
    }

    $waited = 0
    while ((Test-Path $tmp) -eq $false -and $waited -lt 5) {
        Start-Sleep -Milliseconds 200
        $waited += 0.2
    }

    $output = if (Test-Path $tmp) { Get-Content $tmp -Raw } else { "" }
    $exitCode = $job.ExitCode
    if (Test-Path $tmp) { Remove-Item $tmp -EA SilentlyContinue }

    return @{ Output = $output; ExitCode = $exitCode }
}

function Test-Check {
    param(
        [string]$Name,
        [string]$Cmd,
        [int]$Secs,
        [scriptblock]$Success,
        [switch]$Quiet
    )

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $result = Start-CheckProcess -Cmd $Cmd -Secs $Secs
    $elapsed = $sw.Elapsed.TotalSeconds

    if ($result.ExitCode -eq 124) {
        Write-Status "$Name timed out after ${Secs}s" "Red"
        return @{ Passed = $false; Duration = $elapsed; ExitCode = 124 }
    }

    $passed = & $Success $result
    $color = if ($passed) { "Green" } else { "Red" }
    $dur = "{0:N2}s" -f $elapsed
    $msg = if ($passed) { "passed" } else { "failed" }

    if (-not $Quiet) {
        Write-Status "$Name $msg ($dur)" $color
    }

    return @{ Passed = $passed; Duration = $elapsed; ExitCode = $result.ExitCode; Output = $result.Output }
}

$results = @{
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

# ---- BUILD -----------------------------------------------------------
if (-not $SkipBuild) {
    $r = Test-Check -Name "Build" -Cmd "cargo check" -Secs $buildTimeout -Success {
        param($r) $r.ExitCode -eq 0
    }
    $results.Build = $r
    if (-not $r.Passed) { $failed = $true }
}

# ---- FORMAT -----------------------------------------------------------
if (-not $SkipFormat -and -not $failed) {
    $r = Test-Check -Name "Format" -Cmd "cargo fmt --all -- --check" -Secs $formatTimeout -Success {
        param($r) $r.ExitCode -eq 0
    }
    $results.Format = $r
    if (-not $r.Passed) { $failed = $true }
}

# ---- CLIPPY ----------------------------------------------------------
if (-not $SkipClippy -and -not $failed) {
    $r = Test-Check -Name "Clippy" -Cmd "cargo clippy --all-targets --all-features -- -D warnings" -Secs $clippyTimeout -Success {
        param($r) $r.ExitCode -eq 0
    }
    $results.Clippy = $r
    if (-not $r.Passed) { $failed = $true }
}

# ---- TESTS ----------------------------------------------------------
if (-not $SkipTests -and -not $failed) {
    # Silently install cargo-nextest if missing
    if (-not (Get-Command cargo-nextest -EA SilentlyContinue)) {
        cargo install --locked cargo-nextest 2>&1 | Out-Null | Out-Null
    }

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $testTmp = "$env:APPDATA\ci_test_$( [guid]::NewGuid().ToString('N') ).txt"

    # Run nextest with stdout/stderr redirected to file (completely silent)
    $testJob = Start-Process pwsh -ArgumentList "-NoProfile", "-NonI", "-Command",
        "cargo nextest run --all-features --lib 2>&1 | Out-File '$testTmp'; exit `$LASTEXITCODE" `
        -NoNewWindow -PassThru

    $waited = 0
    while ($testJob.HasExited -eq $false -and $waited -lt $testsTimeout) {
        Start-Sleep -Milliseconds 100
        $waited += 0.1
    }

    if ($testJob.HasExited -eq $false) {
        Stop-Process $testJob.Id -Force -EA SilentlyContinue
        $results.Tests = @{ Passed = $false; Duration = $sw.Elapsed.TotalSeconds }
        $failed = $true
    } else {
        Start-Sleep -Milliseconds 200
        $testOutput = if (Test-Path $testTmp) { Get-Content $testTmp -Raw } else { "" }
        if (Test-Path $testTmp) { Remove-Item $testTmp -EA SilentlyContinue }

        $passed = $testJob.ExitCode -eq 0
        $results.Tests = @{ Passed = $passed; Duration = $sw.Elapsed.TotalSeconds }
        if (-not $passed) { $failed = $true }

        # Show only last 5 lines of nextest output (the summary)
        if ($testOutput) {
            $lines = $testOutput -split "`n" | Where-Object { $_.Trim() -ne "" }
            $lastLines = $lines | Select-Object -Last 5
            if ($lastLines) {
                Write-Output ""
                $lastLines | ForEach-Object { Write-Output $_ }
            }
        }
    }
}

# ---- REPORT ----------------------------------------------------------
$total = ((Get-Date) - $startTime).TotalSeconds
Write-Output ""
Write-Status "CI CHECKER REPORT:" "Cyan"

$p = 0; $f = 0
$runOrder = @("Build", "Format", "Clippy", "Tests")
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

# ---- EXIT -----------------------------------------------------------
if ($f -eq 0) {
    Write-Status "All checks passed! Ready for commit." "Green"
    Write-Status "COMMIT REMINDER:" "Yellow"
    Write-Output "  - Describe the WHY, not the what"
    Write-Output "  - Format: 'type: description (reason/impact)'"
    Write-Output "  - Examples:"
    Write-Output "      'feat: add twitterquote task with LLM integration'"
    Write-Output "      'fix: handle rate limit in twitterfollow retry logic'"
    Write-Output "      'docs: rewrite README with TOC (843 -> 350 lines)'"
    Write-Output "  - DO NOT use: 'update', 'fix', 'changes'"
    exit 0
} else {
    Write-Status "Some checks failed. Fix before committing." "Red"
    exit 1
}