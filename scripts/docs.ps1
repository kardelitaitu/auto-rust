# Documentation Generation Script
# Generates API docs and architecture documentation

param([string]$Target = "target/doc")

# Ensure output directory exists
if (!(Test-Path $Target)) {
    New-Item -ItemType Directory -Force $Target | Out-Null
}

Write-Host "=== Documentation Generation ===" -ForegroundColor Cyan

# Generate rustdoc for all features
Write-Host "Generating API documentation (cargo doc --all-features)..." -ForegroundColor Yellow
cargo doc --all-features --no-deps 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "✗ API documentation generation failed" -ForegroundColor Red
    exit 1
}
Write-Host "✓ API documentation generated -> target/doc/index.html" -ForegroundColor Green

# Copy architecture document to docs output
if (Test-Path "docs/ARCHITECTURE.md") {
    Copy-Item "docs/ARCHITECTURE.md" "$Target/architecture.md" -Force
    Write-Host "✓ Architecture document copied to $Target/architecture.md" -ForegroundColor Green
}

# Summary
Write-Host ""
Write-Host "Documentation generation complete!" -ForegroundColor Cyan
Write-Host "  - API docs: $Target/index.html" -ForegroundColor White
Write-Host "  - Architecture: $Target/architecture.md" -ForegroundColor White
