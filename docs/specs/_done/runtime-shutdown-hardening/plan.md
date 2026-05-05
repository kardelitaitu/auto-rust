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
