# .bacon/scripts/bacon-cleanup.ps1 - Bacon session/workspace cleanup
# Usage: .\bacon-cleanup.ps1 [-DryRun]
# Dot-sources bacon-logging.ps1 for Write-BaconLog paths

param(
    [Parameter(Mandatory=$false)]
    [switch]$DryRun,

    [Parameter(Mandatory=$false)]
    [switch]$ScriptVerbose
)

# Load shared logging
. "$PSScriptRoot\bacon-logging.ps1"

# --- Functions ---

function Invoke-BaconCleanup {
    Write-BaconLog "INFO" "Starting Bacon cleanup..." -Component "Cleanup"

    $cleanupCount = 0

    # Clean old shadow workspaces (norino shadow dirs)
    $tempBase = if ($env:TEMP) { $env:TEMP } else { "/tmp" }
    $tempDirs = Get-ChildItem -Path $tempBase -Filter "norino_shadow_*" -ErrorAction SilentlyContinue
    foreach ($dir in $tempDirs) {
        if ($dir.CreationTime -lt (Get-Date).AddDays(-1)) {
            if ($DryRun) {
                Write-Host "[DRY-RUN] Would remove old shadow workspace: $($dir.FullName)" -ForegroundColor Yellow
            } else {
                try {
                    Remove-Item $dir.FullName -Recurse -Force -ErrorAction Stop
                    $cleanupCount++
                    Write-BaconLog "INFO" "Removed old shadow workspace: $($dir.Name)" -Component "Cleanup"
                } catch {
                    Write-BaconLog "WARN" "Failed to remove $($dir.Name): $($_.Exception.Message)" -Component "Cleanup"
                }
            }
        }
    }

    # Clean old session files (>7 days)
    $sessionFiles = Get-ChildItem -Path $SessionsDir -Filter "*.json" -ErrorAction SilentlyContinue
    foreach ($file in $sessionFiles) {
        if ($file.CreationTime -lt (Get-Date).AddDays(-7)) {
            if ($DryRun) {
                Write-Host "[DRY-RUN] Would remove old session file: $($file.Name)" -ForegroundColor Yellow
            } else {
                try {
                    Remove-Item $file.FullName -Force -ErrorAction Stop
                    $cleanupCount++
                    Write-BaconLog "INFO" "Removed old session file: $($file.Name)" -Component "Cleanup"
                } catch {
                    Write-BaconLog "WARN" "Failed to remove $($file.Name): $($_.Exception.Message)" -Component "Cleanup"
                }
            }
        }
    }

    # Clean old or oversized logs
    $logFiles = Get-ChildItem -Path $SessionsDir -Filter "*.log" -ErrorAction SilentlyContinue
    foreach ($file in $logFiles) {
        if ($file.Length -gt 10MB -or $file.CreationTime -lt (Get-Date).AddDays(-3)) {
            if ($DryRun) {
                Write-Host "[DRY-RUN] Would remove old log file: $($file.Name) ($([math]::Round($file.Length / 1MB, 2))MB)" -ForegroundColor Yellow
            } else {
                try {
                    Remove-Item $file.FullName -Force -ErrorAction Stop
                    $cleanupCount++
                    Write-BaconLog "INFO" "Removed old log file: $($file.Name)" -Component "Cleanup"
                } catch {
                    Write-BaconLog "WARN" "Failed to remove $($file.Name): $($_.Exception.Message)" -Component "Cleanup"
                }
            }
        }
    }

    Write-BaconLog "INFO" "Cleanup completed. Removed $cleanupCount items." -Component "Cleanup"
    Write-Host "Cleanup completed: $cleanupCount items removed" -ForegroundColor Green
}

# --- Main ---

Invoke-BaconCleanup
