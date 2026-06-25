# Session Lifecycle

Comprehensive guide to the browser session lifecycle — pool management, factory construction, worker allocation, page lifecycle, circuit breaker, health monitoring, graceful shutdown, and browser connectors.

---

## Architecture

```
Config → SessionPoolManager → ConnectorRegistry → BrowserCapabilities
                                  ↓
                          SessionFactory
                                  ↓
                           Session (browser instance)
                                  ↓
                    ┌─────────────┼─────────────┐
                    ↓             ↓             ↓
             WorkerPermit    acquire_page    acquire_page_at
             (semaphore)     (new page)      (page + URL)
                    ↓             ↓             ↓
               Page interaction → release_page → graceful_shutdown
```

The full lifecycle:
1. `SessionPoolManager` uses a `ConnectorRegistry` to discover browsers from 3 sources (configured profiles, RoxyBrowser cloud, local port scanning)
2. `SessionFactory` connects to discovered browser WebSocket endpoints and creates `Session` instances
3. `Session` wraps a chromiumoxide `Browser` instance with worker semaphore, circuit breaker, page registry, cursor overlay, and health monitoring
4. Workers acquire `WorkerPermit` (semaphore slot), then acquire pages for task execution
5. Pages are registered in a `DashSet`, closed on release, cleaned up on shutdown

---

## File Map

| File | Purpose |
|---|---|
| `src/session/mod.rs` | `Session` struct definition, `new()` constructor, handler task, overlay task, circuit breaker initialization — plus inline tests for state, circuit breaker, health transitions |
| `src/session/pool.rs` | `SessionPoolManager` — discovery with retry, parallel connection, browser filtering |
| `src/session/factory.rs` | `SessionFactory` — connect to browser WebSocket, create sessions; `SessionFactoryBuilder` |
| `src/session/worker.rs` | Worker allocation (`acquire_worker`, `WorkerPermit`), page lifecycle (`acquire_page`, `acquire_page_at`, `release_page`), circuit breaker internals (`cb_check`, `cb_record_success`, `cb_record_failure`), page cleanup (`cleanup_managed_pages`), graceful shutdown (`graceful_shutdown`) |
| `src/session/state.rs` | `SessionState` enum (Idle/Busy/Failed), `unix_timestamp_secs()` helper, `is_circuit_breaker_open_pure()` |
| `src/session/permits.rs` | `WorkerPermit` struct — RAII wrapper around `SemaphorePermit`, auto-decrements active_workers on `Drop` |
| `src/session/cleanup.rs` | `ManagedTabCleanup` trait, `ShutdownSession` trait (test-only), `cleanup_managed_tabs()` — mockable cleanup orchestration |
| `src/session/connector.rs` | `BrowserConnector` trait, `BrowserCapabilities`, `BrowserSource` enum, `ConfiguredProfileConnector`, `RoxyBrowserConnector`, `LocalBrowserConnector`, `ConnectorRegistry` |
| `src/session/duration.rs` | `DurationMs` — `NonZeroU64` wrapper for TOML/JSON config, `duration_with_variance()`, `duration_ms()` |
| `src/session/lifecycle.rs` | Simple getter/setter methods: `register_page`, `unregister_page`, `state()`, `set_state()`, `is_healthy()`, `mark_healthy/unhealthy`, `increment_failure`, circuit breaker accessors |

---

## Session Struct

The `Session` struct is the central type. Key fields:

| Field | Type | Description |
|---|---|---|
| `id` | `String` | Unique session identifier |
| `name` | `String` | Human-readable name (e.g., "Brave Local") |
| `profile_type` | `String` | Browser type ("chrome", "brave", "roxybrowser") |
| `behavior_profile` | `BrowserProfile` | Randomized human-like interaction profile |
| `behavior_runtime` | `ProfileRuntime` | Session-stable derived behavior snapshot |
| `browser` | `Browser` | Chromium Oxide browser instance |
| `handler_task` | `Option<JoinHandle<()>>` | Background event handler (5s poll loop) |
| `cursor_overlay_ms` | `u64` | Cursor overlay sync interval (0 = disabled) |
| `worker_semaphore` | `Arc<Semaphore>` | Max concurrent workers per session |
| `active_workers` | `AtomicUsize` | Current count of active workers |
| `failure_count` | `AtomicUsize` | Consecutive failures for health monitoring |
| `is_healthy` | `AtomicBool` | Health status flag |
| `state` | `Mutex<SessionState>` | Current operational state (Idle/Busy/Failed) |
| `active_pages` | `DashSet<TargetId>` | Registered page IDs |
| `cb_failure_count` | `Arc<AtomicUsize>` | Circuit breaker failure counter |
| `cb_failure_threshold` | `usize` | Failures before circuit opens (default: 5) |
| `cb_timeout_secs` | `u64` | Time before circuit auto-closes (default: 30s) |
| `cb_last_failure_time` | `Arc<AtomicUsize>` | Last failure Unix timestamp |

---

## Session Construction

`Session::new()`:
1. Spawns a **handler task** — polls browser event stream every 5s. Non-fatal timeouts on idle sessions are suppressed.
2. Creates randomized **behavior profile** (`randomize_profile(&random_preset())`) and runtime snapshot
3. Optionally spawns **cursor overlay background task** if `cursor_overlay_ms > 0`
4. Initializes **circuit breaker** from config (default: threshold=5, timeout=30s)
5. Sets initial state to `Idle`, healthy to `true`

---

## Worker Lifecycle

### Acquire
```rust
session.acquire_worker(timeout_ms: u64) -> Option<WorkerPermit<'_>>
```
1. Fast-fail if circuit breaker is open → returns `None`
2. Waits on semaphore with timeout
3. On success: increments `active_workers`, returns `WorkerPermit`
4. On timeout: logs warning, returns `None`

### WorkerPermit (RAII)
- Wraps `SemaphorePermit<'a>` + reference to `active_workers`
- On `Drop`: auto-decrements `active_workers` via `fetch_sub(1, Ordering::SeqCst)`
- Release is automatic — just let the permit go out of scope

---

## Page Lifecycle

### acquire_page
1. `cb_check()` — circuit breaker check; bails with error if open
2. `browser.new_page("about:blank")` — creates new browser tab
3. On success: `cb_record_success()` (resets failure counter to 0)
4. On failure: `cb_record_failure()` (increments, logs `[session_id] Circuit breaker failure count: X/Y`)
5. Wraps page in `Arc`, registers in `active_pages` DashSet, binds cursor overlay, returns `Arc<Page>`

### acquire_page_at(url)
Same as `acquire_page` but opens page directly on the target URL instead of `about:blank`.

### release_page(page)
1. Clears cursor overlay for this page
2. Unbinds overlay state
3. Calls `page.close()` via CDP
4. Unregisters from `active_pages`
5. Errors during close are logged as warnings (non-fatal)

---

## Circuit Breaker

### State Determination (pure function)
```rust
fn is_circuit_breaker_open_pure(
    failure_count, failure_threshold,
    last_failure_time, current_time, timeout_secs
) -> bool
```
Logic: `failure_count >= failure_threshold && current_time.saturating_sub(last_failure_time) < timeout_secs`

- **Closed**: below threshold, or enough time has passed since last failure
- **Open**: at/above threshold AND within timeout window
- Zero threshold (`failure_threshold == 0`) → circuit NEVER opens
- Uses `saturating_sub` for safe time arithmetic (handles wraparound)

### cb_check
Called before page acquisition. Returns `current_time` (as Unix seconds) if closed, or bails with error message listing failure count and timeout.

### cb_record_success
Resets `cb_failure_count` to 0.

### cb_record_failure(current_time)
Atomically increments `cb_failure_count` + 1, stores `current_time` as `cb_last_failure_time`, logs `[session_id] Circuit breaker failure count: X/Y` at warn level.

### Accessors (in lifecycle.rs)
| Method | Purpose |
|---|---|
| `get_circuit_breaker_failure_count()` | Current failure count |
| `get_circuit_breaker_threshold()` | Failure threshold |
| `get_circuit_breaker_timeout_secs()` | Timeout in seconds |
| `is_circuit_breaker_open()` | Check if circuit is open (uses real SystemTime) |
| `reset_circuit_breaker()` | Reset counters and mark healthy |
| `set_circuit_breaker_failure_count(n)` | Set failure count (testing only) |
| `set_circuit_breaker_last_failure_time(t)` | Set last failure time (testing only) |

---

## Session State Machine

| State | Meaning | Transitions |
|---|---|---|
| `Idle` | Available for task assignment | → `Busy` (on acquire) |
| `Busy` | Currently executing a task | → `Idle` (on release) → `Failed` (on error) |
| `Failed` | Unhealthy, not accepting tasks | → `Idle` (on reset/recovery) |

The state is stored in `parking_lot::Mutex<SessionState>` — a synchronous mutex (not async!). Never hold across `.await` points — only used for quick reads/writes.

---

## Health Monitoring

| Method | Behavior |
|---|---|
| `is_healthy()` | Reads `AtomicBool` |
| `mark_healthy()` | Sets `is_healthy = true` |
| `mark_unhealthy()` | Sets `is_healthy = false` |
| `increment_failure()` | `failure_count.fetch_add(1)` |
| `get_failure_count()` | Reads `failure_count` |

Health is marked unhealthy when:
- Circuit breaker opens (`cb_check` returns error)
- The orchestrator detects task failures

---

## Graceful Shutdown

`graceful_shutdown()` performs in order:
1. **Mark state as `Failed`** — stops new task assignment
2. **Close all pages** — iterates all browser pages, clears overlay, unbinds, calls `page.close()`
3. **Close browser** — `browser.close()` with 10s timeout
4. **Abort overlay task** — if cursor overlay is active
5. **Abort handler task** — background event polling

---

## Browser Connectors

### Connector Registry
`ConnectorRegistry` holds 3 connectors by default:
1. `ConfiguredProfileConnector` — from `config.browser.profiles` list
2. `RoxyBrowserConnector` — cloud browser API (disabled by default)
3. `LocalBrowserConnector` — port scanning (disabled by default, `is_available` returns false)

### ConfiguredProfileConnector
- **Available when**: `config.browser.profiles` is non-empty
- **Discovery**: Returns `BrowserCapabilities` for each profile (name, type, ws_endpoint)

### RoxyBrowserConnector
- **Available when**: `roxybrowser.enabled == true` AND `api_url` is non-empty
- **Discovery**: Calls `GET browser/connection_info` with API key, parses response
- **Response format**: `{ code: 0, data: [{ ws, http, windowName, name }] }`
- Handles both `ws` and `http` fields (converts `http` → `ws` for CDP)

### LocalBrowserConnector
- **Default port ranges**: Brave (9001-9050), Chrome (9222-9230)
- **Discovery**: Concurrent port scanning via `check_port()` — calls `http://127.0.0.1:{port}/json/version` and extracts `webSocketDebuggerUrl`
- **Configurable via env vars**: `BRAVE_PORT_START`, `BRAVE_PORT_END`, `CHROME_PORT_START`, `CHROME_PORT_END`
- `parse_port_value(val: &str) -> Option<u16>` — pure parsing function for testability
- Currently `is_available` always returns `false` (port scanning happens only on explicit calls)

### BrowserCapabilities Struct
| Field | Type | Description |
|---|---|---|
| `id` | `String` | Unique identifier (e.g., "config-profile-name", "brave-9222", "roxy-window") |
| `name` | `String` | Human-readable name |
| `browser_type` | `String` | Type string (e.g., "brave", "localChrome", "roxybrowser") |
| `ws_url` | `String` | WebSocket debugger URL for CDP |
| `source` | `BrowserSource` | One of `Configured`, `RoxyBrowser`, `Local` |

---

## Session Pool Manager

`SessionPoolManager` coordinates discovery, filtering, and connection.

### Discovery Flow
```
1. For each available connector → connector.discover(config)
2. Collect all BrowserCapabilities from all connectors
3. Optionally filter by browser name/type

Filter matching: normalizes filters (lowercase, remove non-alphanumeric)
  and checks concatenated "name type id" for substring match.
```

### Retry Logic
- `max_retries` from config (`browser.max_discovery_retries`, default 3)
- Uses `ExponentialBackoff`: base=1s, max=10s, jitter=0.2
- `discover_and_connect()`: retries discovery + connection until at least 1 session
- `discover_with_filters()`: retries filtered discovery, returns specific error for no-match

### Browser Filtering
```rust
session_pool.discover_with_filters(&config, &["brave", "chrome"]).await?
```
- Case-insensitive, special-character-normalized matching
- Checks against concatenated "name type id" string
- If filters specified and nothing matches: returns helpful error with filter names

---

## Session Factory

`SessionFactory` creates sessions from browser capabilities.

### Construction
| Parameter | Default | Min |
|---|---|---|
| `connection_timeout_ms` | 30000 | 5000 (clamped) |
| `max_workers` | 3 | — |
| `cursor_overlay_ms` | 0 | — |

### create_session(capability)
1. Validates ws_url is non-empty
2. `chromiumoxide::Browser::connect()` with timeout
3. On success: creates `Session::new()` with capability metadata
4. On failure or timeout: returns `BrowserError::ConnectionFailed`

### create_sessions_parallel(capabilities)
- Concurrent connection with `buffer_unordered(10)`
- Skips failed connections (returns `Vec<Session>` of successes only)

### SessionFactoryBuilder
Chaining builder for custom session factory construction:
```rust
SessionFactoryBuilder::new()
    .connection_timeout_ms(60000)
    .max_workers(10)
    .cursor_overlay_ms(200)
    .build()
```

---

## DurationMs Type

`DurationMs` is a `NonZeroU64` wrapper for millisecond durations.

| Method | Description |
|---|---|
| `new(value)` | `Option<DurationMs>` — returns `None` for 0 |
| `new_const(value)` | Panics on 0 (const-compatible) |
| `get()` | Raw millisecond value |
| `as_secs()` | Integer division by 1000 |
| `with_variance(pct)` | Randomized value within ±pct% |
| `checked_add(rhs)` | Saturating add, `None` on overflow |
| `checked_sub(rhs)` | Saturating sub, `None` on zero/negative result |
| `From<DurationMs> for Duration` | Converts to `std::time::Duration` |

### Pure Duration Functions
- `duration_with_variance(base_ms, variance_pct)` — uniform random in `[base*(1-pct), base*(1+pct)]`
- `duration_ms(Duration)` — saturating cast `u128` → `u64`

---

## Cleanup Traits

### ManagedTabCleanup (async trait)
```rust
#[async_trait(?Send)]
pub trait ManagedTabCleanup {
    fn session_id(&self) -> &str;
    async fn cleanup_managed_pages(&self) -> anyhow::Result<usize>;
}
```
- Implemented by `Session` — delegates to `Session::cleanup_managed_pages()`
- `cleanup_managed_tabs(sessions)` — iterates all sessions, accumulates closed count
- Continues on per-session errors, logs warnings

### ShutdownSession (test-only)
```rust
#[async_trait(?Send)]
pub trait ShutdownSession {
    async fn shutdown(&mut self) -> anyhow::Result<()>;
}
```
- Implemented by `Session` — delegates to `Session::graceful_shutdown()`
- `shutdown_sessions(sessions)` — calls shutdown on all, continues on errors

---

## Adding or Modifying

### Adding a new connector
1. Create a struct implementing `BrowserConnector` trait (in `connector.rs`):
   - `is_available(&self, config) -> bool`
   - `async discover(config) -> Result<Vec<BrowserCapabilities>>`
   - `async connect(capability, config) -> Result<Session>`
2. Add to `ConnectorRegistry::standard()` or `ConnectorRegistry::empty()`
3. Add to `SessionPoolManager::from_config()` if needed

### Changing circuit breaker parameters
- Threshold: modify `CircuitBreakerConfig.failure_threshold` in config
- Timeout: modify `CircuitBreakerConfig.half_open_time_ms` in config
- Defaults (5 failures, 30s timeout) are applied in `Session::new()` when config is `None`

### Adding session state fields
1. Add field to `Session` struct in `mod.rs`
2. Initialize in `Session::new()`
3. Add getter/setter in `lifecycle.rs` if externally needed
4. Handle in `graceful_shutdown()` in `worker.rs` if resource cleanup needed
5. Add tests in `mod.rs` inline test module

### Changing worker concurrency
- Adjust `max_workers_per_session` in browser config
- The semaphore is initialized with `Semaphore::new(max_workers)` per session

---

## Testing

| Test Location | Command |
|---|---|
| State & circuit breaker unit tests | `cargo test --lib session::mod::tests` |
| Duration type tests | `cargo test --lib session::duration::tests` |
| Factory tests | `cargo test --lib session::factory::tests` |
| Pool tests | `cargo test --lib session::pool::tests` |
| Connector tests | `cargo test --lib session::connector::tests` |
| Cleanup trait tests | `cargo test --lib session::cleanup::tests` |
| All session tests | `cargo test --lib session::` |

---

## Pitfalls

| # | Pitfall | Explanation |
|---|---|---|
| 1 | **State Mutex across `.await`** | `Session.state` uses `parking_lot::Mutex` (sync). Never hold the lock across `.await` — it's for quick read/write only. |
| 2 | **Circuit breaker persists across tasks** | `cb_failure_count` is per-session, not per-task. A burst of failures on one task can prevent other tasks from using the same session. |
| 3 | **Zero threshold = never opens** | `is_circuit_breaker_open_pure` returns `false` when `failure_threshold == 0`. This means a threshold of 0 disables the circuit breaker entirely, it does NOT mean "open immediately." |
| 4 | **Graceful shutdown order matters** | Browser must be closed AFTER pages — closing the browser first leaves orphaned page close attempts. Worker shutdown checks shutdown order in tests. |
| 5 | **Handler task is polled not pushed** | The handler task polls `handler.next()` with a 5s timeout. Non-fatal timeout logs are suppressed during idle. |
| 6 | **LocalBrowserConnector always disabled** | `is_available()` returns `false` for local discovery — it's only triggered by explicit discovery calls. Don't rely on auto-discovery. |
| 7 | **Permit drop is silent** | `WorkerPermit::Drop` silently decrements `active_workers`. If a task panics, the permit is dropped (via unwinding), but the decrement still runs. |
| 8 | **duration_with_variance variance clamped** | `variance_pct` is clamped to `min(100)`. 200% variance behaves the same as 100%. |
| 9 | **DurationMs checked_sub returns None at zero** | Unlike `checked_add` (which allows zero), `checked_sub` returns `None` when the result would be zero because `DurationMs` is `NonZeroU64`. |
| 10 | **Browser close timeout is hardcoded** | The 10s timeout in `graceful_shutdown()` is NOT configurable — it's hardcoded in `worker.rs`. |
