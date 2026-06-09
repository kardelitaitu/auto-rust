# .bacon/scripts/bacon-keys.ps1 - API key rotation
# Usage: .\bacon-keys.ps1 [-DryRun] [-Force]
# Dot-sources bacon-logging.ps1 for Write-BaconLog paths

param(
    [Parameter(Mandatory=$false)]
    [switch]$DryRun,

    [Parameter(Mandatory=$false)]
    [switch]$Force,

    [Parameter(Mandatory=$false)]
    [switch]$ScriptVerbose
)

# Load shared logging
. "$PSScriptRoot\bacon-logging.ps1"

# --- Functions ---

function Invoke-BaconRotateKeys {
    Write-BaconLog "INFO" "Rotating API keys for external LLM providers..." -Component "Security"

    $envFile = Join-Path $ProjectRoot ".env"
    $envBackup = Join-Path $ProjectRoot ".env.backup.$(Get-Date -Format 'yyyyMMddHHmmss')"

    # Backup current .env file
    if (Test-Path $envFile) {
        Copy-Item $envFile $envBackup -Force
        Write-BaconLog "INFO" "Backed up .env to $envBackup" -Component "Security"
    }

    $configFile = Join-Path $BaconDir "bacon.toml"
    if (-not (Test-Path $configFile)) {
        Write-BaconLog "ERROR" "Configuration file not found: $configFile" -Component "Security"
        return $false
    }

    try {
        $configContent = Get-Content $configFile -Raw
        $keysToRotate = @()

        if ($configContent -match '{env:([^}]+)}') {
            $matches | ForEach-Object {
                if ($_ -match '{env:([^}]+)}') {
                    $keyName = $matches[1]
                    if ($keyName -match '(?i)(api[_-]?key|token|secret|password)') {
                        $keysToRotate += $keyName
                    }
                }
            }
        }

        if ($keysToRotate.Count -eq 0) {
            Write-BaconLog "INFO" "No API keys found in configuration" -Component "Security"
            return $true
        }

        if ($DryRun) {
            Write-Host "[DRY-RUN] Would rotate $($keysToRotate.Count) keys: $($keysToRotate -join ', ')" -ForegroundColor Yellow
            return $true
        }

        Write-Host "`n=== API Key Rotation ===" -ForegroundColor Cyan
        Write-Host "Found $($keysToRotate.Count) API keys to rotate:" -ForegroundColor Yellow

        $newEnvContent = @()
        if (Test-Path $envFile) {
            $newEnvContent = Get-Content $envFile
        }

        foreach ($key in $keysToRotate | Select-Object -Unique) {
            Write-Host "`nKey: $key" -ForegroundColor White

            $currentValue = $null
            foreach ($line in $newEnvContent) {
                if ($line -match "^$key=(.+)$") {
                    $currentValue = $matches[1]
                    break
                }
            }
            if ($currentValue) {
                Write-Host "Current value: $($currentValue.Substring(0, [math]::Min(8, $currentValue.Length)))..." -ForegroundColor Gray
            } else {
                Write-Host "Not currently set" -ForegroundColor Yellow
            }

            if (-not $Force) {
                $choice = Read-Host "`nAction: (r)otate, (s)kip, or (d)elete [r/s/d]"
            } else {
                $choice = "s"
            }

            switch ($choice.ToLower()) {
                "r" {
                    $newValue = Read-Host "Enter new value for $key" -AsSecureString
                    $plainText = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
                        [Runtime.InteropServices.Marshal]::SecureStringToBSTR($newValue)
                    )
                    $updated = $false
                    for ($i = 0; $i -lt $newEnvContent.Count; $i++) {
                        if ($newEnvContent[$i] -match "^$key=") {
                            $newEnvContent[$i] = "$key=$plainText"
                            $updated = $true; break
                        }
                    }
                    if (-not $updated) { $newEnvContent += "$key=$plainText" }
                    Write-BaconLog "INFO" "Rotated key: $key" -Component "Security"
                    Write-Host "Key rotated successfully" -ForegroundColor Green
                }
                "s" { Write-Host "Skipping $key" -ForegroundColor Yellow }
                "d" {
                    $newEnvContent = $newEnvContent | Where-Object { $_ -notmatch "^$key=" }
                    Write-BaconLog "INFO" "Deleted key: $key" -Component "Security"
                    Write-Host "Key deleted" -ForegroundColor Red
                }
                default { Write-Host "Invalid choice, skipping" -ForegroundColor Red }
            }
        }

        $newEnvContent | Out-File $envFile -Encoding UTF8
        Write-BaconLog "INFO" "Updated .env file with rotated keys" -Component "Security"

        try {
            icacls $envFile /inheritance:r /grant:r "$($env:USERNAME):(R,W)" 2>&1 | Out-Null
            Write-BaconLog "INFO" "Set restrictive permissions on .env file" -Component "Security"
        } catch {
            Write-BaconLog "WARN" "Failed to set permissions on .env: $($_.Exception.Message)" -Component "Security"
        }

        Write-Host "`nKey rotation completed. Backup saved to $envBackup" -ForegroundColor Green
        return $true

    } catch {
        Write-BaconLog "ERROR" "Failed to rotate keys: $($_.Exception.Message)" -Component "Security"
        return $false
    }
}

# --- Main ---

Invoke-BaconRotateKeys
