# Runtime Shutdown Hardening

Status: `done`

Owner: `spec-agent`
Implementer: `implementation-agent`

## Summary

Make shutdown and cancellation deterministic so active work always unwinds cleanly, session state never stays stuck in `Busy`, and release paths remain correct across cancellation timing edges.

## Scope

- In scope:
  - session cleanup boundaries
  - explicit cancellation outcome handling
  - runtime shutdown coordination
  - cancellation propagation into long waits and backoff paths
  - deterministic tests for shutdown timing edges
- Out of scope:
  - CLI parsing and task registry refactors
  - browser discovery/session factory redesign
  - TaskContext interaction pipeline work
  - new browser capability detection

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

Keep this as the archived reference package for shutdown hardening.

# Baseline

## Current State

- `SessionExecutionGuard` already exists and restores session state after task execution paths.
- `TaskAttemptFailure.cancelled` already makes cancellation explicit in orchestrator flow.
- `ShutdownManager` already centralizes signal wiring and shutdown subscription handling.
- Group execution already accepts cancellation token flow through the runtime execution path.

## Known Gaps

- Cancellation coverage is still missing for some timing edges.
- Tests for cancel before worker acquisition, during execution, during backoff, and after page acquisition are not yet complete.
- Some long-running waits and retry loops may still need explicit token propagation.
- The spec still needs a deterministic matrix that proves cleanup rather than just happy-path shutdown.

## Why This Matters

The current implementation has the right building blocks, but the shutdown contract is not yet fully proven across all timing windows. This spec closes that gap.

