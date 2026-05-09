# TwitterActivity Orchestrator Stability

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Make `src/task/twitteractivity.rs` dependable end to end by keeping the thin orchestrator contract stable, validating timeout and payload handling, and pinning helper-module wiring with regression checks.

## Scope

- In scope:
  - task entrypoint timeout handling
  - payload parsing and config handoff
  - persona selection and behavior profile blending
  - feed scan loop and candidate processing wiring
  - summary logging and limit reporting
  - targeted regression tests for the task shell
- Out of scope:
  - rewriting helper modules under `src/utils/twitter/`
  - adding new engagement behavior
  - changing selector strategy
  - UI redesign or browser flow changes outside the task contract

## Files

- `spec.yaml`
- `baseline.md`
- `internal-api-outline.md`
- `plan.md`
- `validation-checklist.md`
- `validation.md`
- `notes.md`
- `ci-commands.md`
- `decisions.md`
- `quality-rules.md`
- `implementation-notes.md`

## Notes

- Keep the spec small and test-first.
- Keep the task shell thin.
- Prefer fixes that strengthen contract boundaries over broad refactors.
