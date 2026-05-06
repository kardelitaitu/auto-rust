#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [string]$Spec,
    [string]$Reason = "checkpoint"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path

function Throw-CheckpointError([string]$message) {
    throw $message
}

function Ensure-RepoRoot {
    $repoRoot = (& git rev-parse --show-toplevel 2>$null).Trim()
    if (-not $repoRoot) {
        Throw-CheckpointError "spec-stash: not inside a git repository"
    }
    Set-Location $repoRoot
    return $repoRoot
}

function Get-CleanStatus {
    $status = & git status --porcelain
    if ($LASTEXITCODE -ne 0) {
        Throw-CheckpointError "spec-stash: unable to read git status"
    }
    return $status
}

Set-Location $root
$repoRoot = Ensure-RepoRoot
$status = Get-CleanStatus

if ([string]::IsNullOrWhiteSpace($status)) {
    Write-Host "spec-stash: clean worktree, nothing to save"
    exit 0
}

$messageParts = @("spec-checkpoint")
if ($Spec) { $messageParts += $Spec }
if ($Reason) { $messageParts += $Reason }
$message = $messageParts -join ": "

& git stash push -u -m $message
if ($LASTEXITCODE -ne 0) {
    Throw-CheckpointError "spec-stash: git stash push failed"
}

$latest = & git stash list -n 1 --format='%gd | %gs'
if ($LASTEXITCODE -ne 0) {
    Throw-CheckpointError "spec-stash: unable to read the created stash ref"
}

if ($latest) {
    Write-Host "spec-stash: saved $latest"
} else {
    Write-Host "spec-stash: saved checkpoint"
}
