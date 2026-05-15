# .bacon/scripts/bacon-apply.ps1 - Apply approved Bacon patches
# Usage: .\bacon-apply.ps1 [-DryRun]
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

function Invoke-BaconApplyApproved {
    Write-BaconLog "INFO" "Applying approved Bacon patches via Rust supervisor..." -Component "Apply"

    $args = @("run", "--bin", "bacon", "--", "--auto-apply")
    if ($DryRun) { $args += "--dry-run" }

    try {
        Start-Process -FilePath "cargo" -ArgumentList $args -WorkingDirectory $ProjectRoot -Wait -NoNewWindow
        Write-BaconLog "INFO" "Approved patches applied successfully" -Component "Apply"
        return $true
    } catch {
        Write-BaconLog "ERROR" "Failed to apply patches: $($_.Exception.Message)" -Component "Apply"
        return $false
    }
}

# --- Main ---

Invoke-BaconApplyApproved
