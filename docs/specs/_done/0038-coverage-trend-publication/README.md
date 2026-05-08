# Coverage Trend Publication

Status: `done`

Owner: `spec-agent`
Implementer: `archived-pending`

## Summary

Publish the existing machine-readable coverage summary from CI so coverage numbers can be compared across runs without changing runtime behavior or the 40% gate. Keep the local `coverage.ps1` flow intact and make the CI output easy to retain and inspect later.

## Scope

- In scope:
  - stable coverage summary output in the CI coverage job
  - artifact or comparable retained publication for the summary data
  - documentation of the summary path and local usage
- Out of scope:
  - a full coverage dashboard
  - runtime code changes
  - changing test semantics
  - changing the coverage threshold

## Baseline

- `coverage.ps1` already writes HTML and JSON coverage output under `target/reports/coverage`.
- `.github/workflows/ci.yml` already enforces the 40% coverage floor with `cargo llvm-cov`.
- The CI workflow does not currently publish a retained coverage summary for later comparison.
- `TODO.md` still tracks coverage trends over time as an open item.

## Why This Was Needed

- The repo already had a local JSON coverage summary, but nothing durable in CI for comparing runs over time.
- The existing gate proved coverage stays above the floor, but it did not preserve the numbers for trend review.
- Publishing the summary is the smallest change that makes trend tracking practical without expanding scope into a dashboard.

## Files

- `spec.yaml`
- `plan.md`
- `validation.md`
- `notes.md`

## Archive Notes

This package is complete and retained as a reference record for coverage trend publication.
