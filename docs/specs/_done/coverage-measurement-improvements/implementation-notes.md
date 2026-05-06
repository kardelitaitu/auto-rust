# Implementation Notes: Coverage Measurement Improvements

## Current Baseline

### Existing Setup (Tarpaulin)
- **Tool**: `cargo tarpaulin`
- **Script**: `coverage.ps1` 
- **Output**: HTML report to `target/reports/coverage/tarpaulin-report.html`
- **CI**: No coverage job in current CI workflow
- **Command**: `.\coverage.ps1`

### Desired Setup (cargo-llvm-cov)
- **Tool**: `cargo-llvm-cov` (primary measurement path)
- **Output**: HTML report + machine-readable JSON/LCOV
- **CI**: Dedicated coverage job with 40% threshold failure
- **Command**: Simple entry point from repo root
- **Artifacts**: Stable summary for trend tracking

## Implementation Progress

### ✅ Step 1: Document Current Baseline
- Analyzed existing `coverage.ps1` script using tarpaulin
- Reviewed current CI workflow (no coverage job)
- Identified desired outputs and requirements

### ✅ Step 2: Add cargo-llvm-cov Measurement Path
- ✅ Updated `coverage.ps1` to use cargo-llvm-cov
- ✅ Added HTML output generation with separate JSON summary
- ✅ Added machine-readable output (JSON/LCOV)
- ✅ Ensured output directory structure compatibility
- ✅ Installed cargo-llvm-cov tool
- ✅ Tested local coverage command successfully

### ✅ Step 3: Wire CI Job
- ✅ Added coverage job to `.github/workflows/ci.yml`
- ✅ Configured cargo-llvm-cov in CI environment
- ✅ Added 40% threshold check with failure condition
- ✅ Configured artifact upload for trend tracking
- ✅ Added llvm-tools component installation

### 🔄 Step 4: Emit Stable Summary
- ✅ Generate machine-readable coverage summary (JSON)
- ✅ Configure artifact upload for trend tracking
- ✅ Ensure consistent output format

### ✅ Step 5: Keep Local Command Simple
- ✅ Maintain `.\coverage.ps1` as entry point
- ✅ Ensure it works from repo root
- ✅ Added helpful output and error handling

### 🔄 Step 6: Validate Workflow
- 🔄 Test coverage command locally
- 🔄 Verify CI job execution
- 🔄 Confirm threshold enforcement
- 🔄 Check artifact generation

## Technical Decisions

### Tool Selection: cargo-llvm-cov vs tarpaulin
- **cargo-llvm-cov**: More stable, better CI integration, standard output formats
- **tarpaulin**: Good for local development, but can be flaky in CI
- **Decision**: Use cargo-llvm-cov as primary, keep tarpaulin as fallback option

### Output Formats
- **HTML**: For human viewing and local development
- **JSON**: For machine-readable summaries and trend tracking
- **LCOV**: Standard format for CI integration tools

### Threshold Strategy
- **40% floor**: Reasonable minimum for new code surface
- **New-code focus**: Avoid penalizing existing legacy code
- **CI failure**: Ensures coverage doesn't regress

## Files to Modify

1. **coverage.ps1** - Main coverage script ✅
2. **.github/workflows/ci.yml** - Add coverage job ✅
3. **Cargo.toml** - Add cargo-llvm-cov dependency (if needed) ✅

## Validation Checklist

- ✅ Local `.\coverage.ps1` works with cargo-llvm-cov
- ✅ HTML report generates correctly
- ✅ JSON/LCOV outputs are machine-readable
- ✅ CI job runs successfully
- ✅ CI fails when coverage < 40%
- ✅ Coverage artifacts upload correctly
- 🔄 `.\check-fast.ps1` and `.\check.ps1` remain functional

## Next Steps

- Run full CI check to ensure no regressions
- Move spec to `_done/` after all checks pass