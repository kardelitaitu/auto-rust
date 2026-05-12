#!/usr/bin/env pwsh

<#
.SYNOPSIS
    Coverage Improvement Manager - Analyzes coverage and manages improvement items.
.DESCRIPTION
    Manages coverage improvement items: generate from tarpaulin JSON, list, complete, stats.
    Uses batch writes for efficiency.
.EXAMPLE
    .\coverage_improvement_manager.ps1 Generate -coverage_file coverage.json
    .\coverage_improvement_manager.ps1 List
    .\coverage_improvement_manager.ps1 Complete "ID123"
#>

[CmdletBinding(DefaultParameterSetName='List')]
param(
    [Parameter(Position=0)]
    [ValidateSet('Generate', 'List', 'Complete', 'Stats', 'Add')]
    [string]$Command = 'List',

    [Parameter(Mandatory=$true, ParameterSetName="Generate")]
    [string]$CoverageFile,

    [Parameter(ParameterSetName="Generate")]
    [int]$MinGap = 10,

    [Parameter(ParameterSetName="Generate")]
    [switch]$Quiet,

    [Parameter(ParameterSetName="List")]
    [string]$Status,

    [Parameter(ParameterSetName="List")]
    [string]$Priority,

    [Parameter(Mandatory=$true, ParameterSetName="Complete")]
    [string]$Id,

    [Parameter(ParameterSetName="Stats")]
    [switch]$Coverage
)

$TODO_FILE = "coverage_improvement.json"
$TODO_PATH = Join-Path $PSScriptRoot $TODO_FILE

function Read-TodoFile {
    if (Test-Path $TODO_PATH) {
        $content = Get-Content $TODO_PATH -Raw | ConvertFrom-Json
        if ($null -eq $content) { return @() }
        return $content
    }
    return @()
}

function Write-TodoFile {
    param([object]$Data)
    if ($null -eq $Data) { $Data = @() }
    $Data | ConvertTo-Json -Depth 10 | Set-Content -Path $TODO_PATH
}

function Get-DemangledName {
    param([string]$Name)
    $parts = @(); $i = 0
    while ($i -lt $Name.Length) {
        if ([char]::IsDigit($Name[$i])) {
            $j = $i
            while ($j -lt $Name.Length -and [char]::IsDigit($Name[$j])) { $j++ }
            $len = [int]$Name.Substring($i, $j - $i)
            $i = $j
            if ($len -gt 0 -and $i + $len -le $Name.Length) {
                $ident = $Name.Substring($i, $len)
                if ($ident -match '^[a-zA-Z_][a-zA-Z0-9_]*$') { $parts += $ident }
                $i += $len
            }
        } else { $i++ }
    }
    if ($parts.Count -le 1) { return $Name }
    return ($parts[1..($parts.Count-1)] -join '::')
}

function New-TodosFromCoverage {
    param(
        [string]$CoverageFile,
        [int]$MinGap,
        [switch]$Quiet
    )

    if (-not (Test-Path $CoverageFile)) {
        Write-Error "Coverage file not found: $CoverageFile"
        exit 1
    }

    $coverageData = Get-Content $CoverageFile -Raw | ConvertFrom-Json

    # Determine project root
    $projectRoot = $PSScriptRoot
    while ($projectRoot -and -not (Test-Path (Join-Path $projectRoot "Cargo.toml"))) {
        $projectRoot = Split-Path $projectRoot -Parent
    }

    $uncovered = @()

    # --- Tarpaulin JSON format ---
    if ($coverageData.files -and ($null -ne $coverageData.files[0].traces)) {
        foreach ($file in $coverageData.files) {
            if ($file.coverable -eq 0) { continue }

            $rawFile = ($file.path | Where-Object { $_ -ne '\' -and $_ -ne '/' }) -join [System.IO.Path]::DirectorySeparatorChar
            if ($projectRoot -and $rawFile.StartsWith($projectRoot, [StringComparison]::OrdinalIgnoreCase)) {
                $filename = $rawFile.Substring($projectRoot.Length).TrimStart([System.IO.Path]::DirectorySeparatorChar)
            } else {
                $filename = $rawFile
            }

            $uncoveredLines = $file.traces | Where-Object { $_.stats.Line -eq 0 } | ForEach-Object { [int]$_.line } | Sort-Object
            if ($uncoveredLines.Count -eq 0) { continue }

            $regions = @()
            $current = @()
            foreach ($line in $uncoveredLines) {
                if ($current.Count -eq 0 -or $line -eq $current[-1] + 1) {
                    $current += $line
                } else {
                    $regions += ,@($current[0], $current[-1])
                    $current = @($line)
                }
            }
            if ($current.Count -gt 0) {
                $regions += ,@($current[0], $current[-1])
            }

            foreach ($r in $regions) {
                $lineRange = $r[0]..$r[1]
                $label = if ($r[0] -eq $r[1]) { "Line $($r[0])" } else { "Lines $($r[0])-$($r[1])" }
                $uncovered += @{
                    file = $filename
                    function = $label
                    coverage = 0
                    lines = $lineRange
                    uncoveredLines = $lineRange
                }
            }
        }
    }

    # --- cargo llvm-cov export format (legacy fallback) ---
    elseif ($coverageData.data -and $coverageData.data[0].functions) {
        foreach ($func in $coverageData.data[0].functions) {
            if ($func.count -ne 0) { continue }

            $rawFile = $func.filenames[0]
            if ($projectRoot -and $rawFile.StartsWith($projectRoot)) {
                $filename = $rawFile.Substring($projectRoot.Length).TrimStart('\', '/')
            } else {
                $filename = $rawFile
            }

            $allLines = @(); $uncoveredLines = @()
            foreach ($region in $func.regions) {
                for ($l = $region[0]; $l -le $region[2]; $l++) {
                    $allLines += $l
                    if ($region[4] -eq 0) { $uncoveredLines += $l }
                }
            }
            $allLines = $allLines | Select-Object -Unique
            $uncoveredLines = $uncoveredLines | Select-Object -Unique
            if ($uncoveredLines.Count -eq 0) { continue }

            $uncovered += @{
                file = $filename
                function = (Get-DemangledName -Name $func.name)
                coverage = 0
                lines = $allLines
                uncoveredLines = $uncoveredLines
            }
        }
    }

    else {
        Write-Warning "Unknown coverage data structure."
        return
    }

    # --- Batch add all items ---
    $existingTodos = Read-TodoFile
    if ($null -eq $existingTodos) { $existingTodos = @() }
    $addedThisRun = @()

    $newCount = 0
    $skipCount = 0

    foreach ($region in $uncovered) {
        $duplicate = $existingTodos + $addedThisRun | Where-Object {
            $_.status -ne "Done" -and
            $_.coverage_gap -and
            $_.coverage_gap.file -eq $region.file -and
            $_.coverage_gap.function -eq $region.function
        }
        if ($duplicate) { $skipCount++; continue }

        $priority = if ($region.lines.Count -gt $MinGap) { "P1Critical" } else { "P2Important" }

        $existingTodos += @{
            id = [Guid]::NewGuid().ToString().Substring(0,8).ToUpper()
            title = "Test $($region.file):$($region.function)"
            description = "Add tests for uncovered code in $($region.file), $($region.function)"
            priority = $priority
            status = "Todo"
            category = "UnitTest"
            created = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
            completed = $null
            coverage_gap = $region
        }
        $addedThisRun += $region
        $newCount++
    }

    # Write once
    Write-TodoFile $existingTodos

    if (-not $Quiet) {
        Write-Host "Coverage analysis: $newCount new items, $skipCount already tracked"
    }
}

switch ($Command) {
    "Generate" {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        New-TodosFromCoverage -CoverageFile $CoverageFile -MinGap $MinGap -Quiet:$Quiet
        Write-Host "Generate done in $($sw.Elapsed.TotalSeconds.ToString('F1'))s"
    }

    "List" {
        $todos = Read-TodoFile
        if ($Status -or $Priority) {
            if ($Status) { $todos = $todos | Where-Object { $_.status -eq $Status } }
            if ($Priority) { $todos = $todos | Where-Object { $_.priority -eq $Priority } }
        }
        if ($todos.Count -eq 0) {
            Write-Host "No items."
        } else {
            $todos | Format-Table -Property id, @{n='file';e={$_.coverage_gap.file}}, priority, status -AutoSize -Wrap
        }
    }

    "Complete" {
        $todos = Read-TodoFile
        $item = $todos | Where-Object { $_.id -eq $Id }
        if ($item) {
            $item.completed = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
            $item.status = "Done"
            Write-TodoFile $todos
            Write-Host "Marked $Id as Done."
        } else {
            Write-Warning "Item $Id not found."
        }
    }

    "Stats" {
        $todos = Read-TodoFile
        $total = $todos.Count
        $pending = ($todos | Where-Object { $_.status -ne "Done" }).Count
        $completed = ($todos | Where-Object { $_.status -eq "Done" }).Count
        Write-Host "Total: $total | Pending: $pending | Completed: $completed"
    }

    "Add" {
        $todos = Read-TodoFile
        if ($null -eq $todos) { $todos = @() }
        $todos += @{
            id = [Guid]::NewGuid().ToString().Substring(0,8).ToUpper()
            title = "Manual coverage improvement item"
            description = "Manually added coverage improvement item"
            priority = "P3Lower"
            status = "Todo"
            category = "Other"
            created = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
            completed = $null
            coverage_gap = $null
        }
        Write-TodoFile $todos
        Write-Host "Added manual item."
    }

    default {
        Write-Error "Unknown command: $Command"
        exit 1
    }
}
