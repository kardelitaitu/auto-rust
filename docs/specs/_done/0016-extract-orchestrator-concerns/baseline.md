# Baseline

## What I Find

The orchestrator is the **second-largest file** in the codebase (1623 lines — only `executor.rs` at 1296 is close, and that was already reduced from 4736). It bundles five distinct concerns in one file:

1. **Concurrency guards** (lines 87-216): `GlobalExecutionSlot` (semaphore-based global concurrency), `SessionExecutionGuard` (per-session execution tracking with `Drop` semantics), and `acquire_global_execution_slot()` (async semaphore acquisition with cancellation)

2. **Task dispatch** (lines 282-541): `execute_group()`, `execute_group_with_cancel()`, `execute_task_on_session()` — the core execution pipeline routing tasks to sessions

3. **Retry logic** (lines 142-164, 543-853): `TaskAttemptFailure` error type, `execute_task_with_retry()` — retry loop with exponential backoff

4. **Health & helpers** (lines 58-85, 855-865): `format_duration()`, `broadcast_execution_count()`, `should_mark_session_unhealthy()`

5. **Tests** (lines 867-1623): 756 lines of tests interleaved with production code

```rust
// orchestrator.rs line map:
//   58:  fn format_duration(ms: u64) -> String
//   83:  fn broadcast_execution_count(task_count: usize, session_count: usize) -> usize
//   87:  struct GlobalExecutionSlot     // concurrency guard
//  108:  struct SessionExecutionGuard    // session guard
//  142:  struct TaskAttemptFailure       // retry error type
//  166:  async fn acquire_global_execution_slot  // helper
//  218:  pub struct Orchestrator         // main struct
//  227:  impl Orchestrator
//  282:    execute_group()               // core dispatch
//  293:    execute_group_with_cancel()     // cancel-aware dispatch
//  440:    execute_task_on_session()      // per-session execution
//  543:    execute_task_with_retry()      // retry loop
//  855:    should_mark_session_unhealthy() // health check
//  867:  mod tests { ... 756 lines ... }    // tests
```

## What I Claim

Extracting the five concerns into `src/orchestrator/` submodules will:
- Make each concern independently testable and readable
- Reduce orchestrator.rs from 1623 to ≤400 lines (re-export facade only)
- Follow the same successful pattern as the DSL executor extraction (spec 0014, which reduced executor.rs from 4736→1296)
- Zero behavioral changes — identical test suite passes

## What Is the Proof

**Proof 1 — Monolithic size:** orchestrator.rs at 1623 lines is 60% larger than the spec 0014 ≤1000 target for executor.rs. It's the largest remaining monolith in the codebase after the DLL executor extraction.

**Proof 2 — Distinct concerns:** The file contains 5 struct definitions and 6+ async functions spanning concurrency, dispatch, retry, and health — each with clear, non-overlapping responsibilities. The `GlobalExecutionSlot` (lines 87-106) has zero coupling to `should_mark_session_unhealthy` (line 855).

**Proof 3 — Test/test ratio:** 756 of 1623 lines (47%) are tests. Extracting tests alongside their production code into submodules would make them co-located and self-contained, matching the pattern already established in `src/task/dsl/actions/` (each submodule contains its own `#[cfg(test)] mod tests`).

**Proof 4 — Successful precedent:** Spec 0014 extracted the DSL executor's action handlers into `actions/` submodules (browser.rs 572 lines, wait.rs 174, inspection.rs 158, media.rs 146). The pattern works: `cargo check`, `cargo test --lib`, and `cargo clippy` all pass. The orchestrator extraction follows the identical approach.

**Proof 5 — No dead_code in orchestrator:** Unlike the DSL executor extraction (which dealt with dead_code methods), the orchestrator has zero `#[allow(dead_code)]` annotations. Every function and struct is actively used, making the extraction purely mechanical — move code, update imports, verify.
