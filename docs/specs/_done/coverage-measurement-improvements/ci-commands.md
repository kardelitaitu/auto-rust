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
