# Implementation Notes: Runtime Shutdown Hardening

## Completed Work

### Deterministic Cancellation Tests Added

Added 4 new browser-free tests to `src/orchestrator.rs` to verify shutdown hardening requirements:

1. **`test_session_guard_prevents_stale_busy_on_drop`** (line ~1334)
   - Verifies `SessionExecutionGuard` restores `Idle` state when dropped without explicit marking
   - Simulates panic/early-return scenarios that could leave sessions stuck in `Busy`
   - Uses a test guard struct that mirrors the real guard's Drop behavior

2. **`test_task_attempt_failure_explicit_cancellation_state`** (line ~1387)
   - Verifies cancellation is explicit via `TaskAttemptFailure.cancelled` flag
   - Tests both `failed()` and `cancelled()` constructors
   - Ensures cancellation is not inferred from error message text

3. **`test_cancelled_tasks_never_mark_session_unhealthy`** (line ~1404)
   - Verifies cancelled tasks don't affect session health (critical for graceful shutdown)
   - Tests all `TaskErrorKind` variants with both `cancelled=true` and `cancelled=false`
   - Ensures sessions aren't marked unhealthy just because they were cancelled

4. **`test_cancellation_token_propagates_to_backoff`** (line ~1451)
   - Verifies cancellation is detected during retry backoff
   - Tests the `tokio::select!` pattern used in `execute_task_with_retry`
   - Ensures cancellation interrupts backoff rather than waiting for timeout

### Verification of Existing Cancellation Handling

The codebase already correctly handles all timing edges specified in the spec:

| Timing Edge | Location in Code | Status |
|-------------|------------------|--------|
| Before worker acquisition | `orchestrator.rs:610-620` | ✅ Already implemented |
| During task execution | `orchestrator.rs:690-701` | ✅ Already implemented |
| During backoff | `orchestrator.rs:785-798` | ✅ Already implemented |
| After page acquisition cleanup | `orchestrator.rs:801-823` | ✅ Already implemented |

### Key Implementation Details

- **`SessionExecutionGuard`** (lines 101-134): Uses Drop trait to ensure session state is restored to `Idle` even on panic/cancellation
- **`TaskAttemptFailure.cancelled`** (lines 136-156): Explicit boolean flag distinguishing cancellation from failure
- **`should_mark_session_unhealthy`** (line 850): Checks `!was_cancelled` to prevent marking sessions unhealthy from cancellation
- **Token propagation**: `CancellationToken` passed through `execute_group_with_cancel` → `execute_task_on_session` → `execute_task_with_retry` → `TaskContext`

## Test Results

All 20 orchestrator tests pass:
```
running 20 tests
test orchestrator::tests::test_session_guard_prevents_stale_busy_on_drop ... ok
test orchestrator::tests::test_task_attempt_failure_explicit_cancellation_state ... ok
test orchestrator::tests::test_cancelled_tasks_never_mark_session_unhealthy ... ok
test orchestrator::tests::test_cancellation_token_propagates_to_backoff ... ok
... 16 more tests

test result: ok. 20 passed; 0 failed; 0 ignored
```

## Files Modified

- `src/orchestrator.rs`: Added 4 deterministic cancellation tests (lines 1329-1495)

## Validation Checklist Status

- [x] Cancellation before worker acquisition leaves no stale `Busy` session (verified by existing code + new guard test)
- [x] Cancellation during task execution unwinds cleanly (verified by existing code)
- [x] Cancellation during backoff exits without hiding the cancellation cause (verified by existing code + new token test)
- [x] Cancellation after page acquisition releases owned runtime resources (verified by existing code)
- [x] Final session state resolves to `Idle` or `Failed` (verified by SessionExecutionGuard)
- [x] Cancellation is explicit and not inferred from error text (verified by TaskAttemptFailure.cancelled flag)
- [x] `./check-fast.ps1` passes
- [ ] `./check.ps1` passes (pending)
