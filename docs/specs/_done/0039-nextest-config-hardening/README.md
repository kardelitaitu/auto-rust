# Nextest Config Hardening

Status: `done`

Owner: `spec-agent`
Implementer: `archived-pending`

## Summary

Harden the CI nextest profile so the test runner is more reliable and easier to debug without changing test semantics. Keep the repo fast, but make the CI profile explicit about failure output, retries, and timeout behavior.

## Scope

- In scope:
  - CI nextest profile settings in `.config/nextest.toml`
  - failure-output and retry policy for CI runs
  - timeout behavior for long-running or stuck tests in CI
- Out of scope:
  - runtime code changes
  - test logic changes
  - coverage reporting changes
  - any browser/session refactor

## Baseline

- `.config/nextest.toml` currently only sets `[profile.ci] fail-fast = true`.
- `check.ps1` runs `cargo nextest run --all-features --lib` and already passes.
- The profiling pass did not find tests slower than 100ms in the current lib suite.
- The remaining value is CI hardening, not more timing discovery.

## Why This Was Needed

- The repo already has a working nextest migration, but the CI profile is still minimal.
- A stricter CI profile gives better failure visibility and more predictable recovery from flaky tests.
- Because the suite is already fast, this is a low-risk way to improve reliability without adding runtime work.

## Files

- `spec.yaml`
- `plan.md`
- `validation.md`
- `notes.md`

## Next Step

This package is complete and retained as a reference record for nextest config hardening.
