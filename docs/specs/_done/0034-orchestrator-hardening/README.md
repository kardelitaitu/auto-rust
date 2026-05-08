# Orchestrator Hardening

Status: `done`

Owner: `spec-agent`
## Summary

Tighten regression coverage around `src/orchestrator.rs` so the group-timeout branch, task-cancellation cleanup, and guard-state behavior stay stable without changing the scheduling model or retry policy.

## Scope

- In scope:
  - `execute_group_with_cancel` group-timeout branch and shutdown cancellation branch
  - `execute_task_with_retry` cancellation before worker acquisition
  - session idle cleanup after guarded task cancellation
  - regression tests for the existing result aggregation invariant
- Out of scope:
  - scheduler policy changes
  - retry/backoff algorithm changes
  - browser discovery or session assembly changes
  - task parsing or CLI changes

## Baseline

- `src/orchestrator.rs` already contains `SessionExecutionGuard`, `execute_group_with_cancel`, and `execute_task_with_retry`.
- The file already has tests for guard behavior, result aggregation, and cancellation-related paths.
- The remaining risk is timing-sensitive regression coverage, not missing core logic.

## Why This Was Needed

- The existing retry-cancellation test covered backoff waiting, but not the group-timeout branch in `execute_group_with_cancel`.
- The task-level cancel path before worker acquisition needed a deterministic regression check.
- Result aggregation already existed, and the archived spec pinned the exact success/failure counts for the three-result test set.

## Files

- `spec.yaml`
- `plan.md`
- `validation.md`
- `notes.md`

## Archive Notes

This package is complete and retained as a reference record for orchestrator hardening.

# Baseline Notes

- Cancellation and timeout behavior already exists in the orchestrator.
- The spec should prove those paths remain stable under test, not redesign them.
