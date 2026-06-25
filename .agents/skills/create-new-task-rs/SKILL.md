# create-new-task-rs

Expert skill for creating a new built-in Rust task in the auto-rust framework.

## When to use

- User says "create a new task for X"
- User wants a new browser automation routine that doesn't fit an existing task
- User asks you to add a reusable routine following the project's task pattern

Do **not** use this skill when the task should be a DSL/external task (`.task` file).

## How it works

Creating a new Rust task requires changes to **2 files** (one new, one modified):

### 1. Create `src/task/my_new_task.rs`

Use `src/task/task_example.rs` as a reference template. The file must follow this pattern:

```rust
//! Description of what this task does.
//!
//! More detail about the task's behavior, selectors, and edge cases.

use anyhow::Result;
use log::info;
use serde_json::Value;

use crate::prelude::TaskContext;
use crate::utils::timing::{duration_with_variance, run_with_timeout};

// === Constants ===

/// Default task runtime budget in milliseconds.
pub const DEFAULT_MY_TASK_DURATION_MS: u64 = 60_000;

// === Entry Point ===

/// Main task entry point.
pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let duration_ms = task_duration_ms();
    run_with_timeout(duration_ms, "my-task", run_inner(api, payload)).await
}

fn task_duration_ms() -> u64 {
    duration_with_variance(DEFAULT_MY_TASK_DURATION_MS, 20)
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    info!("My task started");

    // Parse config from payload
    // Perform actions using api.* methods
    // Use crate-specific utilities when needed

    info!("My task completed");
    Ok(())
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::task_duration_ms;

    #[test]
    fn task_duration_stays_within_bounds() {
        let duration_ms = task_duration_ms();
        assert!((48_000..=72_000).contains(&duration_ms));
    }
}
```

**Key rules for the task file:**
- `pub async fn run(api: &TaskContext, payload: Value) -> Result<()>` — the public entry point
- Use `run_with_timeout` for top-level timeout wrapping
- Use `duration_with_variance` for runtime randomization
- Prefer `api.*` methods for browser interactions (click, keyboard, wait_for, etc.)
- Include at minimum a duration test
- Document the module with `//!` doc comments

### 2. Register in `src/task/mod.rs`

Make **3 changes**:

**a) Add `pub mod` declaration** (alphabetically among existing mods):
```rust
pub mod my_new_task;
```

**b) Add to `TASK_NAMES` constant** (alphabetically, kebab-case):
```rust
pub const TASK_NAMES: &[&str] = &[
    "cookiebot",
    // ...
    "my-task",      // <-- add here
    "pageview",
    // ...
];
```

**c) Add match arm in `execute_single_attempt`** (alphabetically):
```rust
"my-task" => my_new_task::run(api, payload.clone()).await,
```

### 3. Register a policy in `src/task/policy.rs`

Every task **must** have a policy that defines its runtime budget and permissions.

**a) Add a `pub static` policy constant** (alphabetically among existing ones):

```rust
/// `MyTask` policy - describe what it's allowed to do and why.
pub static MY_TASK_POLICY: std::sync::LazyLock<TaskPolicy> =
    std::sync::LazyLock::new(|| TaskPolicy {
        max_duration_ms: DurationMs::new_const(
            crate::task::my_new_task::DEFAULT_MY_TASK_DURATION_MS,
        ),
        permissions: TaskPermissions {
            // Enable only what the task actually needs:
            allow_screenshot: true,        // ✅ Debug screenshots
            allow_export_cookies: true,    // ✅ Auth verification
            // allow_write_data implied by allow_screenshot
            ..Default::default()           // Everything else off
        },
    });
```

**b) Add match arm in `match_policy_by_name`** (alphabetically):

```rust
fn match_policy_by_name(policy_name: &str) -> &'static TaskPolicy {
    match policy_name {
        // ...
        "my-task" => &MY_TASK_POLICY,
        // ...
        _ => &DEFAULT_TASK_POLICY,
    }
}
```

**c) Update `test_all_task_policies_have_valid_timeouts`** — add `"my-task"` to the test's `task_names` array.

### Policy reference

Policies live in `src/task/policy.rs`. Each task gets a `LazyLock<TaskPolicy>` constant with:

| Field | Type | Description |
|---|---|---|
| `max_duration_ms` | `DurationMs` | **Mandatory** — task is killed after this. Use the constant from the task module. |
| `permissions` | `TaskPermissions` | 12 boolean flags controlling what the task can do (see below). |

**Implied permissions** — enabling some flags automatically enables others:
- `allow_screenshot` → implies `allow_write_data`
- `allow_export_session` → implies `allow_export_cookies`
- `allow_import_session` → implies `allow_import_cookies`

**All 12 permission flags:**

| Permission | Use Case |
|---|---|
| `allow_screenshot` | Task needs to capture debug screenshots |
| `allow_export_cookies` | Task needs to verify/exchange cookies |
| `allow_import_cookies` | Task needs to load pre-existing cookies |
| `allow_export_session` | Task needs full session backup (cookies + localStorage) |
| `allow_import_session` | Task needs to restore a full session |
| `allow_session_clipboard` | Task copies/pastes data (e.g., tweet text) |
| `allow_read_data` | Task reads persona files or config from `config/`, `data/` |
| `allow_write_data` | Task writes output files or state |
| `allow_http_requests` | Task calls external APIs (GET, POST) |
| `allow_dom_inspection` | Task inspects element styles, positions, layout |
| `allow_browser_export` | Task needs complete browser data (cookies + storage + IndexedDB) |
| `allow_browser_import` | Task needs to restore complete browser data |

**Rule of thumb:** Start with all permissions off (`..Default::default()`), then enable only what the task explicitly needs.

### Policy inheritance pattern (for Twitter-family tasks)

Twitter sub-tasks (like, retweet, reply, follow) inherit from `TWITTER_BASE_POLICY`:

```rust
pub static TWITTERLIKE_POLICY: std::sync::LazyLock<TaskPolicy> =
    std::sync::LazyLock::new(|| TaskPolicy {
        max_duration_ms: DurationMs::new_const(
            crate::task::twitterlike::DEFAULT_TWITTERLIKE_TASK_DURATION_MS,
        ),
        permissions: TWITTER_BASE_POLICY.permissions.clone(),
    });
```

To extend (add permissions on top of base):

```rust
pub static TWITTERDIVE_POLICY: std::sync::LazyLock<TaskPolicy> =
    std::sync::LazyLock::new(|| TaskPolicy {
        permissions: crate::task::policy::TaskPermissions {
            allow_read_data: true, // Extra: reads persona files
            ..TWITTER_BASE_POLICY.permissions.clone()
        },
        max_duration_ms: DurationMs::new_const(/* ... */),
    });
```

## Validation

After creating the task, run:

```powershell
cargo check
cargo test --lib
cargo fmt --all --check
```

This verifies:
- The module compiles and is properly registered
- The policy has a valid timeout and is linked to `match_policy_by_name`
- No regressions in existing tasks
- Code formatting is consistent

## Common pitfalls

1. **Forgetting `pub mod` in `mod.rs`** — the file won't compile
2. **Forgetting the `TASK_NAMES` entry** — `perform_task` won't dispatch to it
3. **Forgetting to add a policy** — `get_policy("my-task")` returns `DEFAULT_TASK_POLICY` (3 min timeout, no permissions)
4. **Forgetting to update `match_policy_by_name`** — renders the policy constant unreachable
5. **Forgetting to update `test_all_task_policies_have_valid_timeouts`** — the test will pass but won't cover the new task
6. **Using wrong function signature** — `run()` must take `(&TaskContext, Value)` and return `Result<()>`
7. **Not using `payload.clone()`** — the match arm passes `payload.clone()` to avoid ownership issues
8. **Missing `#[cfg(test)]`** — test modules must be gated
9. **Name mismatch** — the task name in `TASK_NAMES` (kebab-case) must match the match arm string literal and the `policy_name`
