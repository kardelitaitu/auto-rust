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
    $c = if ($colors[$color]) { $colors[$color] } else { "" }
    Write-Output "$c$msg$($colors.Reset)"
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

# ---- BUILD -----------------------------------------------------------
if (-not $SkipBuild) {
    Write-StepHeader $stepNum "Build check (cargo check)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $proc = Start-Process cargo -ArgumentList "check" -NoNewWindow -PassThru -Wait
    $elapsed = $sw.Elapsed.TotalSeconds
    $passed = $proc.ExitCode -eq 0
    $results.Build = @{ Passed = $passed; Duration = $elapsed }
    Write-StepResult $passed
    if (-not $passed) { $failed = $true }
    $stepNum++
}

# ---- FORMAT -----------------------------------------------------------
if (-not $SkipFormat -and -not $failed) {
    Write-StepHeader $stepNum "Format check (cargo fmt --all -- --check)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $proc = Start-Process cargo -ArgumentList "fmt","--all","--","--check" -NoNewWindow -PassThru -Wait
    $elapsed = $sw.Elapsed.TotalSeconds
    $passed = $proc.ExitCode -eq 0
    $results.Format = @{ Passed = $passed; Duration = $elapsed }
    Write-StepResult $passed
    if (-not $passed) { $failed = $true }
    $stepNum++
}

# ---- CLIPPY ----------------------------------------------------------
if (-not $SkipClippy -and -not $failed) {
    Write-StepHeader $stepNum "Clippy check (cargo clippy --all-targets --all-features -- -D warnings)"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $proc = Start-Process cargo -ArgumentList "clippy","--all-targets","--all-features","--","-D","warnings" -NoNewWindow -PassThru -Wait
    $elapsed = $sw.Elapsed.TotalSeconds
    $passed = $proc.ExitCode -eq 0
    $results.Clippy = @{ Passed = $passed; Duration = $elapsed }
    Write-StepResult $passed
    if (-not $passed) { $failed = $true }
    $stepNum++
}

# ---- TESTS ----------------------------------------------------------
if (-not $SkipTests -and -not $failed) {
    Write-StepHeader $stepNum "Nextest check (cargo nextest run --all-features --lib)"

    # Silently install cargo-nextest if missing
    if (-not (Get-Command cargo-nextest -EA SilentlyContinue)) {
        cargo install --locked cargo-nextest 2>&1 | Out-Null | Out-Null
    }

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $testTmp = "$env:APPDATA\ci_test_$( [guid]::NewGuid().ToString('N') ).txt"

    # Run nextest silently (output to file only), then get exit code
    $proc = Start-Process pwsh -ArgumentList "-NoProfile", "-NonI", "-Command",
        "& { `$out = cargo nextest run --all-features --lib 2>&1; `$e = $LASTEXITCODE; `$out | Out-File '$testTmp'; exit `$e }" `
        -NoNewWindow -PassThru -Wait

    $elapsed = $sw.Elapsed.TotalSeconds
    $passed = $proc.ExitCode -eq 0
    $results.Tests = @{ Passed = $passed; Duration = $elapsed }
    if (-not $passed) { $failed = $true }

    # Show only last line (summary)
    if (Test-Path $testTmp) {
        $testOutput = Get-Content $testTmp -Raw
        Remove-Item $testTmp -EA SilentlyContinue
        if ($testOutput) {
            $lines = $testOutput -split "`r?`n" | Where-Object { $_.Trim() -ne "" }
            $summaryLine = $lines | Select-Object -Last 1
            if ($summaryLine) {
                Write-Output $summaryLine
            }
        }
    }
    Write-StepResult $results.Tests.Passed
}

# ---- REPORT ----------------------------------------------------------
$total = ((Get-Date) - $startTime).TotalSeconds
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