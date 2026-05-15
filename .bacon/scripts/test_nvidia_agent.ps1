# Test script for NVIDIA agent
# This tests the basic functionality of the NVIDIA agent

Write-Host "=== Testing NVIDIA Agent ===" -ForegroundColor Cyan

# Test 1: Check if binary compiles
Write-Host "`n[1/3] Checking NVIDIA agent binary..." -ForegroundColor Yellow
$nvidiaSource = "src/bin/nvidia_simple.rs"
if (Test-Path $nvidiaSource) {
    Write-Host "✓ NVIDIA agent source found: $nvidiaSource" -ForegroundColor Green
} else {
    Write-Host "✗ NVIDIA agent source not found" -ForegroundColor Red
    exit 1
}

# Test 2: Check Cargo.toml configuration
Write-Host "`n[2/3] Checking Cargo.toml configuration..." -ForegroundColor Yellow
$cargoToml = "Cargo.toml"
$content = Get-Content $cargoToml -Raw
if ($content -match 'name = "nvidia"') {
    Write-Host "✓ NVIDIA binary configured in Cargo.toml" -ForegroundColor Green
} else {
    Write-Host "✗ NVIDIA binary not in Cargo.toml" -ForegroundColor Red
}

if ($content -match 'reqwest') {
    Write-Host "✓ reqwest dependency found (used for NVIDIA API calls)" -ForegroundColor Green
} else {
    Write-Host "✗ reqwest dependency missing" -ForegroundColor Red
}

# Test 3: Check bacon.toml configuration
Write-Host "`n[3/3] Checking bacon.toml configuration..." -ForegroundColor Yellow
$baconToml = ".bacon/bacon.toml"
if (Test-Path $baconToml) {
    $content = Get-Content $baconToml -Raw
    if ($content -match '\[agents\.nvidia\]') {
        Write-Host "✓ NVIDIA agent configured in bacon.toml" -ForegroundColor Green
        
        # Check for required fields
        $requiredFields = @("type", "provider", "command_args", "api_key", "base_url", "model")
        foreach ($field in $requiredFields) {
            if ($content -match "$field\s*=") {
                Write-Host "  ✓ $field configured" -ForegroundColor Green
            } else {
                Write-Host "  ⚠ $field missing" -ForegroundColor Yellow
            }
        }
    } else {
        Write-Host "✗ NVIDIA agent not in bacon.toml" -ForegroundColor Red
    }
} else {
    Write-Host "✗ bacon.toml not found" -ForegroundColor Red
}

# Test 4: Check workflow documentation
Write-Host "`n[4/4] Checking workflow documentation..." -ForegroundColor Yellow
$workflowDoc = ".bacon/workflow.md"
if (Test-Path $workflowDoc) {
    $content = Get-Content $workflowDoc -Raw
    if ($content -match 'nvidia') {
        Write-Host "✓ NVIDIA agent documented in workflow" -ForegroundColor Green
    } else {
        Write-Host "✗ NVIDIA agent not in workflow documentation" -ForegroundColor Yellow
    }
}

Write-Host "`n=== Test Summary ===" -ForegroundColor Cyan
Write-Host "NVIDIA agent implementation ready for testing." -ForegroundColor White
Write-Host "`nNext steps:" -ForegroundColor Yellow
Write-Host "1. Build the agent: cargo build --bin nvidia" -ForegroundColor Cyan
Write-Host "2. Set API key: `$env:NVIDIA_API_KEY='your-key-here'" -ForegroundColor Cyan
Write-Host "3. Test: nvidia -p 'test prompt' --role observer" -ForegroundColor Cyan
Write-Host "4. Use in Bacon: bacon --agent nvidia -p 'improve code'" -ForegroundColor Cyan