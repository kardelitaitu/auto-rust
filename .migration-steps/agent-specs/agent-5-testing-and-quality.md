# Agent 5 Spec: Test Matrix, Stability Checks, And Quality Gates

## Scope
Own test quality, command correctness, and validation mapping.

## Files
- `src/tests/*`
- `migration-steps/phase-5-testing-validation.md` (if test docs must be synced)

## Functional Requirements
- Keep tests aligned with string-normalized parser contract.
- Ensure commands in docs map to actual project layout (`src/tests` module style).
- Add/adjust tests for smoke command parsing and edge cases.

## Acceptance Checks
- `cargo test` passes for deterministic unit tests.
- Parser smoke test case exists and passes.
- Numeric payload assertion matches string contract.
- No invalid test command in docs.

## Out Of Scope
- Browser production hardening
- Packaging scripts

## Report Format
- Tests added/updated
- Fast suite vs long-running suite split
- Flaky-risk notes
