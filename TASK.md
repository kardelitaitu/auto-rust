# Task Template

This file describes the minimum structure every task should follow.
Goal: keep tasks reliable, scalable, and easy to use.

## Must Have

1. A top-level task duration constant.
2. A task body wrapped with a timeout gate.
3. Boolean permissions defined in `src/task/policy.rs`.
4. A small test that checks the task duration stays within bounds.
5. Clear logging for start, progress, and completion.
6. Payload parsing that has safe defaults.
7. No duplicated policy logic inside the task unless required.

## Standard Shape

```rust
pub const DEFAULT_<TASK>_TASK_DURATION_MS: u64 = 60_000;

fn task_duration_ms() -> u64 {
    duration_with_variance(DEFAULT_<TASK>_TASK_DURATION_MS, 20)
}

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let duration_ms = task_duration_ms();
    timeout(Duration::from_millis(duration_ms), run_inner(api, payload))
        .await
        .map_err(|_| anyhow::anyhow!(
            "[task-name] Task exceeded duration budget of {}ms",
            duration_ms
        ))?
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    info!("[task-name] Task started");
    // task logic
    info!("[task-name] Task completed");
    Ok(())
}
```

## Task Checklist

- [ ] `pub const DEFAULT_<TASK>_TASK_DURATION_MS`
- [ ] `task_duration_ms()` helper
- [ ] `timeout(Duration::from_millis(...), run_inner(...))`
- [ ] `run_inner(...)` contains the actual task logic
- [ ] payload parsing has defaults
- [ ] permissions are in policy, not task code
- [ ] tests cover duration bounds
- [ ] tests cover payload parsing

## Required Pattern

### 1. Task owns duration

Each task file should expose a single top-level duration constant.

Example:

```rust
pub const DEFAULT_PAGEVIEW_TASK_DURATION_MS: u64 = 300_000;
```

### 2. Runtime applies deviation

Use the shared helper:

```rust
duration_with_variance(base_ms, 20)
```

This keeps runtime varied but bounded.

### 3. Policy stays the safety gate

`src/task/policy.rs` should keep:

- boolean permissions
- hard timeout ceiling

That gives a double gate:

- task duration for normal behavior
- policy timeout for safety

## Good Defaults

- `duration_ms`: task-local default
- `max_likes` / `max_scrolls` / similar knobs: safe small values
- boolean flags: `false` unless the task needs them
- logging: short and consistent

## What To Avoid

- hardcoding runtime values deep in helper functions
- duplicating policy flags inside the task
- making task logic depend on hidden global state
- using exact runtime assertions for randomized durations

## Test Pattern

Every task should have at least one test that checks:

- the task duration helper returns a value in the expected range
- payload parsing accepts defaults
- payload overrides work when present

Example:

```rust
#[test]
fn task_duration_stays_within_bounds() {
    let duration_ms = task_duration_ms();
    assert!(duration_ms >= 48_000 && duration_ms <= 72_000);
}
```

## Recommended Review Order

1. Does the task have a top-level duration constant?
2. Does the task use the shared variance helper?
3. Does the task body run under a timeout wrapper?
4. Are permissions still only in policy?
5. Do the tests cover the default and the bounds?

## Example Alternatives

| Approach | Pros | Cons |
|---|---|---|
| Task-owned duration + policy hard stop | clear ownership, safe fallback, easier to tune | two layers to reason about |
| Policy-only duration | centralized | task loses local runtime intent |
| Inline random ranges everywhere | quick for one-off work | duplicates logic and drifts fast |

## Final Rule

If a task does not follow this template, treat it as incomplete until it does.
