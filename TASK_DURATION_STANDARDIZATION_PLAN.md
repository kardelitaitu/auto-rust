# Task Duration Standardization Plan

## Goal

Standardize task runtime control so every task has:

1. A task-owned base duration
2. A random runtime deviation of ±20%
3. A policy hard stop as the safety gate
4. Boolean permissions kept in `policy.rs`

## Target Model

### 1. Task owns duration

Each task file should define a top-level constant such as:

```rust
pub const DEFAULT_PAGEVIEW_TASK_DURATION_MS: u64 = 300_000;
```

This is the task's normal runtime budget.

### 2. Runtime applies deviation

The task runtime should randomize around the base duration using a uniform ±20% spread.

Example:

- base: `300_000`
- runtime range: `240_000..360_000`

### 3. Policy remains the hard stop

`policy.rs` should keep `max_duration_ms` as the double gate:

- it catches tasks that run too long
- it protects tasks that forget to define a duration
- it keeps enforcement separate from task intent

### 4. Permissions stay in policy

Boolean permissions should remain centralized in `policy.rs`:

- `allow_screenshot`
- `allow_read_data`
- `allow_http_requests`
- other capability flags

## Rollout Order

1. Audit every task file for a top-level duration constant.
2. Normalize the naming pattern across all task files.
3. Update task execution paths to apply ±20% deviation from the base duration.
4. Keep policy timeouts as the safety ceiling.
5. Add tests for:
   - task default duration
   - randomized duration bounds
   - policy timeout enforcement
   - permissions defaults
6. Run `cargo check` and `cargo test`.

## Current Direction

The current direction is to keep the codebase simple:

- task file owns its runtime budget
- policy owns permissions and safety stop
- runtime owns the random deviation logic

## Alternatives

| Option | Pros | Cons |
|---|---|---|
| Task duration + policy hard stop | clear ownership, safe fallback, easy to tune per task | two places to reason about |
| Policy-only duration control | centralized, simple enforcement | task files lose local runtime intent |
| Shared duration registry | consistent values, one source of truth | more indirection, harder to tune per task |

## Recommended Standard

Use this default pattern for most tasks:

- task file: `DEFAULT_<TASK>_TASK_DURATION_MS`
- runtime: apply ±20% deviation
- policy: keep `max_duration_ms` as the hard stop
- permissions: remain in `policy.rs`

## Verification Checklist

- [ ] Each task has a top-level duration constant
- [ ] Runtime applies ±20% duration deviation
- [ ] Policy timeout still kills runaway tasks
- [ ] Permissions remain unchanged
- [ ] `cargo check` passes
- [ ] `cargo test` passes

## Notes

- Short tasks can still use smaller defaults if they are intentionally lightweight.
- The policy timeout should stay explicit so timeout testing remains reliable.
- The duration constant should stay easy to edit at the top of each task file.
