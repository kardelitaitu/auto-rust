# Test script to verify Bacon configuration
# Run this after updating bacon.toml or workflow documentation

Write-Host "=== Bacon Configuration Test ===" -ForegroundColor Cyan

# Test 1: Check bacon.toml syntax
Write-Host "`n[1/5] Testing bacon.toml syntax..." -ForegroundColor Yellow
$configFile = ".bacon/bacon.toml"
if (Test-Path $configFile) {
    try {
        $content = Get-Content $configFile -Raw
        # Simple TOML validation - check for required sections
        $requiredSections = @("[workflow]", "[global]", "[monitoring]", "[safety]", "[agents.observer]")
        $missingSections = @()
        
        foreach ($section in $requiredSections) {
            if ($content -notmatch [regex]::Escape($section)) {
                $missingSections += $section
            }
        }
        
        if ($missingSections.Count -eq 0) {
            Write-Host "✓ bacon.toml has all required sections" -ForegroundColor Green
        } else {
            Write-Host "✗ Missing sections: $($missingSections -join ', ')" -ForegroundColor Red
        }
    } catch {
        Write-Host "✗ Failed to read bacon.toml: $_" -ForegroundColor Red
    }
} else {
    Write-Host "✗ bacon.toml not found" -ForegroundColor Red
}

# Test 2: Check workflow documentation
Write-Host "`n[2/5] Testing workflow documentation..." -ForegroundColor Yellow
$workflowFile = ".bacon/.bacon-workflow.md"
if (Test-Path $workflowFile) {
    $content = Get-Content $workflowFile -Raw
    $requiredSections = @("## Getting Started", "## Error Recovery", "## Security Guidelines", "## Troubleshooting")
    $missingSections = @()
    
    foreach ($section in $requiredSections) {
        if ($content -notmatch [regex]::Escape($section)) {
            $missingSections += $section
        }
    }
    
    if ($missingSections.Count -eq 0) {
        Write-Host "✓ Workflow documentation has all required sections" -ForegroundColor Green
    } else {
        Write-Host "✗ Missing sections: $($missingSections -join ', ')" -ForegroundColor Yellow
    }
    
    # Check for practical examples
    if ($content -match "## Practical Examples") {
        Write-Host "✓ Practical examples section found" -ForegroundColor Green
    } else {
        Write-Host "✗ Missing practical examples section" -ForegroundColor Yellow
    }
} else {
    Write-Host "✗ Workflow documentation not found" -ForegroundColor Red
}

# Test 3: Check manager script
Write-Host "`n[3/5] Testing manager script..." -ForegroundColor Yellow
$managerScript = ".bacon/scripts/bacon-manager.ps1"
if (Test-Path $managerScript) {
    $content = Get-Content $managerScript -Raw
    
    # Check for new actions
    $newActions = @("rotate-keys", "report", "dashboard")
    $missingActions = @()
    
    foreach ($action in $newActions) {
        if ($content -notmatch "`"$action`"") {
            $missingActions += $action
        }
    }
    
    if ($missingActions.Count -eq 0) {
        Write-Host "✓ Manager script has new actions" -ForegroundColor Green
    } else {
        Write-Host "✗ Missing actions: $($missingActions -join ', ')" -ForegroundColor Yellow
    }
    
    # Check for helper functions
    $requiredFunctions = @("Invoke-BaconRotateKeys", "Invoke-BaconReport", "Start-BaconDashboard")
    $missingFunctions = @()
    
    foreach ($function in $requiredFunctions) {
        if ($content -notmatch "function $function") {
            $missingFunctions += $function
        }
    }
    
    if ($missingFunctions.Count -eq 0) {
        Write-Host "✓ Manager script has new functions" -ForegroundColor Green
    } else {
        Write-Host "✗ Missing functions: $($missingFunctions -join ', ')" -ForegroundColor Yellow
    }
} else {
    Write-Host "✗ Manager script not found" -ForegroundColor Red
}

# Test 4: Check example agent
Write-Host "`n[4/5] Testing example agent..." -ForegroundColor Yellow
$exampleAgent = ".bacon/scripts/custom_agent_example.py"
if (Test-Path $exampleAgent) {
    Write-Host "✓ Example agent script found" -ForegroundColor Green
    
    # Check Python syntax
    try {
        python -m py_compile $exampleAgent 2>&1 | Out-Null
        Write-Host "✓ Example agent has valid Python syntax" -ForegroundColor Green
    } catch {
        Write-Host "✗ Example agent has Python syntax errors" -ForegroundColor Red
    }
} else {
    Write-Host "✗ Example agent script not found" -ForegroundColor Yellow
}

# Test 5: Check directory structure
Write-Host "`n[5/5] Testing directory structure..." -ForegroundColor Yellow
$requiredDirs = @(".bacon", ".bacon/scripts", ".bacon/sessions", ".bacon/roles", ".bacon/test_runs")
$missingDirs = @()

foreach ($dir in $requiredDirs) {
    if (-not (Test-Path $dir)) {
        $missingDirs += $dir
    }
}

if ($missingDirs.Count -eq 0) {
    Write-Host "✓ All required directories exist" -ForegroundColor Green
} else {
    Write-Host "✗ Missing directories: $($missingDirs -join ', ')" -ForegroundColor Red
}

# Summary
Write-Host "`n=== Test Summary ===" -ForegroundColor Cyan
Write-Host "Configuration tests completed." -ForegroundColor White
Write-Host "Run '.\bacon-manager.ps1 test' for system health check." -ForegroundColor Yellow
Write-Host "Run '.\bacon-manager.ps1 status' for current status." -ForegroundColor Yellow
Write-Host "Run '.\bacon-manager.ps1 rotate-keys' to manage API keys." -ForegroundColor Yellow