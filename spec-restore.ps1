# spec-restore.ps1 - Restore from a git stash checkpoint
# Usage: .\spec-restore.ps1 [stash-ref]

param (
    [Parameter(Mandatory=$false)]
    [string]$StashRef = "stash@{0}"
)

# Ensure we are in the repository root
if (-not (Test-Path ".git")) {
    Write-Error "Error: Must run from the repository root (directory containing .git)."
    exit 1
}

# Verify the stash ref exists
$stashList = git stash list
if ([string]::IsNullOrWhiteSpace($stashList)) {
    Write-Error "Error: No stashes found."
    exit 1
}

$found = $false
foreach ($line in $stashList -split "`r?`n") {
    if ($line -like "$StashRef*") {
        $found = $true
        Write-Host "Found checkpoint: $line"
        break
    }
}

if (-not $found) {
    Write-Error "Error: Stash reference '$StashRef' not found."
    Write-Host "Available stashes:"
    git stash list
    exit 1
}

# Perform a safe apply
Write-Host "Restoring checkpoint '$StashRef'..."
git stash apply $StashRef

if ($LASTEXITCODE -ne 0) {
    Write-Error "Error: Failed to apply stash. You may have merge conflicts."
    exit $LASTEXITCODE
}

Write-Host "`nCheckpoint restored successfully."
Write-Host "Note: The stash entry remains in your stash list. Use 'git stash drop $StashRef' if you want to remove it."
