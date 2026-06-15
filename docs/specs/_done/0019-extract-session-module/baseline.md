# Baseline

## What I Find

`src/session/mod.rs` is 1,951 lines with these types mixed in a single file:

| Lines | Content |
|-------|---------|
| 1-14 | Imports + module declarations (connector, factory, pool, cleanup) |
| 31-134 | `DurationMs` struct + 5 impl blocks (new, get, display, operators) |
| 135-146 | `SessionState` enum (Idle, Busy, Failed) |
| 147-194 | `WorkerPermit` struct + Drop impl |
| 195-248 | `Session` struct with 20 fields |
| 249-556 | `impl Session` #1 — new(), page tracking (register/unregister/active_page_count), health (is_healthy/mark_healthy/mark_unhealthy/increment_failure/get_failure_count), state management (is_idle/mark_busy/mark_idle/mark_failed), browser accessors (ws_endpoint/browser_type/is_connected/state/idle_since/browser_version/process_info/session_info/health_status) |
| 582-960 | `impl Session` #2 — acquire_worker(), cb_check(), cb_record_success(), cb_record_failure(), acquire_page(), acquire_page_at(), release_page(), cleanup_managed_pages(), graceful_shutdown() |
| 557-566 | `unix_timestamp_secs()` helper |
| 567-619 | `is_circuit_breaker_open_pure()` + `#[cfg(test)]` unit tests |
| 620-961 | Session tests (mark_busy, mark_idle, mark_failed, state transitions) |
| 962-963 | Cleanup module declaration |
| 964-1951 | `#[cfg(test)] mod tests` — comprehensive test suite (~987 lines) |

Existing submodules: connector.rs (662), factory.rs (328), pool.rs (630), cleanup.rs (180) = 1,800 lines

## What I Claim

Extracting the 4 type groups (DurationMs, WorkerPermit, Session, SessionState) into submodules will reduce session/mod.rs from 1,951 to ≤200 lines while maintaining full backward compatibility through re-exports. This follows the exact pattern proven in specs 0016 (orchestrator), 0017 (twitter state), and 0018 (config).

## What Is the Proof

1. **11 files import `crate::session::DurationMs`** — re-export chain preservation is critical. Every file from `api/client.rs` to `orchestrator/mod.rs` uses `DurationMs`. The extraction must ensure `pub use duration::*` flows through `mod.rs`.

2. **Already modularized structure**: session/ already has 4 submodules in a flat directory. Adding more (duration, permits, session, state) matches the established pattern and avoids nested complexity.

3. **Clean type boundaries**: DurationMs (104 lines) is a standalone type with no Session dependencies. WorkerPermit (48 lines) depends only on Session. Session has two impl blocks (~307 + ~378 lines) that can split into core + operations. SessionState (12 lines) is a simple enum. These are naturally separable.
