# Task Module Coverage Improvement

Status: `done`

Owner: `spec-agent`
Implementer: `code-implementer`

## Summary

Raise confidence in the under-tested task modules by adding focused unit coverage for pure parsing, candidate ordering, and branch-control helpers in `src/task/twitteractivity.rs`, `src/task/twitterfollow.rs`, and `src/task/twitterintent.rs`. Keep the public task behavior stable and avoid broad refactors.

## Scope

- In scope:
  - helper and branch coverage in the three target task modules
  - small fixture helpers if needed for tests
  - narrow regression checks for run-path behavior only when pure unit tests are not enough
- Out of scope:
  - CLI registry and task parsing refactors
  - browser discovery/session assembly
  - runtime shutdown
  - task API redesign
  - coverage tooling or CI policy changes

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

## Completed

The task module coverage work is complete with comprehensive tests already in place:

- `twitteractivity.rs`: 2 test modules covering config parsing and entry-point selection
- `twitterfollow.rs`: 35+ tests covering URL/username extraction and locator candidate ordering
- `twitterintent.rs`: 30+ tests covering intent parsing and payload URL extraction
- All 2243 tests pass, CI checks pass

# Baseline

## Current State

- `TODO.md` still lists `src/task/*.rs` as the next medium-priority coverage candidate.
- The specific files called out are `twitteractivity.rs`, `twitterfollow.rs`, and `twitterintent.rs`.
- `twitteractivity.rs` has a small helper surface around payload fixtures and entry-point selection.
- `twitterfollow.rs` contains locator candidate ordering plus URL and username parsing helpers.
- `twitterintent.rs` contains intent parsing, payload URL extraction, and query-parameter helpers.

## Known Gaps

- The three target files still have uneven coverage, with some pure helper branches not directly exercised.
- The broad `src/task/*.rs` area is too large for one pass, so this spec should stay on the three named modules.
- Browser-backed tests can mask the real gap if the work drifts away from helper coverage.

## Why This Matters

These modules sit on the task execution path and carry most of the parsing and branch-control logic for Twitter-oriented task behavior. The safest improvement is to add deterministic tests around the pure helpers first, then only add narrow integration checks if a behavior path cannot be covered otherwise.

