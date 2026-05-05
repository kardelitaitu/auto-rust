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
