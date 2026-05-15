# .bacon/scripts/test_configuration.ps1 - Bacon Configuration Verification
# Run after pipeline updates to verify the config, docs, and script structure are consistent.
# Last updated: 2026-05-15 (post-manager-decomposition)

Write-Host "=== Bacon Configuration Test ===" -ForegroundColor Cyan
$allPassed = $true

# ============================================================
# Test 1: bacon.toml structure
# ============================================================
Write-Host "`n[1/5] Testing bacon.toml structure..." -ForegroundColor Yellow
$configFile = ".bacon/bacon.toml"
if (Test-Path $configFile) {
    try {
        $c = Get-Content $configFile -Raw
        $required = @("[pipeline]", "[agents.nvidia]")
        $missing = @()
        foreach ($s in $required) {
            if ($c -notmatch [regex]::Escape($s)) { $missing += $s }
        }
        $hasJobs = $c -match '\[jobs\.'
        $hasStageDelay = $c -match 'stage_delay_ms\s*='

        if ($missing.Count -eq 0 -and $hasJobs) {
            $details = "$($required.Count) required sections OK, job definitions found"
            if ($hasStageDelay) {
                $details += ", stage_delay_ms configured"
            } else {
                $details += " (no stage_delay_ms — using default 0)"
            }
            Write-Host "  $details" -ForegroundColor Green
        } else {
            Write-Host "  Missing sections: $($missing -join ', ')" -ForegroundColor Red
            if (-not $hasJobs) { Write-Host "  No [jobs.*] sections found" -ForegroundColor Red }
            $allPassed = $false
        }
    } catch {
        Write-Host "  FAILED to parse: $_" -ForegroundColor Red
        $allPassed = $false
    }
} else {
    Write-Host "  NOT FOUND" -ForegroundColor Red
    $allPassed = $false
}

# ============================================================
# Test 2: workflow.md structure
# ============================================================
Write-Host "`n[2/5] Testing workflow.md structure..." -ForegroundColor Yellow
$wf = ".bacon/workflow.md"
if (Test-Path $wf) {
    $content = Get-Content $wf -Raw
    $requiredTopics = @("Pipeline Overview", "Pipeline Stages", "Configuration")
    $missing = @()
    foreach ($t in $requiredTopics) {
        if ($content -notmatch [regex]::Escape($t)) { $missing += $t }
    }
    # Check for NVIDIA and bacon binary usage references (bacon, not cargo run --bin bacon)
    $hasNvidia = $content -match "nvidia"
    $hasBaconBin = $content -match "bacon.*-p"

    if ($missing.Count -eq 0) {
        Write-Host "  $($requiredTopics.Count) required topics OK" -ForegroundColor Green
    } else {
        Write-Host "  Missing topics: $($missing -join ', ')" -ForegroundColor Red
        $allPassed = $false
    }
    if (-not $hasNvidia) {
        Write-Host "  No NVIDIA agent reference found" -ForegroundColor Red
        $allPassed = $false
    }
    if (-not $hasBaconBin) {
        Write-Host "  No bacon CLI usage reference found" -ForegroundColor Red
        $allPassed = $false
    }
} else {
    Write-Host "  NOT FOUND" -ForegroundColor Red
    $allPassed = $false
}

# ============================================================
# Test 3: All 7 decomposed scripts (vs. old monolithic structure)
# ============================================================
Write-Host "`n[3/5] Testing all 7 decomposed scripts..." -ForegroundColor Yellow
$scriptDir = ".bacon/scripts"
# The 7 decomposed scripts replacing the old monolithic bacon-manager.ps1 (879 lines):
#   bacon-logging.ps1  — shared logging module, dot-sourced by all
#   bacon-service.ps1  — daemon lifecycle: start/stop/status/logs
#   bacon-health.ps1   — health checks: prerequisites/system test/metrics
#   bacon-keys.ps1     — API key rotation
#   bacon-cleanup.ps1  — session/workspace/log cleanup
#   bacon-apply.ps1    — patch application
#   bacon-manager.ps1  — thin dispatcher
$required = @(
    "bacon-logging.ps1",
    "bacon-service.ps1",
    "bacon-health.ps1",
    "bacon-keys.ps1",
    "bacon-cleanup.ps1",
    "bacon-apply.ps1",
    "bacon-manager.ps1"
)
$found = @()
$missing = @()
foreach ($f in $required) {
    if (Test-Path (Join-Path $scriptDir $f)) { $found += $f }
    else { $missing += $f }
}

if ($missing.Count -eq 0) {
    Write-Host "  All $($found.Count) scripts present" -ForegroundColor Green
    # Verify bacon-manager.ps1 is the thin dispatcher, not the old monolith
    $lineCount = (Get-Content (Join-Path $scriptDir "bacon-manager.ps1")).Count
    if ($lineCount -le 120) {
        Write-Host "  bacon-manager.ps1: $lineCount lines (thin dispatch) ✓" -ForegroundColor Green
    } else {
        Write-Host "  bacon-manager.ps1: $lineCount lines — expected < 120 (may still be monolithic)" -ForegroundColor Red
        $allPassed = $false
    }
} else {
    Write-Host "  Missing: $($missing -join ', ')" -ForegroundColor Red
    $allPassed = $false
}

# ============================================================
# Test 4: NVIDIA agent config
# ============================================================
Write-Host "`n[4/5] Testing NVIDIA agent config..." -ForegroundColor Yellow
if (Test-Path $configFile) {
    $c = Get-Content $configFile -Raw
    $hasSection = $c -match '\[agents\.nvidia\]'
    $hasProvider = $c -match 'provider\s*=\s*"nvidia"'
    $requiredFields = @("model", "temperature", "max_tokens")
    $missingFields = @()
    foreach ($field in $requiredFields) {
        if ($c -notmatch "$field\s*=") { $missingFields += $field }
    }

    if ($hasSection -and $hasProvider -and $missingFields.Count -eq 0) {
        Write-Host "  NVIDIA agent config: section OK, provider OK, fields OK" -ForegroundColor Green
    } else {
        if (-not $hasSection) { Write-Host "  Missing [agents.nvidia] section" -ForegroundColor Red; $allPassed = $false }
        if (-not $hasProvider) { Write-Host "  Missing provider = 'nvidia'" -ForegroundColor Red; $allPassed = $false }
        if ($missingFields.Count -gt 0) { Write-Host "  Missing fields: $($missingFields -join ', ')" -ForegroundColor Red; $allPassed = $false }
    }
} else {
    Write-Host "  SKIP (no bacon.toml)" -ForegroundColor Yellow
}

# ============================================================
# Test 5: Role prompt files
# ============================================================
Write-Host "`n[5/5] Testing role prompt files..." -ForegroundColor Yellow
$rolesDir = ".bacon/roles"
$requiredRoles = @(
    "01_bacon-observer.md",
    "02_bacon-strategy.md",
    "03_bacon-coder.md",
    "04_bacon-auditor.md"
)
$foundRoles = @()
$missingRoles = @()
foreach ($r in $requiredRoles) {
    $path = Join-Path $rolesDir $r
    if (Test-Path $path) {
        $foundRoles += $r
        $size = (Get-Item $path).Length
        if ($size -lt 100) {
            Write-Host "  WARNING: $r is only $size bytes (may be empty)" -ForegroundColor Yellow
        }
    } else {
        $missingRoles += $r
    }
}

if ($missingRoles.Count -eq 0) {
    Write-Host "  All $($foundRoles.Count) role prompts present" -ForegroundColor Green
} else {
    Write-Host "  Missing roles: $($missingRoles -join ', ')" -ForegroundColor Red
    $allPassed = $false
}

# Summary
Write-Host "`n====================================" -ForegroundColor Cyan
if ($allPassed) {
    Write-Host "RESULT: ALL 5 TESTS PASSED" -ForegroundColor Green
} else {
    Write-Host "RESULT: SOME TESTS FAILED" -ForegroundColor Red
}
Write-Host "====================================" -ForegroundColor Cyan

if (-not $allPassed) { exit 1 }
