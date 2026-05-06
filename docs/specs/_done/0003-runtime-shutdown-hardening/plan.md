# Plan

## Step 1: Baseline and contracts

- Confirm the current shutdown and session cleanup flow.
- Re-read the recent journal entries for the guard, cancellation, and shutdown manager work.
- Lock the acceptance criteria in `spec.yaml` before any code changes.

## Step 2: Deterministic cancellation tests

- Add a browser-free test for cancellation before worker acquisition.
- Add a test for cancellation during task execution.
- Add a test for cancellation during backoff.
- Add a test for cancellation after page acquisition.

## Step 3: Runtime cleanup behavior

- Ensure cleanup always resolves the session state.
- Ensure page and permit release happen on the canceled path.
- Keep cancellation explicit in the failure model.

## Step 4: Token propagation

- Propagate `CancellationToken` into any long wait, retry, or backoff path that can block shutdown.
- Verify that cancellation interrupts those paths rather than waiting for timeout.

## Step 5: Verification

- Run `.
check-fast.ps1` during iteration.
- Run `.
check.ps1` before handoff.
- Move the spec to `_done/` only after the full gate passes.

# Internal API Outline

## Ownership Boundaries

- `src/runtime/shutdown.rs`
  - Own shutdown signal wiring and shutdown subscription behavior.
  - Keep the shutdown manager the single place that translates external shutdown events into runtime control flow.

- `src/runtime/execution.rs`
  - Own active group execution, cancellation token wiring, and shutdown waiting behavior.
  - Stop new work when shutdown has begun, then wait for active work to unwind.

- `src/orchestrator.rs`
  - Own session state transitions around work start, cleanup, and failure classification.
  - Keep cancellation handling explicit.

- `src/session/mod.rs`
  - Own session state machine invariants and release semantics.
  - Never leave a session stuck in `Busy` after a canceled attempt.

- `src/utils/navigation.rs`
  - Accept cancellation-aware waits where long selector waits must be interruptible.

## API Shape Rules

- Do not add a new public surface unless a test proves the existing surface cannot express the shutdown contract.
- Prefer small internal helpers over broad refactors.
- Prefer explicit cancellation state over string matching or inferred failure classification.

# Decisions

- Use explicit cancellation state instead of parsing error message text.
- Keep shutdown coordination centralized in the runtime shutdown manager.
- Prefer deterministic tests over browser-heavy timing tests whenever possible.
- Keep this spec focused on shutdown correctness, not on broader task or CLI refactors.

