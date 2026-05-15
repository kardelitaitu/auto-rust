# .bacon/scripts/bacon-manager.ps1 - Bacon Management Dashboard
# Thin dispatcher that delegates to focused sub-scripts.
#
# Usage:
#   .\bacon-manager.ps1 -Action start|stop|status|logs|metrics|cleanup|test|apply-approved|rotate-keys
#
# Sub-scripts (all in .bacon/scripts/):
#   bacon-service.ps1   - start/stop/status/logs (daemon lifecycle)
#   bacon-health.ps1    - prerequisites/test/metrics (health checks)
#   bacon-keys.ps1      - rotate-keys (API key rotation)
#   bacon-cleanup.ps1   - cleanup (session/workspace/log cleanup)
#   bacon-apply.ps1     - apply-approved (patch application)

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("start", "stop", "status", "logs", "metrics", "cleanup", "test", "apply-approved", "rotate-keys")]
    [string]$Action = "status",

    [Parameter(Mandatory=$false)]
    [switch]$DryRun,

    [Parameter(Mandatory=$false)]
    [switch]$RunCheck,

    [Parameter(Mandatory=$false)]
    [switch]$Force,

    [Parameter(Mandatory=$false)]
    [switch]$ScriptVerbose
)

# Load shared logging
. "$PSScriptRoot\bacon-logging.ps1"

# Ensure sessions directory exists
if (-not (Test-Path $SessionsDir)) {
    New-Item -ItemType Directory -Path $SessionsDir -Force | Out-Null
}

Write-BaconLog "INFO" "Bacon Manager action: $Action"

# --- Dispatch to sub-scripts ---
switch ($Action) {
    "start"  { & "$PSScriptRoot\bacon-service.ps1" -Action start -ScriptVerbose:$ScriptVerbose }
    "stop"   { & "$PSScriptRoot\bacon-service.ps1" -Action stop -ScriptVerbose:$ScriptVerbose }
    "status" { & "$PSScriptRoot\bacon-service.ps1" -Action status -ScriptVerbose:$ScriptVerbose }
    "logs"   { & "$PSScriptRoot\bacon-service.ps1" -Action logs -ScriptVerbose:$ScriptVerbose }

    "metrics"     { & "$PSScriptRoot\bacon-health.ps1" -Action metrics -ScriptVerbose:$ScriptVerbose }
    "test"        { & "$PSScriptRoot\bacon-health.ps1" -Action test -ScriptVerbose:$ScriptVerbose }

    "cleanup"      { & "$PSScriptRoot\bacon-cleanup.ps1" -DryRun:$DryRun -ScriptVerbose:$ScriptVerbose }
    "apply-approved" { & "$PSScriptRoot\bacon-apply.ps1" -DryRun:$DryRun -ScriptVerbose:$ScriptVerbose }
    "rotate-keys"  { & "$PSScriptRoot\bacon-keys.ps1" -DryRun:$DryRun -Force:$Force -ScriptVerbose:$ScriptVerbose }

    default {
        Write-Host "Unknown action: $Action" -ForegroundColor Red
        Write-Host "Available actions: start, stop, status, logs, metrics, cleanup, test, apply-approved, rotate-keys" -ForegroundColor Yellow
    }
}
