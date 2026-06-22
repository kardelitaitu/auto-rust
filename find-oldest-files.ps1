<#
.SYNOPSIS
    Find the 3 oldest .rs and 3 oldest .md files by LastWriteTime for audit targeting.
.DESCRIPTION
    Scans the project tree (excluding target/, .git/, .bacon/, crates/)
    and prints the 3 oldest .rs source files and 3 oldest .md docs files
    by LastWriteTime, so you know which files are most stale and likely need audit.
.EXAMPLE
    .\find-oldest-files.ps1
#>

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

if (-not (Test-Path "$projectRoot/Cargo.toml")) {
    Write-Output "ERROR: Must run from project root (where Cargo.toml is)"
    exit 1
}

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

Write-Status "=== Oldest .rs Files (by LastWriteTime) ===" "Cyan"

$rsFiles = Get-ChildItem -Path $projectRoot -Recurse -Filter "*.rs" |
    Where-Object {
        $_.FullName -notmatch '[/\\]target[/\\]' -and
        $_.FullName -notmatch '[/\\]\.git[/\\]' -and
        $_.FullName -notmatch '[/\\]crates[/\\]' -and
        $_.FullName -notmatch '[/\\]node_modules[/\\]' -and
        $_.FullName -notmatch '[/\\]\.bacon[/\\]snapshots[/\\]' -and
        $_.FullName -notmatch '[/\\]\.nodejs-reference[/\\]'
    } |
    Sort-Object LastWriteTime |
    Select-Object -First 3

if ($rsFiles) {
    foreach ($f in $rsFiles) {
        $rel = [System.IO.Path]::GetRelativePath($projectRoot, $f.FullName)
        Write-Status "  $rel" "Yellow"
        Write-Output "    LastWrite: $($f.LastWriteTime.ToString('yyyy-MM-dd HH:mm'))"
        Write-Output "    Size: $($f.Length) bytes"
    }
} else {
    Write-Status "  (none found)" "Red"
}

Write-Output ""

Write-Status "=== Oldest .md Files (by LastWriteTime) ===" "Cyan"

$mdFiles = Get-ChildItem -Path $projectRoot -Recurse -Filter "*.md" |
    Where-Object {
        $_.FullName -notmatch '[/\\]target[/\\]' -and
        $_.FullName -notmatch '[/\\]\.git[/\\]' -and
        $_.FullName -notmatch '[/\\]crates[/\\]' -and
        $_.FullName -notmatch '[/\\]node_modules[/\\]' -and
        $_.FullName -notmatch '[/\\]\.bacon[/\\]snapshots[/\\]' -and
        $_.FullName -notmatch '[/\\]\.nodejs-reference[/\\]'
    } |
    Sort-Object LastWriteTime |
    Select-Object -First 3

if ($mdFiles) {
    foreach ($f in $mdFiles) {
        $rel = [System.IO.Path]::GetRelativePath($projectRoot, $f.FullName)
        Write-Status "  $rel" "Yellow"
        Write-Output "    LastWrite: $($f.LastWriteTime.ToString('yyyy-MM-dd HH:mm'))"
        Write-Output "    Size: $($f.Length) bytes"
    }
} else {
    Write-Status "  (none found)" "Red"
}

Write-Output ""
Write-Status "Done. Use 'git log --oneline -- <path>' to see commit history for each file." "Green"
