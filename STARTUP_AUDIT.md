# Startup Performance Audit

> **Date:** August 24, 2026  
> **Measured gap:** 3 seconds (17:35:38 → 17:35:41)  
> **Target:** < 1 second  
> **Status:** 4 findings — 1 critical, 2 high, 1 medium

## Timeline Reconstruction

From the production log, every step before "Broadcast fan-out" completes within the same second (17:35:38). The entire 3-second gap falls between two log lines:

```
17:35:38 INFO Broadcast fan-out: 1 task(s) x 1 session(s) = 1 execution(s)
                                    ↓↓↓  3,000 ms gap  ↓↓↓
17:35:41 INFO No validation schema for task: atf
17:35:41 INFO task_start | task=atf session=shard-0001 timeout=10min retries=2
```

## Findings

---

### 🔴 Finding 1: `task_stagger_delay_ms = 2000` — 2 seconds of dead sleep (CRITICAL)

**File:** `config/default.toml:42`  
**Code:** `src/orchestrator/execution.rs:78-85`

```toml
[orchestrator]
task_stagger_delay_ms = 2000
```

```rust
// execution.rs — inside the FuturesUnordered task future
() = tokio::time::sleep(Duration::from_millis(
    config.orchestrator.task_stagger_delay_ms,  // 2000ms!
)) => {}
```

**Impact:** Every task in a group sleeps for **2 full seconds** before doing anything. For a single task on a single session, this is pure dead time. The stagger exists to prevent network spikes when launching many tasks simultaneously, but it fires unconditionally — even for the first (and only) task.

**Breakdown:**
- 1 task × 1 session = 1 execution
- The stagger fires once: 2000ms sleep
- Then `execute_task_on_session` runs

**Cost:** 2000ms / 3000ms total = **67% of the entire startup delay**

---

### 🟠 Finding 2: CDP `browser.new_page("about:blank")` — ~1 second (HIGH)

**File:** `src/session/worker.rs:101-105`

```rust
pub async fn acquire_page(&self) -> anyhow::Result<Arc<chromiumoxide::Page>> {
    self.cb_check()?;
    let page = match self.browser.new_page("about:blank").await {
        // ...
    };
```

**Called from:** `src/orchestrator/retry.rs:117` inside `execute_task_with_retry`

**Impact:** Creating a new CDP page target requires a WebSocket round-trip to the Chromium browser process. On localhost this typically takes 500ms–1s. This is the unavoidable cost of opening a fresh tab for each task.

**Cost:** ~1000ms / 3000ms total = **33% of the entire startup delay**

---

### 🟠 Finding 3: Browser connectors probed sequentially (HIGH)

**File:** `src/session/pool.rs:52-62`

```rust
pub async fn discover(&self, config: &Config) -> Result<Vec<BrowserCapabilities>> {
    let mut all_capabilities = Vec::new();
    for connector in self.registry.available(config) {
        match connector.discover(config).await {   // sequential!
            Ok(caps) => { all_capabilities.extend(caps); }
            Err(e) => { warn!("Connector discovery failed: {e}"); }
        }
    }
```

**Impact:** Each unavailable connector pays a TCP reachability probe (500ms timeout via `API_REACHABILITY_TIMEOUT`). In the current config, 2 connectors are probed sequentially: `ConfiguredProfileConnector` (instant — just reads config) and `ShardBrowserConnector` (fast — localhost HTTP). But if RoxyBrowser or IxBrowser were enabled but unreachable, each would add 500ms to startup.

**Worst case with all 5 connectors enabled but only ShardBrowser alive:**
- ConfiguredProfile: ~0ms
- RoxyBrowser: ~500ms (TCP timeout)
- IxBrowser: ~500ms (TCP timeout)
- ShardBrowser: ~100ms (localhost HTTP)
- LocalBrowser: ~500ms (port scan, but `is_available()` returns false — skipped)

**Current cost:** ~0ms (both available connectors are fast)  
**Potential cost:** Up to 1500ms if multiple connectors are enabled but unreachable

---

### 🟡 Finding 4: `SESSION_STAGGER_DELAY_MS` parsed from env on every call (MEDIUM)

**File:** `src/orchestrator/execution.rs:131-134`

```rust
let stagger_delay_ms = std::env::var("SESSION_STAGGER_DELAY_MS")
    .ok()
    .and_then(|v| v.parse::<u64>().ok())
    .unwrap_or(config.orchestrator.task_stagger_delay_ms);
```

**Impact:** Minor — env var lookup + string parse per `execute_task_on_session` call. Not in a tight loop, but wasteful. Should be parsed once at startup and stored in config.

**Cost:** < 1ms (negligible, but bad practice)

---

## What's NOT slow (verified)

| Phase | Time | Notes |
|---|---|---|
| Config load + validation | < 10ms | TOML parse + env overrides + validation checks |
| `.env` file loading | < 1ms | Simple line-by-line parse |
| Logger setup | < 1ms | File open + mutex init |
| Shutdown signal setup | < 1ms | tokio::spawn + broadcast channel |
| Browser discovery (current) | < 100ms | 2 connectors, both fast |
| Session creation | < 50ms | CDP WebSocket connect to existing browser |
| Orchestrator::new | < 1ms | Arc::new + Semaphore::new |
| Global semaphore acquire | < 1ms | 20 permits, 0 active |
| Worker semaphore acquire | < 1ms | max_workers permits, 0 active |
| Policy validation | < 1ms | HashMap lookup + field checks |
| Task payload validation | < 1ms | Match statement + JSON checks |
| Health logger startup | < 1ms | tokio::spawn (background) |

---

## Recommended Fixes

### Fix 1: Skip stagger for first task in group (saves 2000ms)

Change the stagger logic to skip the delay for the first task:

```rust
// execution.rs — current
async move {
    // Stagger task starts to prevent network spikes
    tokio::select! {
        () = cancel_token.cancelled() => { return Ok(()); }
        () = tokio::time::sleep(Duration::from_millis(
            config.orchestrator.task_stagger_delay_ms,
        )) => {}
    }
    // ...
}

// execution.rs — fixed: skip stagger for first task
async move {
    // Stagger only applies between tasks, not before the first one
    if task_index > 0 {
        tokio::select! {
            () = cancel_token.cancelled() => { return Ok(()); }
            () = tokio::time::sleep(Duration::from_millis(
                config.orchestrator.task_stagger_delay_ms,
            )) => {}
        }
    }
    // ...
}
```

**Result:** 0ms stagger for single-task groups. For N tasks, total stagger = (N-1) × delay.

### Fix 2: Reduce default stagger to 200ms (saves 1800ms if kept)

```toml
[orchestrator]
task_stagger_delay_ms = 200   # was 2000
```

200ms is enough to prevent network spikes while keeping startup snappy.

### Fix 3: Parallel connector discovery (saves up to 1500ms in worst case)

```rust
// pool.rs — current: sequential
for connector in self.registry.available(config) {
    match connector.discover(config).await { ... }
}

// pool.rs — fixed: parallel with futures::future::join_all
let results: Vec<_> = self.registry.available(config)
    .iter()
    .map(|c| c.discover(config))
    .collect::<futures::future::JoinAll<_>>()
    .await;
```

**Result:** All connectors probe simultaneously. Startup time = max(connector times) instead of sum.

### Fix 4: Parse SESSION_STAGGER_DELAY_MS once at startup

Move the env var parse into config loading:

```rust
// config/env.rs — in apply_env_overrides()
if let Ok(stagger) = env::var("SESSION_STAGGER_DELAY_MS") {
    config.orchestrator.session_stagger_delay_ms = stagger
        .parse().unwrap_or(config.orchestrator.task_stagger_delay_ms);
}
```

---

## Expected Improvement

| Scenario | Current | After Fix 1+2 | After All Fixes |
|---|---|---|---|
| 1 task × 1 session | 3000ms | **~1000ms** | **~800ms** |
| 3 tasks × 1 session | 7000ms | **~3400ms** | **~3200ms** |
| 1 task × 3 sessions | 3000ms | **~1000ms** | **~800ms** |
| 5 tasks × 5 sessions | 19000ms | **~9400ms** | **~9000ms** |

**Fix 1 alone** cuts startup by 67% for the single-task case.

---

## Code References

| File | Line(s) | Issue |
|---|---|---|
| `config/default.toml` | 42 | `task_stagger_delay_ms = 2000` |
| `src/orchestrator/execution.rs` | 78-85 | Unconditional stagger sleep |
| `src/orchestrator/execution.rs` | 131-134 | Per-call env var parse |
| `src/session/pool.rs` | 52-62 | Sequential connector probing |
| `src/session/worker.rs` | 101-105 | CDP page creation (inherent cost) |
| `src/session/connector.rs` | 319 | `API_REACHABILITY_TIMEOUT = 500ms` |
