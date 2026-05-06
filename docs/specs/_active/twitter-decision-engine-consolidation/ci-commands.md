# CI Commands

## Development Loop

```powershell
# Quick check during development
.\check-fast.ps1
```

## Pre-Commit Verification

```powershell
# Full verification before handoff
.\check.ps1
```

## Specific Checks

```powershell
# Run only twitter-related tests
cargo test twitteractivity_decision

# Check compilation of decision module
cargo check --lib 2>&1 | Select-String -Pattern "decision"

# Clippy focused on new code
cargo clippy --package rust-orchestrator --lib -- -W clippy::unwrap_used 2>&1 | Select-String -Pattern "decision"
```

## Coverage

```powershell
# Run coverage report
.\coverage.ps1

# Check decision module coverage
cargo tarpaulin --out Stdout 2>&1 | Select-String -Pattern "decision"
```

## Spec Lint

```powershell
# Validate spec package
.\spec-lint.ps1
```
