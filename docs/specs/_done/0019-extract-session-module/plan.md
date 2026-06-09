# Plan

## What Is the Solution

Extract 4 type groups from `session/mod.rs` into new flat submodules under `src/session/`:

| New File | Content | Source Lines | Target |
|----------|---------|-------------|--------|
| `duration.rs` | `DurationMs` struct + 5 impl blocks | 31-134 | ≤150 |
| `permits.rs` | `WorkerPermit<'a>` struct + Drop impl | 147-194 | ≤70 |
| `session_core.rs` | `Session` struct + `impl Session` #1 (new, page tracking, health, state management methods) | 195-556 | ≤400 |
| `session_ops.rs` | `impl Session` #2 (acquire_worker, cb_check, cb_record_*, acquire_page, release_page, cleanup_managed_pages, graceful_shutdown) | 582-960 | ≤400 |
| `state.rs` | `SessionState` enum + `is_circuit_breaker_open_pure()` + helpers | 135-146, 557-619 | ≤100 |
| `mod.rs` | Module declarations + re-exports + `unix_timestamp_secs()` | shrunken | ≤200 |

**Test distribution**: Session tests (620-961) stay in `session.rs` as `#[cfg(test)]`; comprehensive test suite (964-1951) stays in `mod.rs` as `#[cfg(test)] mod tests`.

**Re-export chain**: `pub use duration::*; pub use permits::*; pub use session::*; pub use state::*;` in `mod.rs` ensures all existing `crate::session::DurationMs` paths continue to work.

**11-file import graph preserved**: All consumers (`api/client.rs`, `config/`, `orchestrator/`, etc.) import via `use crate::session::DurationMs` — unchanged by this refactor.
