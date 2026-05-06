# spec-stash.ps1 - Create a named git stash checkpoint
# Usage: .\spec-stash.ps1 <checkpoint-name>

param (
    [Parameter(Mandatory=$true)]
    [string]$Name
)

# Ensure we are in the repository root
if (-not (Test-Path ".git")) {
    Write-Error "Error: Must run from the repository root (directory containing .git)."
    exit 1
}

# Check if there are any changes to stash
$status = git status --porcelain
if ([string]::IsNullOrWhiteSpace($status)) {
    Write-Host "No changes to checkpoint."
    exit 0
}

# Create the stash
$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$stashMessage = "spec-checkpoint: $Name ($timestamp)"
Write-Host "Creating checkpoint: $stashMessage..."

$stashResult = git stash push --include-untracked --message "$stashMessage"
if ($LASTEXITCODE -ne 0) {
    Write-Error "Error: Failed to create stash."
    exit $LASTEXITCODE
}

# Get the stash ref (it's always stash@{0} after a push)
$stashRef = "stash@{0}"
Write-Host "Checkpoint created successfully: $stashRef"
Write-Host "Description: $stashMessage"
Write-Host "`nTo restore this checkpoint, use:"
Write-Host ".\spec-restore.ps1 $stashRef"
