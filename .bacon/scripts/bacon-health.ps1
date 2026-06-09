# .bacon/scripts/bacon-health.ps1 - Bacon health checks
# Usage: .\bacon-health.ps1 -Action prerequisites|test|metrics
# Dot-sources bacon-logging.ps1 for Write-BaconLog paths

param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("prerequisites", "test", "metrics")]
    [string]$Action,

    [Parameter(Mandatory=$false)]
    [switch]$ScriptVerbose
)

# Load shared logging
. "$PSScriptRoot\bacon-logging.ps1"

# --- Functions ---

function Test-BaconPrerequisites {
    Write-BaconLog "INFO" "Checking Bacon system prerequisites..." -Component "Prerequisites"

    $issues = @()
    $warnings = @()
    $details = @{}

    # Required directories
    $requiredDirs = @($BaconDir, $SessionsDir, (Join-Path $BaconDir "scripts"))
    $details.Directories = @()
    foreach ($dir in $requiredDirs) {
        $dirExists = Test-Path $dir
        $entry = @{ Path = $dir; Exists = $dirExists; Writable = $false }
        if ($dirExists) {
            try {
                $testFile = Join-Path $dir "test_write_$(Get-Random)"
                "test" | Out-File -FilePath $testFile -Encoding UTF8 -ErrorAction Stop
                Remove-Item $testFile -Force -ErrorAction SilentlyContinue
                $entry.Writable = $true
            } catch { $entry.Writable = $false }
        }
        $details.Directories += $entry
        if (-not $dirExists) { $issues += "Required directory missing: $dir" }
    }

    # Required files
    $requiredFiles = @((Join-Path $BaconDir "bacon.toml"))
    $details.Files = @()
    foreach ($file in $requiredFiles) {
        $fileExists = Test-Path $file
        $details.Files += @{ Path = $file; Exists = $fileExists }
        if (-not $fileExists) { $issues += "Required file missing: $file" }
    }

    # Required commands
    $requiredCommands = @("git", "cargo")
    $details.Commands = @()
    foreach ($cmd in $requiredCommands) {
        try {
            $cmdInfo = Get-Command $cmd -ErrorAction Stop
            $details.Commands += @{
                Name = $cmd; Found = $true; Path = $cmdInfo.Source
                Version = try { & $cmd --version 2>&1 | Select-Object -First 1 } catch { "Unknown" }
            }
        } catch {
            $details.Commands += @{ Name = $cmd; Found = $false; Path = $null; Version = $null }
            $issues += "Required command not found: $cmd"
        }
    }

    # Optional commands
    foreach ($cmd in @("bacon", "jq")) {
        try { $null = Get-Command $cmd -ErrorAction Stop } catch { $warnings += "Optional command not found: $cmd" }
    }

    # System resources
    $details.System = @{
        Memory = (Get-CimInstance -ClassName Win32_OperatingSystem | Select-Object TotalVisibleMemorySize, FreePhysicalMemory | ForEach-Object {
            "$([math]::Round($_.FreePhysicalMemory / 1MB, 2))GB free of $([math]::Round($_.TotalVisibleMemorySize / 1MB, 2))GB total"
        })
    }

    if ($ScriptVerbose) {
        Write-BaconLog "DEBUG" "Prerequisites check completed" -Component "Prerequisites" -Context $details
    }
    if ($issues.Count -gt 0) {
        Write-BaconLog "ERROR" "Prerequisites check failed:" -Component "Prerequisites"
        foreach ($issue in $issues) { Write-BaconLog "ERROR" "  - $issue" -Component "Prerequisites" }
    }
    if ($warnings.Count -gt 0) {
        Write-BaconLog "WARN" "Optional components missing:" -Component "Prerequisites"
        foreach ($warning in $warnings) { Write-BaconLog "WARN" "  - $warning" -Component "Prerequisites" }
    }

    return $issues.Count -eq 0
}

function Test-BaconSystem {
    Write-BaconLog "INFO" "Testing Bacon system..." -Component "SystemTest"

    $testsPassed = 0
    $testsTotal = 0

    # Test prerequisites
    $testsTotal++
    if (Test-BaconPrerequisites) { $testsPassed++; Write-Host "[PASS] Prerequisites test passed" -ForegroundColor Green }
    else { Write-Host "[FAIL] Prerequisites test failed" -ForegroundColor Red }

    # Test configuration parsing
    $testsTotal++
    try {
        Get-Content (Join-Path $BaconDir "bacon.toml") -Raw -ErrorAction Stop | Out-Null
        Write-Host "[PASS] Configuration file readable" -ForegroundColor Green; $testsPassed++
    } catch { Write-Host "[FAIL] Configuration file error: $($_.Exception.Message)" -ForegroundColor Red }

    # Test script availability
    $testsTotal++
    $requiredScripts = @("bacon-manager.ps1", "bacon-service.ps1", "bacon-health.ps1", "bacon-logging.ps1")
    $scriptsFound = ($requiredScripts | Where-Object { Test-Path (Join-Path $Script:BaconScriptDir $_) }).Count
    if ($scriptsFound -eq $requiredScripts.Count) { $testsPassed++; Write-Host "[PASS] All required scripts found" -ForegroundColor Green }
    else { Write-Host "[FAIL] Missing scripts: $scriptsFound/$($requiredScripts.Count) found" -ForegroundColor Red }

    # Test directory permissions
    $testsTotal++
    try {
        $testFile = Join-Path $SessionsDir "test_write_$(Get-Date -Format 'yyyyMMddHHmmss')"
        "test" | Out-File -FilePath $testFile -Encoding UTF8 -ErrorAction Stop
        Remove-Item $testFile -Force -ErrorAction Stop
        Write-Host "[PASS] Directory permissions OK" -ForegroundColor Green; $testsPassed++
    } catch { Write-Host "[FAIL] Directory permission error: $($_.Exception.Message)" -ForegroundColor Red }

    Write-Host "`nTest Results: $testsPassed/$testsTotal tests passed" -ForegroundColor $(if ($testsPassed -eq $testsTotal) { "Green" } else { "Yellow" })
    return $testsPassed -eq $testsTotal
}

function Show-BaconMetrics {
    $metricsFile = Join-Path $SessionsDir "metrics.json"

    if (-not (Test-Path $metricsFile)) {
        Write-Host "No metrics file found" -ForegroundColor Yellow
        return
    }

    try {
        $metrics = Get-Content $metricsFile -Raw | ConvertFrom-Json -ErrorAction Stop

        Write-Host "`n=== Bacon Metrics ===" -ForegroundColor Green

        $eventCounts = $metrics | Group-Object event | ForEach-Object {
            @{ Event = $_.Name; Count = $_.Count; Success = ($_.Group | Where-Object { $_.status -eq "success" }).Count; Failed = ($_.Group | Where-Object { $_.status -eq "failed" }).Count }
        }

        foreach ($count in $eventCounts) {
            Write-Host "$($count.Event): $($count.Count) total ($($count.Success) success, $($count.Failed) failed)"
        }

        Write-Host "`nRecent Activity:" -ForegroundColor Cyan
        $metrics | Sort-Object timestamp -Descending | Select-Object -First 10 | ForEach-Object {
            $statusColor = if ($_.status -eq "success") { "Green" } elseif ($_.status -eq "failed") { "Red" } else { "Yellow" }
            Write-Host "[$($_.timestamp)] $($_.event) - $($_.status)" -ForegroundColor $statusColor
        }
    } catch {
        Write-BaconLog "ERROR" "Failed to parse metrics: $($_.Exception.Message)" -Component "Metrics"
    }
}

# --- Main ---

function Main {
    switch ($Action) {
        "prerequisites" {
            $result = Test-BaconPrerequisites
            if ($result) { Write-Host "Prerequisites check: PASSED" -ForegroundColor Green }
            else { Write-Host "Prerequisites check: FAILED" -ForegroundColor Red }
        }
        "test" { Test-BaconSystem }
        "metrics" { Show-BaconMetrics }
    }
}

Main
