#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Archive a completed spec package according to archive conventions.
.DESCRIPTION
    Moves a spec package from _active/ to _done/, updates status fields,
    and normalizes the implementer field to archived conventions.
.PARAMETER PackagePath
        The path to the spec package to archive (relative to docs/specs/_active/)
.EXAMPLE
    .\spec-archive.ps1 spec-package-archive-safety
    .\spec-archive.ps1 twitteractivity-contract-alignment
#>

$ErrorActionPreference = "Stop"

# Get the package path from arguments
if ($args.Count -eq 0) {
    Write-Error "Usage: .\spec-archive.ps1 <package-name>"
    exit 1
}
$PackagePath = $args[0]

# Resolve to absolute paths
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$packagePath = Resolve-Path "$root\docs\specs\_active\$PackagePath"

if (-not (Test-Path $packagePath)) {
    Write-Error "Package path not found: $PackagePath"
    exit 1
}

# ---- Pre-flight Checks ----

# Check for spec-lint.ps1 availability
$specLint = "$root\spec-lint.ps1"
if (-not (Test-Path $specLint)) {
    Write-Error "spec-lint.ps1 not found at $specLint"
    exit 1
}

# Run spec-lint as a gate (matches promote_to_done() in auditor.rs)
Write-Host "Running spec-lint on $PackagePath..."
$lintOutput = & pwsh -NoProfile -NonInteractive -File $specLint -Directory $packagePath 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Error "spec-lint failed for $PackagePath - aborting archive:`n$lintOutput"
    exit 1
}
Write-Host "spec-lint passed"

# ---- Package Validation ----

$specYaml = "$packagePath\spec.yaml"
$readme = "$packagePath\README.md"

if (-not (Test-Path $specYaml)) {
    Write-Error "Missing spec.yaml in package: $PackagePath"
    exit 1
}

# README.md is optional — spec packages use 3 files (spec.yaml, plan.md, validation.md)
$hasReadme = Test-Path $readme

# Read current spec status
$specContent = Get-Content $specYaml -Raw
$status = [regex]::Match($specContent, 'status:\s*(\w+)').Groups[1].Value.Trim()

if ($status -notin @("approved", "implemented")) {
    Write-Error "Package must be approved or implemented to archive: $PackagePath (current status: $status)"
    exit 1
}

# Determine target directory
$targetDir = "$root\docs\specs\_done"
$packageName = Split-Path $packagePath -Leaf

Write-Host "Archiving package: $packageName"
Write-Host "From: $packagePath"
Write-Host "To: $targetDir\$packageName"

# Read README.md content before moving files (if it exists)
$readmeContent = if ($hasReadme) { Get-Content $readme -Raw } else { "" }

# Create target directory if needed
New-Item -ItemType Directory -Force -Path $targetDir\$packageName | Out-Null

# Move package files
Move-Item -Path $packagePath\* -Destination $targetDir\$packageName

# Update spec.yaml status to done
$specContent = $specContent -replace 'status:\s*\w+', "status: done"
Set-Content -Path $targetDir\$packageName\spec.yaml -Value $specContent -NoNewline

# Normalize implementer field to archived convention
$implementer = [regex]::Match($specContent, 'implementer:\s*(\w+)').Groups[1].Value.Trim()
if ($implementer -eq "pending") {
    $archivedImplementer = "archived-$($implementer)"
    $specContent = $specContent -replace "implementer:\s*\w+", "implementer: $archivedImplementer"
    Set-Content -Path $targetDir\$packageName\spec.yaml -Value $specContent -NoNewline
}

# Rewrite docs paths from _active/ to _done/ in spec.yaml
$specContent = $specContent `
    -replace 'docs/specs/_active/', 'docs/specs/_done/' `
    -replace 'docs\\specs\\_active\\', 'docs\\specs\\_done\\'
Set-Content -Path $targetDir\$packageName\spec.yaml -Value $specContent -NoNewline

# Update README.md (only if it existed in the original package)
if ($hasReadme) {
    $updatedReadme = $readmeContent -replace "Status:.*", "Status: Done (Archived)"
    Set-Content -Path $targetDir\$packageName\README.md -Value $updatedReadme -NoNewline
}

Write-Host "✅ Package archived successfully!"
Write-Host "Status updated to 'done'"
Write-Host "Implementer normalized to 'archived-*'"
Write-Host "Docs paths rewritten to _done/"
Write-Host "Files moved to _done/$packageName"