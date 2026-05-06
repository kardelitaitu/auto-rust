# Plan

## Step 1

Document the current tarpaulin baseline and the desired coverage outputs.

## Step 2

Add a `cargo-llvm-cov` measurement path that can produce HTML and machine-readable output.

## Step 3

Wire the CI job to run the coverage path and fail when the threshold is below 40%.

## Step 4

Emit a stable summary artifact for trend tracking and keep the local command easy to run.

## Step 5

Validate the workflow with the repo checks and the coverage command itself.

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

# Decisions

| Choice | Pros | Cons |
|---|---|---|
| `cargo tarpaulin` only | Already exists and writes a simple HTML report | Harder to gate, harder to trend, weaker integration-test story |
| `cargo-llvm-cov` | Supports integration-test coverage, lcov/json output, and explicit fail-under thresholds | Adds a new tool and a slightly heavier CI setup |

## Decision

Use `cargo-llvm-cov` as the source of truth for policy and trend data. Keep `coverage.ps1` as the simple local entry point, but let it produce structured output that CI can consume.

## Notes

- Keep the coverage gate in CI, not in `check-fast.ps1`.
- Use machine-readable output for trend tracking instead of a committed manual report.
- Keep tarpaulin out of the new policy path unless a fallback is needed.

