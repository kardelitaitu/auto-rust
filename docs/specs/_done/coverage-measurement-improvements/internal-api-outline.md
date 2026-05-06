# Internal Boundary Outline

- `coverage.ps1`
  - local entry point for developers
  - should stay simple to run from the repo root
  - may grow options for HTML, lcov, and JSON output
- `cargo-llvm-cov` invocation layer
  - owns coverage collection and report generation
  - must include integration-test execution
  - must emit machine-readable output for the gate and trend summary
- CI workflow layer
  - owns install, run, artifact upload, and threshold enforcement
  - should fail the job when the configured floor is not met
- Trend output layer
  - owns a stable summary artifact or file
  - should not depend on manual post-processing
- Not owned here
  - runtime code paths
  - CLI dispatch
  - task, browser, or session abstractions
