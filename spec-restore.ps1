#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [string]$Ref,
    [switch]$Pop
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path

function Throw-RestoreError([string]$message) {
    throw $message
}

function Ensure-RepoRoot {
    $repoRoot = (& git rev-parse --show-toplevel 2>$null).Trim()
    if (-not $repoRoot) {
        Throw-RestoreError "spec-restore: not inside a git repository"
    }
    Set-Location $repoRoot
    return $repoRoot
}

Set-Location $root
$repoRoot = Ensure-RepoRoot

if ([string]::IsNullOrWhiteSpace($Ref)) {
    $Ref = "stash@{0}"
}

$mode = if ($Pop) { "pop" } else { "apply" }
Write-Host "spec-restore: $mode $Ref"

& git stash $mode --index $Ref
if ($LASTEXITCODE -ne 0) {
    Throw-RestoreError "spec-restore: git stash $mode failed for $Ref"
}

if (-not $Pop) {
    Write-Host "spec-restore: checkpoint retained; use -Pop after verifying the restore"
}
