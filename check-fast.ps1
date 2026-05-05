#!/usr/bin/env pwsh
<# 
.SYNOPSIS
    Fast scoped checks for local iteration.
.DESCRIPTION
    Runs file-level rustfmt checks and target-scoped cargo checks/clippy based on changed paths.
    Use this while iterating. Use .\check.ps1 before push or remote handoff.
.EXAMPLE
    .\check-fast.ps1
    .\check-fast.ps1 -Paths src\main.rs src\lib.rs
#>
[CmdletBinding()]
param(
    [string[]]$Paths,
    [switch]$IncludeUntracked
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

function Fail([string]$message) {
    Write-Error $message
    exit 1
}

function Get-ChangedPaths {
    if ($Paths -and $Paths.Count -gt 0) {
        return $Paths
    }

    $lines = @()
    try {
        $lines += & git status --porcelain -uall 2>$null
    } catch {
        return @()
    }

    $out = New-Object System.Collections.Generic.List[string]
    foreach ($line in $lines) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        if ($line.Length -lt 4) { continue }
        $path = $line.Substring(3).Trim()
        if ($path -match '->') {
            $path = ($path -split '->')[-1].Trim()
        }
        if ($path) { $out.Add($path) }
    }
    return $out.ToArray()
}

function Has-Prefix([string[]]$Items, [string[]]$Prefixes) {
    foreach ($item in $Items) {
        $normalized = $item -replace '\\', '/'
        foreach ($prefix in $Prefixes) {
            if ($normalized.StartsWith($prefix)) { return $true }
        }
    }
    return $false
}

function Get-RustFiles([string[]]$Items) {
    return @($Items | Where-Object { $_ -match '\.rs$' -and $_ -notmatch 'target/' })
}

function Invoke-RustfmtCheck([string[]]$Files) {
    if (-not $Files -or $Files.Count -eq 0) { return $true }
    Write-Host "rustfmt --check on $($Files.Count) file(s)"
    & rustfmt --edition 2021 --check @Files
    return $LASTEXITCODE -eq 0
}

function Invoke-CargoCheck([string[]]$Args) {
    if (-not $Args -or $Args.Count -eq 0) { return $true }
    Write-Host "cargo check $($Args -join ' ')"
    & cargo check @Args
    return $LASTEXITCODE -eq 0
}

function Invoke-CargoClippy([string[]]$Args) {
    if (-not $Args -or $Args.Count -eq 0) { return $true }
    Write-Host "cargo clippy $($Args -join ' ')"
    & cargo clippy @Args -- -D warnings
    return $LASTEXITCODE -eq 0
}

$changed = Get-ChangedPaths
if (-not $changed -or $changed.Count -eq 0) {
    Write-Host "check-fast: no changed files"
    exit 0
}

$rustFiles = Get-RustFiles $changed
$needsCargo = Has-Prefix $changed @("src/", "tests/", "examples/", "benches/")
$needsBins = Has-Prefix $changed @("src/main.rs", "src/bin/", "src/bin")
$needsBenches = Has-Prefix $changed @("benches/", "src/benchmarks/")
$needsDocsSpec = Has-Prefix $changed @("docs/specs/", "spec-lint.ps1", "check.ps1")

$failed = $false

if ($needsDocsSpec) {
    Write-Host "spec-lint"
    & pwsh -NoProfile -NonInteractive -File .\spec-lint.ps1
    if ($LASTEXITCODE -ne 0) { $failed = $true }
}

if (-not $failed -and $rustFiles.Count -gt 0) {
    if (-not (Invoke-RustfmtCheck $rustFiles)) { $failed = $true }
}

if (-not $failed -and $needsCargo) {
    $checkArgs = @()
    $clippyArgs = @()

    if ($needsBins) {
        $checkArgs += "--bins"
        $clippyArgs += "--bins"
    }

    if ($needsBenches) {
        $checkArgs += "--benches"
        $clippyArgs += "--benches"
    }

    $checkArgs += "--lib"
    $clippyArgs += "--lib"

    if (Has-Prefix $changed @("tests/")) {
        $checkArgs += "--tests"
        $clippyArgs += "--tests"
    }

    if (-not (Invoke-CargoCheck $checkArgs)) { $failed = $true }
    if (-not $failed -and -not (Invoke-CargoClippy $clippyArgs)) { $failed = $true }
}

if ($failed) {
    Write-Host "check-fast: failed"
    exit 1
}

Write-Host "check-fast: pass"
