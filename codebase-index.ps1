#!/usr/bin/env pwsh
<#
.SYNOPSISIS
    Generate a list of .md and .rs files for context-mode re-indexing.
.DESCRIPTION
    Finds all .md and .rs files in the repo (excluding target/ and node_modules/)
    and outputs the list to codebase-files.txt and stdout.
    The assistant should then call ctx_index for each file.
.EXAMPLE
    .\codebase-index.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $root

function Get-CodebaseFiles {
    $files = Get-ChildItem -Recurse -Include *.md,*.rs |
        Where-Object { $_.FullName -notmatch 'target[\\/]|node_modules[\\/]' } |
        Select-Object -ExpandProperty FullName
    return $files
}

$files = Get-CodebaseFiles
$output = Join-Path $root "codebase-files.txt"
$files | Out-File -FilePath $output -Encoding utf8

Write-Host "Found $($files.Count) files. List saved to $output"
Write-Output "---FILE LIST---"
$files | ForEach-Object { Write-Output $_ }
Write-Output "---END---"

# Run Node.js helper to index files via context-mode
$nodeScript = Join-Path $root "index-codebase.js"
if (Test-Path $nodeScript) {
    Write-Host "Running index-codebase.js..."
    & node $nodeScript $output
} else {
    Write-Warning "index-codebase.js not found; skipping automatic indexing."
    Write-Host "To index manually, run: node index-codebase.js $output"
}
