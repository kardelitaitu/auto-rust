# Error Handling & Reliability

Skill for understanding the layered fault tolerance system — structured error types, circuit breakers, retry with backoff, session health monitoring, graceful shutdown, policy enforcement, and task validation.

## Architecture Overview

```
Task Execution
  │
  ├── Pre-flight Validation (src/validation/)
  │   ├── task.rs         — Payload validation per task type
  │   └── task_registry.rs — Task name validation, registry lookup
  │
  ├── Orchestration (src/orchestrator/)
  │   ├── execution.rs  — Group execution, broadcast fan-out, session dispatch
  │   ├── retry.rs      — Task execution with retry + backoff + cancellation
  │   ├── guards.rs     — Global semaphore, session execution guards
  │   └── health.rs     — Session health marking, duration formatting
  │
  ├── Runtime (src/runtime/)
  │   └── shutdown.rs   — Graceful shutdown, Ctrl+C handling, broadcast channel
  │
  ├── Error Types (src/error.rs + src/result/)
  │   ├── error.rs          — OrchestratorError, BrowserError, TaskError, etc.
  │   ├── result/errors.rs  — TaskErrorKind, ErrorPattern, classify_error_pattern
  │   └── result/types.rs   — TaskResult, TaskStatus, builder methods
  │
  ├── Policy Enforcement (src/task/policy.rs)
  │   ├── TaskPolicy        — max_duration_ms + 12 permission flags
  │   ├── TaskPermissions   — Granular capability control
  │   └── get_policy()      — Policy registry lookup
  │
  └── Health Monitoring (src/health_logger.rs)
      └── HealthLogger      — Periodic memory + task success rate logging
```

## File Map

### Structured Error Types

| File | Purpose |
|---|---|
| `src/error.rs` | `OrchestratorError` (6 domain variants: Browser, Session, Task, Config, Network, Io), `BrowserError`, `SessionError`, `TaskError`, `ConfigError`, `NetworkError` — all `#[non_exhaustive]`, `thiserror`-based |
| `src/result/errors.rs` | `TaskErrorKind` (7 variants: Timeout, Validation, Navigation, Session, Browser, ExternalService, Unknown), `ErrorPattern` (14 patterns), `classify_error_pattern()` — shared pattern matching for both transient/permanent classification and kind classification |
| `src/result/types.rs` | `TaskResult` (status, attempt, max_retries, last_error, error_kind, duration_ms, metadata), `TaskStatus` (Success, Failed(String), Timeout, Cancelled), builder methods |

### Orchestration & Fault Tolerance

| File | Purpose |
|---|---|
| `src/orchestrator/execution.rs` | Group execution with timeout, session dispatch, broadcast fan-out across all sessions, stagger delay, partial failure tolerance |
| `src/orchestrator/retry.rs` | `execute_task_with_retry()` — worker acquisition, page acquisition, timeout enforcement, retry loop with exponential backoff, session health marking |
| `src/orchestrator/guards.rs` | `GlobalExecutionSlot` (semaphore-based concurrency), `SessionExecutionGuard` (per-session state tracking), `acquire_global_execution_slot()` |
| `src/orchestrator/health.rs` | `should_mark_session_unhealthy()`, `format_duration()`, `broadcast_execution_count()` |

### Graceful Shutdown

| File | Purpose |
|---|---|
| `src/runtime/shutdown.rs` | `ShutdownManager` — broadcast channel, `request_shutdown()`, `spawn_ctrl_c_listener()`, `wait()`, cloneable across components |

### Health Monitoring

| File | Purpose |
|---|---|
| `src/health_logger.rs` | `HealthLogger` — background task logging memory usage, task success rate, native lock metrics at configurable interval |

### Policy & Validation

| File | Purpose |
|---|---|
| `src/task/policy.rs` | `TaskPolicy` (max_duration_ms + 12 permissions), `TaskPermissions`, `SessionData`, `BrowserData`, `get_policy()`, per-task static policy definitions |
| `src/validation/task.rs` | `TaskPayload::validate()` — task-specific payload validation, `resolve_pageview_target()` |
| `src/validation/task_registry.rs` | `validate_task()`, `validate_task_groups()`, `validate_task_groups_strict()`, `is_known_task()` |

## Key Concepts

### 1. Error Classification System

The error system has two layers that share a common pattern-matching engine:

**Layer 1: `ErrorPattern`** (in `result/errors.rs`) — Classifies any error string into 14 patterns:
- **Permanent** (don't retry): `NotFound`, `PermissionDenied`, `TargetTerminated`
- **Transient** (safe to retry): `Timeout`, `Connection`, `Temporary`, `Network`, `Cancelled`, `Disconnected`, `RateLimited`
- **TaskErrorKind-specific**: `Validation`, `Navigation`, `SessionChannel`, `Unknown`

**Layer 2: `TaskErrorKind` & `OrchestratorError`** — High-level typed errors used throughout:
- `TaskErrorKind::classify(msg)` — maps an error string to a `TaskErrorKind` using `classify_error_pattern()`
- `TaskErrorKind::is_retryable()` — `Validation` is not retryable; all others are
- `OrchestratorError` — 6 domain variants wrapping domain-specific error types

**Pattern matching priority** (ordered in `classify_error_pattern`):
1. "node is disconnected" checked before standalone "disconnected"
2. "timeout" checked before "invalid"
3. "not found" checked before "invalid"
4. "overloaded" checked before "temporary"
5. "connection" patterns checked before generic "session"
6. All checks are case-insensitive via `to_lowercase()`

### 2. Task Execution Pipeline (with all fault tolerance layers)

```
execute_task_on_session()
  │
  ├── Stagger delay (SESSION_STAGGER_DELAY_MS env var or config)
  ├── Acquire global execution slot (semaphore with cancellation):
  │   - tokio::select! between semaphore and cancellation token
  │   - Returns TaskResult::cancelled() if cancelled while waiting
  │
  ├── execute_task_with_retry():
  │   ├── Validate policy before use
  │   ├── Validate task payload (validate_task())
  │   ├── Check session health (is_healthy())
  │   ├── Check session idle (is_idle())
  │   ├── Create SessionExecutionGuard (sets Busy, resets on Drop)
  │   ├── Acquire worker permit (with cancellation)
  │   ├── Acquire page from session
  │   ├── Create LogContext (session_id, profile, task name)
  │   │
  │   └── Retry loop (max_retries = 0 by default — no retries):
  │       ├── Check cancellation before each attempt
  │       ├── Create TaskContext with policy and cancel token
  │       ├── Execute task with timeout (policy.max_duration_ms):
  │       │   └── timeout is enforced via tokio::time::timeout
  │       │       ├── Ok(Ok(result)) if success: mark healthy, return success
  │       │       ├── Ok(Ok(failure)): classify error kind, record failure
  │       │       ├── Ok(Err(e)): classify error kind
  │       │       └── Err(_) timeout: enforce policy timeout, kill task
  │       ├── Check retryability (is_retryable() on error kind)
  │       ├── Exponential backoff: initial_delay * factor^attempt + jitter
  │       │   - RetryPolicy: max_retries=0, initial_delay=config, max_delay=30s, factor=2.0, jitter=0.3
  │       └── Cancellation checked during backoff too
  │
  ├── Cleanup:
  │   ├── Release page back to session
  │   ├── Drop worker permit
  │   ├── Increment failure count (if not cancelled)
  │   ├── Evaluate session health (should_mark_session_unhealthy)
  │   │   - Cancelled tasks never mark unhealthy
  │   │   - Validation/Unknown errors never mark unhealthy
  │   │   - Timeout/Navigation/Session/Browser/ExternalService DO mark unhealthy
  │   └── Return TaskResult with attempt info
  │
  ├── Group-level timeout (config.orchestrator.group_timeout_ms)
  │   └── Cancels all outstanding tasks when group timeout fires
  │
  └── Partial failure tolerance:
      - If at least one session succeeds, group returns Ok
      - If all sessions fail, group returns Err
```

### 3. Global Concurrency Guard

```rust
// GlobalExecutionSlot — acquired before any task execution
//
// Architecture:
//   Atomic counter (active_count) + Tokio Semaphore
//
// acquire_global_execution_slot(task_name, session_id, queue_start, ...):
//   1. tokio::select! between semaphore.acquire_owned() and cancel_token
//   2. If cancelled while queued → TaskResult::cancelled()
//   3. On acquire → GlobalExecutionSlot increments active counter
//   4. On Drop → decrements active counter automatically

// SessionExecutionGuard — per-session state tracking
//   1. On creation → sets session state to Busy
//   2. On mark_idle()/mark_failed() → sets appropriate state, marks inactive
//   3. On Drop (if still active) → resets to Idle (panic safety)
```

### 4. Transient Error Classification (CDP Retry)

Used in `page_nav.rs` for CDP operations with `with_retry()`:

```
is_transient_error(err) → bool:
  TRANSIENT (retry):
    ├── Timeout ✓
    ├── Connection ✓
    ├── Temporary ✓
    ├── Network ✓
    ├── Cancelled ✓
    ├── Disconnected ✓  (standalone, not "node is disconnected")
    └── RateLimited ✓

  PERMANENT (fail fast):
    ├── NotFound ✗
    ├── PermissionDenied ✗
    └── TargetTerminated ✗ ("node is disconnected", "target closed")
```

Used only in `page_nav.rs` for CDP eval retry. Other CDP calls fail immediately.

### 5. Graceful Shutdown

```rust
// ShutdownManager (src/runtime/shutdown.rs)
//
// Architecture:
//   broadcast::Sender<()> wrapped in Arc — cloneable across all runtime components
//
// Usage flow:
//   let manager = ShutdownManager::new();
//
//   // 1. Spawn Ctrl+C listener (on startup)
//   let _handle = manager.spawn_ctrl_c_listener();
//
//   // 2. Subscribe for shutdown notifications (each component)
//   let mut rx = manager.subscribe();
//
//   // 3. Check in loops:
//   tokio::select! {
//       _ = rx.recv() => { /* shutdown */ }
//       _ = task_execution() => { /* continue */ }
//   }
//
//   // 4. Trigger shutdown:
//   manager.request_shutdown();  // returns bool (false if no subscribers)

// Key behaviors:
//   - Late subscribers don't receive prior signals (broadcast channel)
//   - Ctrl+C handler runs in a separate tokio task
//   - Used in execute_task_groups_with_shutdown() to stop between/before groups
//   - Group cancellation uses CancellationToken child tokens
```

### 6. Policy Enforcement

```rust
// TaskPolicy (src/task/policy.rs)
//
// Two parts:
//   1. max_duration_ms — Hard limit (type-guaranteed non-zero by DurationMs)
//   2. 12 permission flags — What the task is allowed to do

// Permission flags:
//   allow_screenshot         → implies allow_write_data
//   allow_export_cookies     → implied by allow_export_session
//   allow_import_cookies     → implied by allow_import_session
//   allow_export_session
//   allow_import_session
//   allow_session_clipboard
//   allow_read_data
//   allow_write_data
//   allow_http_requests
//   allow_dom_inspection
//   allow_browser_export
//   allow_browser_import

// Per-task static policies:
//   DEFAULT_TASK_POLICY (180s timeout, all permissions off)
//   COOKIEBOT_POLICY (30s, allow_export_cookies + allow_screenshot)
//   PAGEVIEW_POLICY (120s, all defaults)
//   TWITTERACTIVITY_POLICY (configurable, 6 permissions)
//   TWITTER_BASE_POLICY (45s, 4 permissions — parent for all Twitter tasks)
//   Twitter tasks: TWITTERDIVE, TWITTERFOLLOW, TWITTERINTENT, TWITTERLIKE,
//                  TWITTERQUOTE, TWITTERREPLY, TWITTERRETWEET, TWITTERTEST
//   Demo tasks: DEMO_KEYBOARD, DEMO_MOUSE, DEMO_QA, TASK_EXAMPLE
//   TEST_LLM_REPLY_POLICY (3min, 4 permissions)
//
// Policy lookup flow:
//   get_policy(name) → normalize → registry.lookup() → match_policy_by_name() → &'static TaskPolicy
//   Falls back to DEFAULT_TASK_POLICY if task not found
```

### 7. Task Validation

Two-stage validation:

**Stage 1: Task name validation** (`src/validation/task_registry.rs`)
```
validate_task("twitterfollow")
  ├── normalize_task_name() (removes .js suffix)
  ├── registry.lookup() (checks built-in + configured tasks)
  ├── Returns TaskValidationResult { is_known, source, policy_name, warnings }
  └── validate_task_groups_strict() — fails fast on any warning
```

**Stage 2: Payload validation** (`src/validation/task.rs`)
```
TaskPayload::new(name, payload).validate()
  ├── Dispatches to task-specific validation:
  │   ├── "pageview" → requires url or value
  │   ├── "twitterfollow" → requires username, url, or value
  │   ├── "twitterquote" → requires url/value, enforces 280-char quote_text
  │   ├── "twitterreply" → requires url or value
  │   ├── "twitteractivity" → requires object
  │   ├── "cookiebot" → requires object
  │   └── all others → requires object
  └── Unknown tasks → logs info, passes (no validation)
```

Validation errors use `OrchestratorError::Task(TaskError::ValidationFailed { ... })`.

### 8. Session Health & Failure Tracking

```
Session health lifecycle:
  ┌────────────┐
  │   Healthy   │
  └─────┬───────┘
        │ task succeeds → mark_healthy()
        │
        ├── task fails (non-cancelled, non-validation, non-unknown error kind)
        │   → mark_unhealthy() + set_state(Failed)
        │
        └── task cancelled → NO health change
            validation error → NO health change
            unknown error → NO health change

Session state transitions:
  Idle ←→ Busy (via SessionExecutionGuard)
  Idle → Failed (via mark_unhealthy)
  Failed persists until session reconnection/recovery
```

`should_mark_session_unhealthy(kind, was_cancelled)`:
- Cancelled tasks: **never** mark unhealthy
- Validation/Unknown errors: **never** mark unhealthy
- Timeout/Navigation/Session/Browser/ExternalService: mark unhealthy **only if not cancelled**

### 9. Health Logger

Background task that runs at configurable interval (default 60s):

```
HealthLogger::start()
  ┌─────────────────────────────────────────────┐
  │  Every interval (default 60s):              │
  │  ├── System memory (used / total / %)       │
  │  ├── Task stats (active, total, success %)  │
  │  ├── Native lock stats (acquisitions,       │
  │  │   contentions, avg/max wait/hold ms)     │
  │  ├── Verbose mode: failures, timeouts,      │
  │  │   avg duration, failure breakdown        │
  │  └── Warning if memory > 86% threshold      │
  └─────────────────────────────────────────────┘
```

Stops via `shutdown.notify_one()` or when the background task handle is dropped.

### 10. TaskResult Builder Methods

```rust
// TaskResult provides builder methods for constructing results:
TaskResult::success(duration_ms)
TaskResult::failure(duration_ms, error, error_kind)  // Timeout kind → TaskStatus::Timeout
TaskResult::cancelled(duration_ms, error, error_kind) // Always TaskStatus::Cancelled

// Builder methods (chaining):
result.with_retry(attempt, max_retries, last_error)
result.with_attempt(attempt, max_retries)
result.with_error_kind(kind)

// Status checks:
result.is_success() → bool
```

## Common Modification Patterns

### Adding a new error variant

1. Add variant to the appropriate enum in `src/error.rs` (or `src/result/errors.rs` for `TaskErrorKind`)
2. Add `#[error("...")]` display message
3. If it's a permanent error, add pattern match in `classify_error_pattern()` (before transient patterns)
4. If retryable behavior matters, update `is_retryable()` on `TaskErrorKind`
5. Add tests for the new variant

### Adding a new task policy

1. Add `pub static MY_TASK_POLICY` in `src/task/policy.rs`
2. Add match arm in `match_policy_by_name()`
3. Add validation for the task's `max_duration_ms` in existing tests
4. Register the task in the task registry (`src/task/registry.rs`)

### Modifying retry behavior

1. Change `RetryPolicy` parameters in `execute_task_with_retry()` in `retry.rs`:
   - `max_retries` (currently 0 — no retries!)
   - `initial_delay`, `max_delay`, `factor`, `jitter`
2. Update `is_retryable()` on `TaskErrorKind` if error classification needs changes
3. Run retry tests: `cargo test --lib retry`

### Adding a validation check

1. Add match arm in `TaskPayload::validate()` in `src/validation/task.rs`
2. Add validation info in `get_task_validation_info()`
3. Add tests for valid/invalid payloads

### Changing session health marking

1. Modify `should_mark_session_unhealthy()` in `src/orchestrator/health.rs`
2. Update tests in `health.rs` and `retry.rs`

## Test Locations

| Test | Location | Command |
|---|---|---|
| Error type formatting & conversions | `src/error.rs` | `cargo test --lib error` |
| Error pattern classification | `src/result/errors.rs` | `cargo test --lib errors` |
| TaskResult builder & status | `src/result/types.rs` | `cargo test --lib types` |
| Retry logic & cancellation | `src/orchestrator/retry.rs` | `cargo test --lib retry` |
| Concurrency guards | `src/orchestrator/guards.rs` | `cargo test --lib guards` |
| Health/duration formatting | `src/orchestrator/health.rs` | `cargo test --lib health` |
| Group execution | `src/orchestrator/execution.rs` | `cargo test --lib execution` |
| Shutdown manager | `src/runtime/shutdown.rs` | `cargo test --lib shutdown` |
| Health logger | `src/health_logger.rs` | `cargo test --lib health_logger` |
| Task policy & permissions | `src/task/policy.rs` | `cargo test --lib policy` |
| Task payload validation | `src/validation/task.rs` | `cargo test --lib task` |
| Task name/registry validation | `src/validation/task_registry.rs` | `cargo test --lib task_registry` |

## Pitfalls

1. **`max_retries = 0`** — The retry loop currently does NOT retry. `max_retries` is set to 0 in `execute_task_with_retry()`. If you want retries, change this value.
2. **`is_retryable()` includes `Unknown`** — Unknown errors default to retryable, which could cause infinite retry loops for unclassified permanent errors.
3. **Cancellation is not retried** — If a task is cancelled, `was_cancelled = true` prevents retry and prevents session health marking.
4. **Validation errors skip session health marking** — Validation failures never mark sessions unhealthy, regardless of cancellation status.
5. **Late shutdown subscribers miss signals** — `broadcast::channel` means subscribers created after `request_shutdown()` won't receive the signal.
6. **`SessionExecutionGuard` is panic-safe** — The guard's `Drop` impl resets session to Idle. If you change session state manually, ensure the guard is dropped or marked appropriately.
7. **Policy lookup uses the registry** — `get_policy()` calls `registry.lookup()` which normalizes task names. If you add a task without registering it, the policy falls back to `DEFAULT_TASK_POLICY`.
8. **`effective_permissions()` implies permissions transitively** — `allow_screenshot` implies `allow_write_data`, `allow_export_session` implies `allow_export_cookies`. Don't check raw permissions; always use `effective_permissions()`.
9. **`classify_error_pattern` ordering matters** — More specific patterns must come before general ones. "node is disconnected" before "disconnected". "element not found" before generic "not found". Always add new patterns in the correct priority position.
