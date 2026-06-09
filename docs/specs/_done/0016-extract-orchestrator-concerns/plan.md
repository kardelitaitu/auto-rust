# Plan

## What Is the Solution

Create `src/orchestrator/` directory with submodules, then extract each concern:

### Step 1 — Create submodule files

```
src/orchestrator/
  mod.rs        — Orchestrator struct, new(), pub re-exports (≤100 lines)
  guards.rs     — GlobalExecutionSlot, SessionExecutionGuard, acquire_global_execution_slot + tests (≤250 lines)
  execution.rs  — execute_group, execute_group_with_cancel, execute_task_on_session + tests (≤400 lines)
  retry.rs      — TaskAttemptFailure, execute_task_with_retry + tests (≤250 lines)
  health.rs     — format_duration, broadcast_execution_count, should_mark_session_unhealthy + tests (≤150 lines)
  test_utils.rs — create_test_config(), connect_test_session() shared by all submodule tests
```

### Step 2 — Move code into submodules

For each submodule:
1. Copy the relevant functions/structs/impls from `orchestrator.rs`
2. Add necessary imports (`tokio`, `parking_lot`, `std::sync::Arc`, `AtomicUsize`, etc.)
3. Move associated `#[cfg(test)]` tests into the submodule
4. Update visibility: `pub(super)` for items used by sibling modules, `pub` for public API

### Step 3 — Wire `mod.rs`

Replace the extracted bodies with re-exports:
```rust
mod guards;
mod execution;
mod retry;
mod health;

pub use guards::{GlobalExecutionSlot, SessionExecutionGuard};
pub use execution::OrchestratorExt; // trait with execute_group, etc.
// ... etc
```

### Step 4 — Verify

```bash
cargo check -p auto-rust
cargo test --lib orchestrator
cargo clippy --all-targets --all-features
```

### Key constraints

- **No behavioral changes** — identical test suite must pass
- **No public API breakage** — `Orchestrator::new()`, `execute_group()` signatures unchanged
- **Drop semantics preserved** — `GlobalExecutionSlot` and `SessionExecutionGuard` Drop impls must remain identical
- **Async fn extraction** — ensure tokio runtime is available in submodules (it is — it's a workspace dependency)

### Files changed

| File | Action | Target lines |
|------|--------|-------------|
| `src/orchestrator.rs` | → renamed to `src/orchestrator/mod.rs` | ≤100 |
| `src/orchestrator/guards.rs` | New — concurrency guards + acquire helper | ≤250 |
| `src/orchestrator/execution.rs` | New — group/task dispatch | ≤400 |
| `src/orchestrator/retry.rs` | New — retry logic | ≤250 |
| `src/orchestrator/health.rs` | New — health helpers | ≤150 |
| `src/orchestrator/test_utils.rs` | New — shared test helpers | ≤50 |
| `src/main.rs` | Update import if needed | — |
| `src/lib.rs` | Update `pub mod orchestrator;` → `pub mod orchestrator { pub mod ... }` | — |
