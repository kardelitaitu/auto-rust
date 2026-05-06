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
