# Coverage Measurement Improvements

Status: `done`

Owner: `spec-agent`
Implementer: `implementation-agent`

## Summary

Make coverage reporting reliable enough for integration-heavy changes by moving the measurement path to `cargo-llvm-cov`, adding a CI gate for the coverage floor on the new-code surface, and publishing stable trend outputs. Keep the change tooling-only and leave runtime behavior untouched.

## Scope

- In scope:
  - replace or wrap the current tarpaulin-only flow with `cargo-llvm-cov`
  - measure integration tests in the normal CI path
  - enforce a 40% floor on the coverage surface used for new code
  - emit a machine-readable summary for trend tracking
  - keep a simple local entry point for developers
- Out of scope:
  - runtime code changes
  - task, browser, or session refactors
  - CLI parsing changes
  - changing test semantics or adding large new test suites just for the tool

## Files

- `spec.yaml`
- `baseline.md`
- `internal-api-outline.md`
- `plan.md`
- `validation-checklist.md`
- `ci-commands.md`
- `decisions.md`
- `quality-rules.md`
- `implementation-notes.md`

## Next Step

Hand this package to the implementer after the coverage workflow and CI policy are accepted.
