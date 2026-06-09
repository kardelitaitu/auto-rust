# .bacon/scripts/bacon-logging.ps1 - Shared Logging Module
# Dot-source this from other scripts: . "$PSScriptRoot\bacon-logging.ps1"

# Global path variables (use .bacon/ as anchor for script dir -> project root)
$Script:BaconScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Script:ProjectRoot = Split-Path -Parent (Split-Path -Parent $Script:BaconScriptDir)
$Script:BaconDir = Join-Path $Script:ProjectRoot ".bacon"
$Script:SessionsDir = Join-Path $Script:BaconDir "sessions"
$Script:LogFile = Join-Path $Script:SessionsDir "bacon_manager.log"

# Structured logging function
function Write-BaconLog {
    param(
        [Parameter(Mandatory=$true)]
        [ValidateSet("DEBUG", "INFO", "WARN", "ERROR", "CRITICAL")]
        [string]$Level,

        [Parameter(Mandatory=$true)]
        [string]$Message,

        [Parameter(Mandatory=$false)]
        [string]$Component = "Manager",

        [Parameter(Mandatory=$false)]
        [hashtable]$Context = @{}
    )

    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $contextStr = if ($Context.Count -gt 0) { " [$($Context.Keys -join ':')]" } else { "" }
    $logEntry = "[$timestamp] [$Level] [$Component] $Message$contextStr"

    $color = switch ($Level) {
        "DEBUG" { "Cyan" }
        "INFO" { "Green" }
        "WARN" { "Yellow" }
        "ERROR" { "Red" }
        "CRITICAL" { "Magenta" }
        default { "White" }
    }

    if ($ScriptVerbose -or $Level -in @("WARN", "ERROR", "CRITICAL")) {
        Write-Host $logEntry -ForegroundColor $color
    }

    # File logging with rotation
    try {
        $logDir = Split-Path -Parent $Script:LogFile
        if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir -Force | Out-Null }
        if ((Test-Path $Script:LogFile) -and (Get-Item $Script:LogFile).Length -gt 50MB) {
            $backupFile = $Script:LogFile.Replace(".log", "_$(Get-Date -Format 'yyyyMMddHHmmss').log")
            Move-Item $Script:LogFile $backupFile -Force
        }
        Add-Content -Path $Script:LogFile -Value $logEntry -ErrorAction SilentlyContinue
    } catch {
        Write-Host "Failed to write to log file: $($_.Exception.Message)" -ForegroundColor Red
    }
}
