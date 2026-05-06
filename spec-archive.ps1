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

# Validate package structure
$specYaml = "$packagePath\spec.yaml"
$readme = "$packagePath\README.md"

if (-not (Test-Path $specYaml)) {
    Write-Error "Missing spec.yaml in package: $PackagePath"
    exit 1
}

if (-not (Test-Path $readme)) {
    Write-Error "Missing README.md in package: $PackagePath"
    exit 1
}

# Read current spec status
$specContent = Get-Content $specYaml -Raw
$status = [regex]::Match($specContent, 'status:\s*(\w+)').Groups[1].Value.Trim()

if ($status -notin @("approved", "implementing")) {
    Write-Error "Package must be approved or implementing to archive: $PackagePath (current status: $status)"
    exit 1
}

# Determine target directory
$targetDir = "$root\docs\specs\_done"
$packageName = Split-Path $packagePath -Leaf

Write-Host "Archiving package: $packageName"
Write-Host "From: $packagePath"
Write-Host "To: $targetDir\$packageName"

# Read README.md content before moving files
$readmeContent = Get-Content $readme -Raw

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

# Update README.md
$updatedReadme = $readmeContent -replace "Status:.*", "Status: Done (Archived)"
Set-Content -Path $targetDir\$packageName\README.md -Value $updatedReadme -NoNewline

Write-Host "✅ Package archived successfully!"
Write-Host "Status updated to 'done'"
Write-Host "Implementer normalized to 'archived-*'"
Write-Host "Files moved to _done/$packageName"