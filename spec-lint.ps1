#!/usr/bin/env pwsh
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path

function Fail([string]$message) {
    Write-Error $message
    exit 1
}

function Read-Text([string]$path) {
    if (-not (Test-Path $path)) { return $null }
    return Get-Content $path -Raw
}

function Parse-Field([string]$text, [string]$pattern) {
    if (-not $text) { return $null }
    $m = [regex]::Match($text, $pattern, [System.Text.RegularExpressions.RegexOptions]::Multiline)
    if ($m.Success) { return $m.Groups[1].Value.Trim() }
    return $null
}

$requiredFiles = @(
    "README.md",
    "spec.yaml",
    "baseline.md",
    "internal-api-outline.md",
    "plan.md",
    "validation-checklist.md",
    "ci-commands.md",
    "decisions.md",
    "quality-rules.md",
    "implementation-notes.md"
)

$packages = @()

$templatePath = Join-Path $root "docs\specs\_template"
if (-not (Test-Path $templatePath)) { Fail "Missing spec template: $templatePath" }
$packages += [PSCustomObject]@{ Path = $templatePath; Bucket = "_template" }

foreach ($bucket in @("_active", "_done")) {
    $bucketPath = Join-Path $root "docs\specs\$bucket"
    if (-not (Test-Path $bucketPath)) { Fail "Missing spec bucket: $bucketPath" }
    Get-ChildItem $bucketPath -Directory | ForEach-Object {
        $packages += [PSCustomObject]@{ Path = $_.FullName; Bucket = $bucket }
    }
}

foreach ($pkg in $packages) {
    foreach ($file in $requiredFiles) {
        $filePath = Join-Path $pkg.Path $file
        if (-not (Test-Path $filePath)) {
            Fail "Missing required file: $filePath"
        }
    }

    $readme = Read-Text (Join-Path $pkg.Path "README.md")
    $spec = Read-Text (Join-Path $pkg.Path "spec.yaml")
    if (-not $readme -or -not $spec) { Fail "Unreadable spec package: $($pkg.Path)" }

    $readmeStatus = Parse-Field $readme '^Status:\s*`([^`]+)`\s*$'
    $specStatus = Parse-Field $spec '^status:\s*([A-Za-z]+)\s*$'
    $specId = Parse-Field $spec '^id:\s*([^\s]+)\s*$'
    $specImplementer = Parse-Field $spec '^implementer:\s*([^\s]+)\s*$'

    if (-not $readmeStatus) { Fail "Missing README status in $($pkg.Path)" }
    if (-not $specStatus) { Fail "Missing spec.yaml status in $($pkg.Path)" }
    if ($readmeStatus -ne $specStatus) { Fail "Status mismatch in $($pkg.Path): README=$readmeStatus spec.yaml=$specStatus" }

    foreach ($file in $requiredFiles) {
        if ($file -eq "spec.yaml") { continue }
        if ($spec -notmatch [regex]::Escape($file)) {
            Fail "Missing required file reference in spec.yaml: $file ($($pkg.Path))"
        }
    }

    $folderName = Split-Path $pkg.Path -Leaf
    if ($pkg.Bucket -ne "_template" -and $specId -and $specId -ne $folderName) {
        Fail "Folder/id mismatch in $($pkg.Path): folder=$folderName spec.yaml id=$specId"
    }

    switch ($pkg.Bucket) {
        "_template" {
            if ($specStatus -ne "draft") {
                Fail "Template must stay draft: $($pkg.Path)"
            }
            if ($specImplementer -ne "pending") {
                Fail "Template implementer must stay pending: $($pkg.Path)"
            }
        }
        "_active" {
            if ($specStatus -notin @("approved", "implementing")) {
                Fail "Active spec must be approved or implementing: $($pkg.Path)"
            }
        }
        "_done" {
            if ($specStatus -ne "done") {
                Fail "Done spec must be done: $($pkg.Path)"
            }
            if ($specImplementer -eq "pending") {
                Fail "Done spec must not have pending implementer: $($pkg.Path)"
            }
            $notes = Read-Text (Join-Path $pkg.Path "implementation-notes.md")
            if (-not $notes -or $notes.Trim().Length -eq 0) {
                Fail "Done spec must have implementation notes: $($pkg.Path)"
            }
        }
    }
}

Write-Host "spec-lint: pass"
