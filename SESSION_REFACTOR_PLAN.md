# Session Refactor Plan

> **Status:** Proposed — not yet implemented  
> **Risk:** Medium | **Reward:** Medium | **Effort:** ~2 days  
> **Created:** August 2026

## Problem

`Session` in `src/session/mod.rs` is a god-object with **22 fields** and **9 constructor parameters**. It mixes identity, browser connection, worker scheduling, health tracking, circuit breaker logic, and cursor overlay state into a single struct. While the internal machinery is well-encapsulated behind methods, the sheer size makes it hard to reason about, test in isolation, or modify without side-effect risk.

## Proposed Split: 4 Sub-Structs

```
Session (facade — ~10 fields)
├── id, name, profile_type                // identity
├── behavior_profile, behavior_runtime    // behavior config
├── browser, browser_ws_url, handler_task // browser connection
├── active_pages: DashSet<TargetId>       // page registry
├── workers: WorkerPool                   // NEW — groups 3 fields
├── health: SessionHealth                 // NEW — groups 4 fields + state
├── circuit_breaker: CircuitBreaker       // NEW — groups 4 fields
└── overlay: Option<OverlayHandle>        // NEW — groups 3 fields
```

### Sub-Struct 1: `WorkerPool`

**File:** `src/session/workers.rs` (new)

```rust
pub(crate) struct WorkerPool {
    semaphore: Arc<Semaphore>,
    pub max_workers: usize,
    pub active_workers: AtomicUsize,
}

impl WorkerPool {
    pub fn new(max_workers: usize) -> Self;
    pub async fn acquire(&self) -> tokio::sync::SemaphorePermit<'_>;
    pub fn has_capacity(&self) -> bool;
    pub fn is_idle(&self) -> bool;
    pub fn is_busy(&self) -> bool;
}
```

**Replaces:** `worker_semaphore`, `max_workers`, `active_workers` fields  
**Moves from:** `lifecycle.rs` methods `acquire_worker()`, `has_worker_capacity()`, `is_worker_idle()`

---

### Sub-Struct 2: `SessionHealth`

**File:** `src/session/health.rs` (new)

```rust
pub(crate) struct SessionHealth {
    failure_count: AtomicUsize,
    is_healthy: AtomicBool,
    state: parking_lot::Mutex<SessionState>,
}

impl SessionHealth {
    pub fn new() -> Self;
    pub fn is_healthy(&self) -> bool;
    pub fn mark_healthy(&self);
    pub fn mark_unhealthy(&self);
    pub fn increment_failure(&self) -> usize;
    pub fn failure_count(&self) -> usize;
    pub fn state(&self) -> SessionState;
    pub fn set_state(&self, state: SessionState);
}
```

**Replaces:** `failure_count`, `is_healthy`, `state` fields  
**Moves from:** `lifecycle.rs` methods `mark_healthy()`, `mark_unhealthy()`, `check_health()`, `record_failure()`

---

### Sub-Struct 3: `CircuitBreaker`

**File:** `src/session/circuit_breaker.rs` (new)

```rust
pub(crate) struct CircuitBreaker {
    failure_count: Arc<AtomicUsize>,
    failure_threshold: usize,
    timeout_secs: u64,
    last_failure_time: Arc<AtomicUsize>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: usize, timeout_secs: u64) -> Self;
    pub fn is_open(&self) -> bool;
    pub fn record_success(&self);
    pub fn record_failure(&self);
    pub fn check(&self) -> bool;
    pub fn reset(&self);
    pub fn state(&self) -> CircuitState;
}
```

**Replaces:** `cb_failure_count`, `cb_failure_threshold`, `cb_timeout_secs`, `cb_last_failure_time` fields  
**Moves from:** `lifecycle.rs` methods `check_circuit_breaker()`, `record_circuit_breaker_failure()`

---

### Sub-Struct 4: `OverlayHandle`

**File:** `src/session/overlay.rs` (new)

```rust
pub(crate) struct OverlayHandle {
    cursor_overlay_ms: u64,
    overlay_state: Mutex<OverlayState>,
    overlay_task: Option<JoinHandle<()>>,
}

impl OverlayHandle {
    pub fn new(cursor_overlay_ms: u64) -> Self;
    pub async fn activate(&mut self, ws_url: &str) -> anyhow::Result<()>;
    pub async fn deactivate(&mut self) -> anyhow::Result<()>;
    pub fn is_active(&self) -> bool;
    pub async fn shutdown(&mut self);
}
```

**Replaces:** `cursor_overlay_ms`, `overlay_state`, `overlay_task` fields  
**Moves from:** `lifecycle.rs` methods `activate_overlay()`, `deactivate_overlay()`, overlay checks

---

## Refactored Session (Facade)

After the split, `Session` becomes a thin facade:

```rust
pub struct Session {
    // Identity
    pub id: SessionId,
    pub name: String,
    pub profile_type: ProfileType,

    // Behavior
    pub behavior_profile: BehaviorProfile,
    pub behavior_runtime: BehaviorRuntime,

    // Browser connection
    browser: Browser,
    pub browser_ws_url: String,
    handler_task: Option<JoinHandle<()>>,

    // Page registry
    active_pages: DashSet<TargetId>,

    // Sub-structs (composition)
    workers: WorkerPool,
    health: SessionHealth,
    circuit_breaker: CircuitBreaker,
    overlay: Option<OverlayHandle>,
}
```

**Constructor shrinks from 9 params to ~6** (overlay becomes optional construction inside).

### Delegate Methods Stay on Session

These methods remain on `Session` as thin delegates for backward compatibility:

```rust
impl Session {
    pub fn is_healthy(&self) -> bool { self.health.is_healthy() }
    pub async fn acquire_worker(&self) -> SemaphorePermit<'_> { self.workers.acquire().await }
    pub fn has_worker_capacity(&self) -> bool { self.workers.has_capacity() }
    pub fn is_worker_idle(&self) -> bool { self.workers.is_idle() }
    pub async fn acquire_page(&self) -> Result<TargetId> { /* unchanged */ }
    pub async fn release_page(&self, id: TargetId) { /* unchanged */ }
}
```

This means **zero changes to callers** — retry.rs, execution.rs, browser.rs, main.rs all continue using `session.is_healthy()`, `session.acquire_worker()`, etc.

---

## External Access Audit

The outside world touches Session through a narrow surface. All of these stay unchanged:

| Caller | Accesses | After Refactor |
|---|---|---|
| `retry.rs` | `.id`, `.name`, `.profile_type`, `.behavior_profile`, `.behavior_runtime`, `.browser_ws_url`, `.is_healthy()`, `.max_workers`, `.acquire_worker()` | Unchanged — delegates |
| `execution.rs` | `.id`, `.acquire_page()`, `.release_page()` | Unchanged — delegates |
| `browser.rs` | `.name`, `.profile_type` | Unchanged — direct field |
| `main.rs` | `.is_healthy()`, `.graceful_shutdown()` | Unchanged — delegates |
| `pool.rs` | `.profile_type`, `.is_worker_idle()` | Unchanged — delegates |
| `lifecycle.rs` | All internal machinery | Moves to sub-struct files |

---

## Migration Path

### Phase 1: Extract sub-structs (no behavior change)

1. Create `src/session/workers.rs`, `health.rs`, `circuit_breaker.rs`, `overlay.rs`
2. Move the field groups and their associated methods into each file
3. Add `mod workers; mod health; mod circuit_breaker; mod overlay;` to `mod.rs`
4. Replace the flat fields in `Session` with the sub-struct instances
5. Add delegate methods on `Session` that call through to sub-structs
6. `cargo check` + `cargo test` — all callers should be unchanged

### Phase 2: Update tests

1. Update `Session` constructors in test helpers
2. Sub-struct unit tests can now be written independently
3. Each sub-struct gets its own `#[cfg(test)] mod tests` block

### Phase 3: Documentation

1. Add module-level doc comments to each new file explaining the concern
2. Update `AGENTS.md` to reflect the new module structure

---

## Risk Assessment

| Risk | Severity | Mitigation |
|---|---|---|
| Breaking callers | Low | Delegate methods keep the public API identical |
| Merge conflicts with OOPIF | Low | OOPIF touches `browser_ws_url` which stays on Session directly |
| Test breakage | Medium | Tests construct mock Sessions — update constructors |
| Behavioral change | None | Pure structural refactor, no logic changes |
| Performance | None | Composition adds zero overhead (no boxing, no dynamic dispatch) |

---

## Expected Benefits

1. **Testability** — Each concern (circuit breaker, health, workers) can be unit-tested in isolation
2. **Readability** — New contributors see 4 focused files instead of 1 monolithic struct
3. **Ownership** — Changes to circuit breaker logic go in `circuit_breaker.rs`, not mixed into `lifecycle.rs`
4. **Constructor clarity** — 6 params instead of 9; optional overlay is explicit
5. **Future extensibility** — Adding a new concern (e.g., rate limiting per session) is a new sub-struct, not more fields on Session

---

## See Also

- `src/session/mod.rs` — current Session struct (22 fields)
- `src/session/lifecycle.rs` — methods that will move to sub-struct files
- `src/session/worker.rs` — worker task execution (stays as-is)
- `src/orchestrator/retry.rs` — primary external consumer of Session
