## Implementation Notes

- Updated `.github/workflows/ci.yml` to write `target/reports/coverage/coverage.json` during the coverage gate.
- Added a coverage output directory step so the JSON report path exists before `cargo llvm-cov` writes it.
- Added an `actions/upload-artifact@v4` step to retain the coverage summary as `coverage-summary`.
- Kept `coverage.ps1` unchanged so local HTML and JSON output behavior stayed the same.
