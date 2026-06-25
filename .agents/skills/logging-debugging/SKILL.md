# Logging & Debugging System

Teaches agents about the multi-layered logging, health monitoring, metrics collection, and debug/tracing infrastructure for DSL tasks.

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                     Logging & Debugging Stack                    │
├──────────────────────────────────────────────────────────────────┤
│ Layer 1: FileLogger (src/logger.rs)                              │
│   - Writes to both stdout and log file (truncated each run)      │
│   - Thread-local LogContext: [session_id][profile][task]         │
│   - Scoped context guard for automatic restore                   │
│   - Filters chromiumoxide noise                                  │
├──────────────────────────────────────────────────────────────────┤
│ Layer 2: Health Logger (src/health_logger.rs)                    │
│   - Background tokio task, configurable interval (default 60s)   │
│   - Logs memory, success rate, native lock metrics               │
│   - Warning at 86% memory threshold                              │
├──────────────────────────────────────────────────────────────────┤
│ Layer 3: Metrics Collector (src/metrics.rs)                      │
│   - Atomic counters for total/succeeded/failed/timeout/cancelled │
│   - Task & session breakdowns (FxHashMap for performance)        │
│   - Run counters for structured event tracking                   │
│   - Export to run-summary.json on completion                     │
├──────────────────────────────────────────────────────────────────┤
│ Layer 4: DSL Debug (src/task/dsl/debug.rs)                       │
│   - DebugEvent types with 8 event variants                       │
│   - Breakpoint system (by index, type, variable watch)           │
│   - Variable change detection                                    │
├──────────────────────────────────────────────────────────────────┤
│ Layer 5: DSL Profiling (src/task/dsl/profiling.rs)               │
│   - ActionProfiler (aggregate stats per action type)             │
│   - ActionMetrics (per-action timing + outcome)                  │
│   - ExecutionReport (JSON-exportable run summary)                │
└──────────────────────────────────────────────────────────────────┘
```

## File Map

| File | Purpose |
|---|---|
| `src/logger.rs` | `FileLogger` impl, `LogContext` thread-local, scoped guard, chromiumoxide filter |
| `src/health_logger.rs` | Background health monitoring, periodic memory/task logging, native lock metrics |
| `src/metrics.rs` | `MetricsCollector`, atomic counters, breakdowns, run counters, `export_summary_to()` |
| `src/task/dsl/debug.rs` | `DebugEvent`, `DebugEventType` (8 variants), `Breakpoint`, `watch_variable()` |
| `src/task/dsl/profiling.rs` | `ActionProfiler`, `ActionMetrics`, `ExecutionReport` with JSON export |
| `src/orchestrator/health.rs` | `format_duration()`, `should_mark_session_unhealthy()` |
| `src/main.rs` | `--debug` flag, `RUST_LOG` env var, `LevelFilter` setup |

## Log Levels: When to Use Each

| Level | When to Use | Example |
|---|---|---|
| `error!` | Irrecoverable failures, unexpected crashes | Browser disconnect, config parse failure, panic |
| `warn!` | Recoverable issues, degradation, unusual states | Memory threshold exceeded, retry limit approaching |
| `info!` | Normal operation milestones, state transitions | Task started/completed, session connected, action succeeded |
| `debug!` | Detailed execution flow, troubleshooting | Scroll attempts, candidate evaluations, element lookups |

The default level is `Info`. Debug is enabled via:
- `--debug` CLI flag → sets `LevelFilter::Debug`
- `RUST_LOG=debug` env var → overrides the log level (set before the logger is initialized)

In `main.rs`, the log level is set based on the `--debug` flag:
```rust
let log_level = if args.debug {
    LevelFilter::Debug
} else {
    LevelFilter::Info
};
log::set_max_level(log_level);
```

## Context Tag Format

Log messages are automatically prefixed with thread-local context:

```
<timestamp> [session_id][profile_name][task_name] LOG_LEVEL message
```

Example output:
```
14:32:05 [brave-9002][Teen][pageview] INFO Navigating to https://example.com
14:32:06 [brave-9002][Teen][pageview] INFO Page loaded (1.2s)
14:32:07 [brave-9002][Teen] WARN Session health degraded
14:32:08 [brave-9002][Teen] ERROR Browser disconnected unexpectedly
```

### LogContext struct
```rust
pub struct LogContext {
    pub session_id: Option<String>,
    pub profile_name: Option<String>,
    pub task_name: Option<String>,
}
```

Format is `session_id + profile_name + task_name` — missing fields are skipped:
- All 3 present: `[brave-9002][Teen][pageview]`
- Session + task only: `[brave-9002][pageview]`
- Empty: (no prefix)

### Thread-local API

```rust
use crate::logger::{set_log_context, get_log_context, clear_log_context, scoped_log_context};

// Set context for current thread
set_log_context(LogContext {
    session_id: Some("brave-9002".into()),
    profile_name: Some("Teen".into()),
    task_name: Some("pageview".into()),
});

// Scoped context (auto-restores on drop)
let _guard = scoped_log_context(LogContext {
    session_id: Some("inner-session".into()),
    profile_name: None,
    task_name: Some("nativeclick".into()),
});
// Do work... guard drops, previous context restored
```

## FileLogger Detail

### Creation

```rust
let logger = FileLogger::new("log")?;  // creates/truncates "log" file
log::set_boxed_logger(Box::new(logger))?;
log::set_max_level(LevelFilter::Info);
```

- Log file path is `"log"` (in project root)
- Truncated on each run (no append)
- Writes to both stdout (via `print!`) and the log file

### chromiumoxide Filter

All log records from `chromiumoxide` targets are silently dropped:
```rust
if record.target().starts_with("chromiumoxide") {
    return; // suppress
}
```

This prevents CDP WebSocket noise from flooding logs.

## Health Logger Detail

### Configuration

```rust
HealthLoggerConfig {
    interval: Duration::from_secs(60),     // How often to log
    memory_warning_percentage: 86.0,       // Warning threshold
    verbose: false,                        // Include failure breakdown
}
```

### Usage

```rust
let health_config = HealthLoggerConfig::default();
let health_logger = HealthLogger::new(health_config, metrics.clone());
let _health_handle = health_logger.start();

// ... run tasks ...

health_logger.stop();
let _ = _health_handle.await;
```

### What gets logged

Every interval (default 60s):
```
[health] active_tasks=N total_tasks=N success_rate=XX.X% memory=XXX.XMiB/XXXX.XMiB (XX.X%)
[health-native-lock] acquisitions=N contentions=N avg_wait_ms=X.X max_wait_ms=X avg_hold_ms=X.X max_hold_ms=X
```

When verbose=true:
```
[health-detail] failures=N timeouts=N avg_duration_ms=XXX
[health-detail] failure_breakdown={...}
[health] Process memory: XXX.X MiB
```

When memory exceeds threshold:
```
WARN [health] Memory usage XX.X% (XXX.X MiB) exceeds threshold XX.X%
```

### Shutdown

Uses `tokio::sync::Notify` for clean shutdown. Multiple `stop()` calls are safe.

## Metrics Collection Detail

### MetricsCollector

Thread-safe collector using `Arc<AtomicUsize>` for counters and `Arc<RwLock<FxHashMap>>` for breakdowns:

| Counter | Type | Purpose |
|---|---|---|
| `total_tasks` | AtomicUsize | Total tasks started |
| `succeeded` | AtomicUsize | Successful completions |
| `failed` | AtomicUsize | Failed outcomes |
| `timed_out` | AtomicUsize | Timeout outcomes |
| `cancelled` | AtomicUsize | Cancelled outcomes |
| `total_duration_ms` | AtomicUsize | Sum of all durations |
| `active_tasks` | AtomicUsize | Currently running |
| `task_history` | VecDeque<TaskMetrics> | Rolling history (max N) |

Breakdowns use `FxHashMap` (faster than standard `HashMap` for small keys):
- `failure_breakdown`: `FxHashMap<TaskErrorKind, usize>`
- `task_breakdown`: `FxHashMap<Arc<String>, OutcomeBreakdown>`
- `session_breakdown`: `FxHashMap<Arc<String>, OutcomeBreakdown>`
- `run_counters`: `FxHashMap<String, usize>`

Using `Arc<String>` as keys enables O(1) clone (ref-count increment) vs O(n) string copy.

### TaskMetrics

```rust
pub struct TaskMetrics {
    pub task_name: Arc<String>,
    pub status: TaskStatus,        // Success | Failed | Timeout | Cancelled
    pub duration_ms: u64,
    pub session_id: Arc<String>,
    pub attempt: u32,
    pub error_kind: Option<TaskErrorKind>,
    pub last_error: Option<String>,
    pub metadata: Option<BTreeMap<String, String>>,
}
```

### Recording Metrics

```rust
// Task started
metrics.task_started();

// Task completed (via TaskMetrics)
metrics.task_completed(TaskMetrics { ... });

// Or via TaskResult
metrics.task_completed_from_result(task_name, session_id, &result);

// Run counters for structured events
metrics.increment_run_counter(RUN_COUNTER_LIKE_SUCCESS, 1);

// Snapshot
let stats = metrics.get_stats();
let rate = metrics.success_rate();
```

### Run Counters

Named constants for structured event tracking (exported to `run-summary.json`):

| Counter Constant | Purpose |
|---|---|
| `RUN_COUNTER_CANDIDATE_SCANNED` | Feed candidates evaluated |
| `RUN_COUNTER_LIKE_SUCCESS` / `RUN_COUNTER_LIKE_FAILURE` | Like outcomes |
| `RUN_COUNTER_RETWEET_SUCCESS` / `RUN_COUNTER_RETWEET_FAILURE` | Retweet outcomes |
| `RUN_COUNTER_FOLLOW_SUCCESS` / `RUN_COUNTER_FOLLOW_FAILURE` | Follow outcomes |
| `RUN_COUNTER_REPLY_SUCCESS` / `RUN_COUNTER_REPLY_FAILURE` | Reply outcomes |
| `RUN_COUNTER_BOOKMARK_SUCCESS` / `RUN_COUNTER_BOOKMARK_FAILURE` | Bookmark outcomes |
| `RUN_COUNTER_QUOTE_SUCCESS` / `RUN_COUNTER_QUOTE_FAILURE` | Quote outcomes |
| `RUN_COUNTER_DIVE_SUCCESS` / `RUN_COUNTER_DIVE_FAILURE` | Thread dive outcomes |
| `RUN_COUNTER_CLICK_ATTEMPTED` / `RUN_COUNTER_CLICK_SUCCESS` | Click learning |
| `RUN_COUNTER_RETRY_ATTEMPT` | Retries |
| `RUN_COUNTER_TRANSIENT_ERROR` / `RUN_COUNTER_PERMANENT_ERROR` / `RUN_COUNTER_FATAL_ERROR` | Error classification |
| `RUN_COUNTER_CIRCUIT_BREAKER_OPEN` | Circuit breaker state |
| `RUN_COUNTER_GRACEFUL_DEGRADATION` | Degradation events |
| `RUN_COUNTER_CONFIDENCE_HIGH/MEDIUM/LOW` | Pipeline confidence |

### Export Summary

At the end of a run, metrics are exported to `run-summary.json`:

```rust
metrics.export_summary_to("run-summary.json", active_sessions, healthy_sessions, &fan_out_metrics)?;
```

The JSON contains: timestamp, task outcomes, session health, twitteractivity counters, click learning counters, native lock metrics, and fan-out metrics.

## DSL Debug System

### DebugEvent

```rust
pub struct DebugEvent {
    pub timestamp: String,          // RFC 3339
    pub event_type: DebugEventType, // 8 variants
    pub action_index: Option<usize>,
    pub action_type: Option<String>,
    pub variable_name: Option<String>,
    pub variable_value: Option<String>,
    pub condition_result: Option<bool>,
    pub error: Option<String>,
}
```

### DebugEventType (8 variants)

| Variant | Occurs When |
|---|---|
| `ActionStart` | Action execution begins |
| `ActionComplete` | Action completes successfully |
| `ActionError` | Action fails |
| `Breakpoint` | Breakpoint condition hit |
| `VariableSet` | A watched variable changes |
| `TaskCallStart` | Sub-task call begins |
| `TaskCallComplete` | Sub-task call ends |
| `ConditionEvaluated` | Conditional expression evaluated |

All variants serialize to `snake_case` JSON (e.g., `"action_start"`).

### Breakpoint System

```rust
// On specific action index
let bp = Breakpoint::on_action(5);

// On any action of a type
let bp = Breakpoint::on_action_type("Click");

// Watch variable changes
let bp = Breakpoint::watch_variable("myVar");

// Combined filters (index + type)
let bp = Breakpoint {
    action_index: Some(2),
    action_type: Some("Click"),
    watch_variable: None,
    condition: Some(Arc::new(|vars: &HashMap<String, String>| {
        vars.get("status").map(|v| v == "active").unwrap_or(false)
    })),
};
```

`should_trigger(action_index, action_type, variables)` checks all specified filters.

### Variable Watching

```rust
pub fn watch_variable(
    watched: &mut HashMap<String, String>,
    debug_mode: bool,
    name: &str,
    value: &str,
) -> Option<DebugEvent> {
    // Returns Some(DebugEvent) if value changed and debug_mode is on
}
```

Only emits events when `debug_mode = true`. Uses a `&mut HashMap` to track previous values.

## DSL Profiling System

### ActionProfiler

Aggregate profiling per action type:

```rust
pub struct ActionProfiler {
    pub action_type: String,
    pub total_executions: u64,
    pub total_duration: Duration,
    pub min_duration: Option<Duration>,
    pub max_duration: Option<Duration>,
    pub failures: u64,
}
```

### ActionMetrics

Per-action timing:

```rust
let metrics = ActionMetrics::new(0, "Click");
let completed = metrics.complete();  // marks success
let failed = metrics.fail("Timeout");  // marks failure
```

### ExecutionReport

Comprehensive run report with JSON export:

```rust
let report = ExecutionReport { ... };
println!("{}", report.summary());  // "Task 'test' executed 10 actions in 5s (8 successful, 2 failed)"
let json = report.to_json();       // serde_json::Value
```

## Health Utilities (orchestrator/health.rs)

### format_duration

```rust
fn format_duration(ms: u64) -> String
// 500ms → "500ms"
// 5000ms → "5s"
// 125000ms → "2min 5s"
// 7320000ms → "2h 2min"
```

### should_mark_session_unhealthy

```rust
fn should_mark_session_unhealthy(kind: TaskErrorKind, was_cancelled: bool) -> bool
```

Returns `true` (mark unhealthy) when NOT cancelled AND error is: Timeout, Navigation, Session, Browser, or ExternalService.

## Adding a New Log Message

### Step 1: Choose the right level

```rust
info!("[task] Task {} started", task_name);
debug!("[scroll] Attempt {}: delta={}, result={:?}", attempt, delta, result);
warn!("[circuit] Breaker opened after {} failures", count);
error!("[browser] Session lost: {}", err);
```

### Step 2: Include context tags

Good: `info!("[feed] Scanned {} candidates", count);`
Better: `info!("[{engagement_type}] [SUCCESS] Liked tweet {}/{}", current, max);`

### Step 3: Set thread-local context (for auto-prefixing)

```rust
let _guard = scoped_log_context(LogContext {
    session_id: Some(session_id),
    profile_name: Some(profile_name),
    task_name: Some(task_name),
});
// All log!() calls within this scope get auto-prefixed
```

## Adding a New Run Counter

### Step 1: Declare the constant in `metrics.rs`

```rust
pub const RUN_COUNTER_MY_EVENT: &str = "my_event_name";
```

### Step 2: Track the event

```rust
metrics.increment_run_counter(RUN_COUNTER_MY_EVENT, 1);
```

### Step 3: Add to `run-summary.json` export

Add a new field to `TwitterActivityRunCounters`, `ClickLearningRunCounters`, or create a new counters struct. Then wire it up in `export_summary_to()`.

## Adding a New Metric Field

### Step 1: Add to `MetricsSnapshot` struct

```rust
pub struct MetricsSnapshot {
    // existing fields...
    pub my_new_metric: usize,
}
```

### Step 2: Add atomic counter to `MetricsCollector`

```rust
pub struct MetricsCollector {
    // ...
    my_new_metric: Arc<AtomicUsize>,
}
```

### Step 3: Add increment method

```rust
pub fn record_my_metric(&self, val: usize) {
    self.my_new_metric.fetch_add(val, Ordering::Relaxed);
}
```

### Step 4: Add to `get_stats()`

```rust
MetricsSnapshot {
    // ...
    my_new_metric: self.my_new_metric.load(Ordering::Acquire),
}
```

## Testing Your Logging Changes

```powershell
# Logger tests
cargo test --lib logger::tests

# Health logger tests
cargo test --lib health_logger::tests

# Metrics tests
cargo test --lib metrics::tests

# DSL debug tests
cargo test --lib task::dsl::debug::tests

# DSL profiling tests
cargo test --lib task::dsl::profiling::tests

# Health utilities tests
cargo test --lib orchestrator::health::tests
```

## Common Pitfalls

1. **Log level not applied**: The `--debug` flag sets `LevelFilter::Debug` AFTER the logger is registered — but `set_max_level` is called before any logging, so this is fine. However, `RUST_LOG` is read by `env_logger`, not `FileLogger`. The `--debug` flag is the official way to enable debug logging.

2. **chromiumoxide suppression**: If you need to see CDP WebSocket messages for debugging, you must remove or comment out the `chromiumoxide` filter in `logger.rs`. Otherwise, ALL chromiumoxide logs are silently dropped.

3. **Thread-local context isolation**: Each async task runs on a different thread. Always set `scoped_log_context()` at the start of each task execution. The context is NOT inherited across `await` points — it's per-thread.

4. **`Arc<String>` vs `String` in metrics**: Always use `Arc<String>` for `task_name` and `session_id` in `TaskMetrics`. Cloning an Arc is O(1) (ref-count increment), cloning a String is O(n). This matters when recording thousands of task completions.

5. **`Relaxed` vs `Acquire` ordering**: Atomic counters use `Relaxed` for increments (we only need eventual consistency), but `Acquire` in `get_stats()` ensures the final snapshot is consistent.

6. **Health logger not stopping**: Always call `health_logger.stop()` and await the handle before exiting. The background task uses `tokio::sync::Notify` and will run forever if not explicitly stopped.

7. **`DebugEvent` serialization**: Events use `chrono::Local::now().to_rfc3339()` for timestamps. For machine-readable logs, use the exported JSON. For human reading, use the `DebugEvent` struct directly.

8. **`Breakpoint::clone()` drops conditions**: The `condition` field uses `Arc<dyn Fn()>` which cannot be cloned. After cloning a breakpoint, the condition is `None`. Create fresh breakpoints if conditions are needed.

9. **`run-summary.json` overwrites**: The metrics `export_summary_to()` writes to a fixed path. Consecutive runs overwrite the file. For historical analysis, rename the file between runs.

10. **Memory snapshots are platform-specific**: `get_allocated_memory()` returns `None` on Windows, reads `/proc/self/status` on Linux. Memory monitoring accuracy varies by OS.
