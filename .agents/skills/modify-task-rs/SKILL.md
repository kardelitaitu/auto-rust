# modify-task-rs

Expert skill for **modifying existing Rust tasks** in the auto-rust framework without breaking the codebase.

## When to use

- User says "update the X task to do Y"
- User wants to change behavior of an existing Rust task
- User wants to add/remove logic, selectors, or parameters from a task
- User wants to change a task's policy or duration

Do **not** use this skill when creating a brand-new task (`create-new-task-rs`) or when the change is a DSL task (`modify-task-dsl`).

## Safety-first workflow

### Step 0: Read the full file before touching anything

```rust
// Read the task file, its policy, and its registration in mod.rs
src/task/<task_name>.rs       // The task itself
src/task/mod.rs               // Registration (TASK_NAMES, match arm)
src/task/policy.rs            // Task policy (permissions, timeout)
src/task/<related_utils>.rs   // Any shared utilities the task uses
```

**Never modify without reading first.** Understand the full function structure, imports, and test suite before making any changes.

### Step 1: Identify what can safely change vs what is structural

| ✅ Safe to modify | ❌ Must not change (unless deliberate) |
|---|---|
| Logic inside `run_inner()` | Public function signature: `pub async fn run(api: &TaskContext, payload: Value) -> Result<()>` |
| Helper/private functions | Module name and `pub mod` in `mod.rs` |
| Selectors and locators | Task name in `TASK_NAMES` array |
| Duration constant value | Match arm string in `execute_single_attempt` |
| Policy permissions | Policy constant name and `match_policy_by_name` entry |
| Test assertions and test data | Exported duration constant name referenced by policy.rs |
| `#[derive]` attributes on internal types | struct names/types referenced across modules |

### Step 2: Make the change

**Changing logic in the task body:**
- Edit `run_inner()` or helper functions directly
- Keep the `run()` → `run_with_timeout()` → `run_inner()` flow intact
- Use the same `api.*` methods the task already uses for consistency
- Follow existing patterns for logging, timing, and error handling

**Adding/removing helper functions:**
- Add new functions as private `fn` (no `pub`)
- If you make a function `pub`, use `code-searcher` to find and update all callers
- Remove dead code — if a `pub fn` or `pub const` is no longer used, find all references first

**Changing the duration constant:**
- Update `DEFAULT_MY_TASK_DURATION_MS` in the task file
- The policy in `policy.rs` references this constant — verify `get_policy("my-task")` still works
- Update the `#[test] fn task_duration_stays_within_bounds()` test assertion

**Changing the policy:**
- Edit the `pub static` in `src/task/policy.rs`
- `max_duration_ms` should reference the constant from the task module, not a hardcoded value
- Enable only what the task actually needs — start with `..Default::default()` and add flags
- Remember implied permissions: `allow_screenshot` → `allow_write_data`, etc.

**Adding a new parameter to the task's payload:**
- Extract it in `run_inner()` using `payload.get("param_name")`
- Use `unwrap_or(default_value)` so existing callers are not broken
- Add a test for the new parameter (both present and absent)

**Removing a parameter:**
- Search for all callers using `code-searcher` — find places where the task is dispatched with that parameter
- Remove the extraction from `run_inner()`
- If the parameter was in the validation logic (`src/validation/task.rs`), remove it there too

### Step 3: Handle breaking changes

**Breaking change** = anything that would cause a compile error or runtime failure for existing callers.

| Change | Impact | Fix |
|---|---|---|
| Renaming a task | Dispatch broken, policy lookup broken | Update mod.rs (3 places) + policy.rs (2 places) + tests |
| Changing `pub` function signature | Callers won't compile | Use `code-searcher` to find and update all callers |
| Removing a `pub const` duration | Policy.rs won't compile | Update the policy to use a hardcoded value or a different constant |
| Changing a struct field or type | All usages of that struct break | `code-searcher` — find and update all references |
| Adding a required parameter | Old callers will fail validation at runtime | Use `.unwrap_or(default)` instead of `?` or `expect()` |

**Always run `code-searcher` when changing any `pub` item** — functions, constants, types, or struct fields.

### Step 4: Update tests

Every modification breaks at least one test — that's the test suite doing its job.

**Types of tests to check:**

| After changing... | Update these tests |
|---|---|
| Duration constant | `task_duration_stays_within_bounds()` in the task module |
| Helper function logic | All tests that call that function |
| Selectors/locators | Locator ordering tests, state detection tests |
| Payload parameters | `extract_*_from_payload()` tests |
| Policy permissions | `test_*_has_extended_permissions()` or `test_all_task_policies_have_valid_timeouts()` in `policy.rs` |
| Added new functionality | Add new `#[test]` functions — test success case + at least one edge case |

**Test patterns used in this codebase:**
```rust
#[cfg(test)]
mod tests {
    use super::*;          // Import everything from the parent module
    use serde_json::json;  // For building test payloads

    #[test]
    fn test_descriptive_name() {
        // Arrange
        let payload = json!({"param": "value"});
        let expected = "expected";

        // Act
        let result = helper_function(&payload).unwrap();

        // Assert
        assert_eq!(result, expected);
    }
}
```

### Step 5: Validation — run in this order

```powershell
# 1. Fast compile check (catches syntax/type errors quickly)
cargo check

# 2. Run the specific task's tests (fast feedback)
cargo test --lib <task_name>

# 3. Run lib tests to check for regressions in other modules
cargo test --lib

# 4. Code formatting
cargo fmt --all --check
```

**If `cargo check` fails:** Read the error, fix it, repeat.
**If tests fail:** Read the failure message, figure out if the test or the code is wrong. If the behavior intentionally changed, update the test assertion. If the behavior is unexpected, fix the code.
**If `cargo fmt` fails:** Run `cargo fmt --all` to auto-format, then re-check.

### Step 6: Code review

After all checks pass, do a final review:

- [ ] Does the task still follow the `run()` → `run_inner()` pattern?
- [ ] Are all new functions private (no `pub`) unless they need to be?
- [ ] Are all unused imports/variables removed?
- [ ] Does the policy still match the task's actual needs?
- [ ] Are there tests covering the changed behavior?
- [ ] Did `cargo check` pass with zero errors and zero warnings?
- [ ] Did all related tests pass?

## Common pitfalls

1. **Changing `run()` signature** — `pub async fn run(api: &TaskContext, payload: Value) -> Result<()>` must remain exactly this. Change only `run_inner()`.
2. **Removing a `pub const` used by policy.rs** — `cargo check` will catch this, but it's worth checking first. The policy file references `crate::task::my_task::DEFAULT_MY_TASK_DURATION_MS`.
3. **Forgetting to update `match_policy_by_name`** — if you renamed a policy constant, the match arm still points to the old name.
4. **Adding `pub` to a helper function unintentionally** — Rust won't warn you, but it widens the API surface. Keep things private.
5. **Removing a parameter without finding all callers** — old callers will silently pass unused data, but if you make it required they'll break at runtime.
6. **Forgetting to update the test assertion after changing logic** — the old test expects the old behavior. Always run the tests.
7. **Changing a selector/locator** — the locator might be referenced by multiple code paths (e.g., `find_and_click_follow_button` and `is_already_following`). Search for the selector string across the codebase.
8. **Not running `cargo fmt`** — CI will fail on formatting issues later.
