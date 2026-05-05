# Implementation Notes

Status: verified

## Completed

- Moved the benchmark harnesses from root `benches/` into `src/benchmarks/`.
- Added a minimal `PredictiveEngagementScorer` benchmark seam so the harness uses the real library code.
- Kept the accessibility locator benchmark feature-gated and wired Cargo to the new path.
- Removed the empty root `benches/` folder after verification.

## Verification

- `cargo check`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo check --benches --all-features`
- `.\check.ps1`
