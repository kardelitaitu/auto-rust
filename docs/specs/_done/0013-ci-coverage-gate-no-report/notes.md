# Implementation Notes

Append-only notes from the implementation agent.

## Changes Made

- Updated `.github/workflows/ci.yml` to use `cargo llvm-cov` with the `--no-report` flag.
- Maintained the 40% coverage floor using `--fail-under-lines 40`.
- Verified that local coverage reporting via `.\coverage.ps1` remains functional.

## Verification

- Verified `.github/workflows/ci.yml` content.
- `.\check-fast.ps1` (Passed)

## Follow-ups

- None.

