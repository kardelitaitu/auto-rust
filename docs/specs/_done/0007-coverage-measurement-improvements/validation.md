# Validation Checklist

- `coverage.ps1` runs from the repo root.
- The coverage run includes integration tests.
- A machine-readable summary file or report is created.
- The CI job fails below the 40% floor.
- A trend artifact is produced with stable fields.
- `.\check-fast.ps1` still works for local iteration.
- `.\check.ps1` still passes before handoff.
- `spec-lint.ps1` stays green for the package.

# CI Commands

## Local

- `.\coverage.ps1`
- `cargo llvm-cov --all-features --workspace --html --output-dir target/reports/coverage`
- `cargo llvm-cov --all-features --workspace --lcov --output-path target/reports/coverage/lcov.info`
- `cargo llvm-cov --all-features --workspace --json --output-path target/reports/coverage/coverage.json`
- `cargo llvm-cov --all-features --workspace --fail-under-lines 40`

## CI

- `taiki-e/install-action@cargo-llvm-cov`
- `cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info`
- `cargo llvm-cov --all-features --workspace --json --output-path coverage.json`

# Quality Rules

- The coverage command must be deterministic and runnable from the repo root.
- Coverage policy must be explicit in the workflow, not hidden in ad hoc scripts.
- Trend output must be stable enough to compare across runs.
- The gate must not block on flaky formatting or unrelated build noise.
- Coverage tooling must not replace real test coverage work.
- Keep the local command short enough for repeated use during iteration.

