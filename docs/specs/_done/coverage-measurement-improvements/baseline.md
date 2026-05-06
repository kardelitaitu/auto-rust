# Baseline

- `coverage.ps1` currently runs `cargo tarpaulin --out Html --output-dir target/reports/coverage`.
- The report is HTML-only, so it is readable by humans but hard to gate or trend automatically.
- `.github/workflows/ci.yml` runs build, lint, and nextest, but there is no coverage job or threshold.
- `TODO.md` calls out `cargo-llvm-cov`, a 40% gate, and coverage trend tracking.
- The current gap is measurement and policy, not missing tests in the application code.
