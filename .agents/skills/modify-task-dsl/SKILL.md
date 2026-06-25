# modify-task-dsl

Expert skill for **modifying existing DSL (YAML) task files** in the auto-rust framework without breaking the codebase.

## When to use

- User says "update the X DSL task to add/remove/change an action"
- User wants to change parameters, selectors, or control flow in an existing `.task` file
- User wants to change the policy assigned to a DSL task
- User needs to fix a broken DSL task

Do **not** use this skill when creating a brand-new DSL task (`create-new-task-dsl`) or when the change is a Rust task (`modify-task-rs`).

## Safety-first workflow

### Step 0: Read the full file before touching anything

```yaml
# Read the task file itself
<task-file>.task                         # e.g., docs/tutorials/intermediate/handle-errors.task

# Also read any included files — check the include: path in the task header
# (there is no global lib/ directory; includes are task-relative)

# Read the policy definition (in Rust)
src/task/policy.rs                       # to verify policy: name is valid

# Read any test files that reference this task
tests/dsl_integration_tests.rs           # for DSL integration test patterns
```

**Never modify without reading first.** DSL tasks can have complex control flow (nested `if`, `loop`, `try/retry`) and changing one action can break the entire workflow.

### Step 1: Understand the task structure

A DSL `.task` file has these parts:

```yaml
name: my-task
description: "What this task does"
policy: default          # <-- maps to a TaskPolicy in src/task/policy.rs

parameters:
  url:
    type: url
    required: true
    description: "Target URL"

include:                 # <-- optional: merge actions from other files
  - path: ../lib/setup.task

actions:
  - navigate:
      url: "{{url}}"
  # ... more actions
```

**Key constraint:** The `name:` field must match the file stem for auto-discovery. If the file is `my-task.task`, the name should be `my-task` (or at least that's what the registry resolves to).

### Step 2: Identify what can safely change

| ✅ Safe to modify | ⚠️ Risky — needs care | ❌ Must not change (unless deliberate) |
|---|---|---|
| Description text | Policy name (may change runtime budget) | File name (changes task identity) |
| Actions list (add/remove/reorder) | Required parameters (breaks callers) | Structural YAML type (must stay a valid DSL) |
| Optional parameters (add/remove) | Variable names in actions ({{x}}) | — |
| Include paths (add/remove) | Action types (if/loop/retry nesting) | — |
| Selector values | Commented-out actions | — |
| Logging/log levels | Screenshot paths | — |

### Step 3: Make the change

**Adding a new action:**
- Insert at the right position in the sequence
- Use one of the 23 supported actions (see `create-new-task-dsl` reference)
- If the action needs data from a previous step, ensure the variable is extracted first

**Removing an action:**
- Delete the action block
- Check that no subsequent action references a variable that was set by the removed action
- If the removed action was inside an `if`/`loop`/`try` block, make sure the block still has valid content

**Changing a selector:**
- Search for the selector in other tasks or test files — DSL tasks share common patterns like `[data-testid="like"]`
- Test the selector manually if possible (browser DevTools)
- Consider adding fallback selectors if the element might change

**Changing parameters:**
- Adding a required parameter breaks all callers that use `call:` to invoke this task
- Adding an optional parameter is safe — use a sensible `default:` value
- Renaming a parameter means updating all `{{old_name}}` → `{{new_name}}` references in actions
- Removing a parameter requires finding all callers and removing the argument

**Changing the policy:**
- Update `policy: new-policy-name` in the YAML header
- Verify the policy exists in `src/task/policy.rs` via `match_policy_by_name`
- If the policy doesn't exist yet, follow `create-new-task-rs` to add one
- If the name is misspelled or unregistered, the system silently falls back to `DEFAULT_TASK_POLICY` (3 min, no permissions)

**Restructuring control flow:**
- Adding an `if` condition: keep the condition simple (one check), test both `then` and `else` paths
- Wrapping in a `try`/`retry`: set a reasonable `max_attempts: 3` and `delay_ms: 1000`
- Adding a `loop`/`foreach`: ensure there's a termination condition to avoid infinite loops
- Nesting control flow: each level adds complexity — 2 levels max recommended

### Step 4: Handle breaking changes

**Breaking change** = anything that causes a runtime failure or changes the task's identity.

| Change | Impact | Fix |
|---|---|---|
| Changing `name:` field | Upstream `call:` references break | Update `call` params in all referring tasks |
| Changing file name | Task no longer discovered | Old file name is the canonical identity — keep it |
| Removing a required parameter | Validator rejects `call` without it | Either make the param optional or update all callers |
| Changing a variable name ({{x}}) | Downstream actions silently use empty strings | Update every `{{x}}` reference in actions and includes |
| Changing policy to unknown name | Silent fallback to default policy | Add the policy to `policy.rs` or use an existing one |

DSL tasks are validated at load time — `cargo test --lib` will catch many issues because the parser validates the task definitions during registry initialization.

### Step 5: Validation

```powershell
# 1. Validate all external task files (boolean flag, no path arg)
auto-rust --validate-tasks

# 2. Run the DSL integration tests (catches most structural issues)
cargo test dsl_integration

# 3. Run lib tests (catches full-registry and validation issues)
cargo test --lib

# 4. Check compile (validates the DSL parsing code itself hasn't changed)
cargo check
```

**What the validator catches:**
- Missing required fields (`name`, `description`, `actions`)
- Invalid action names (typos like `navigat` instead of `navigate`)
- Invalid condition types
- Parameter type mismatches
- Cyclic includes
- Nested control flow depth violations

**What the validator does NOT catch:**
- Logical errors (wrong selector, wrong URL)
- Variable name mismatches between extract and usage
- Policy names that don't exist in the registry (silent default fallback)
- Runtime errors (element not found, timeout exceeded)

### Step 6: Post-change checklist

- [ ] Does the YAML parse without syntax errors? (Check with validator)
- [ ] If you changed `policy:`, does that policy exist in `src/task/policy.rs`?
- [ ] If you changed `parameters:`, are `{{}}` references in actions still correct?
- [ ] If you added `include:`, is the included file accessible and not cyclic?
- [ ] If you moved `actions:` content between files, did you remove the original?
- [ ] Did `auto-rust --validate-task` pass?
- [ ] Did `cargo test dsl_integration` pass?
- [ ] Did `cargo test --lib` pass?

## Common pitfalls

1. **YAML indentation errors** — DSL uses YAML meaning indentation matters. Use 2-space indentation. A single space too many/few breaks the entire file.
2. **Variable typo in actions** — `{{urll}}` won't error at validation, only at runtime (empty string).
3. **Changing `name:` doesn't change discovery** — The file stem (<filename>.task) is the canonical name. The declared `name:` is used only for `call:` references.
4. **Silent default policy fallback** — If `policy: my-custom` doesn't match anything in `match_policy_by_name`, the task runs under `DEFAULT_TASK_POLICY` (3 min, no permissions). This can cause confusing runtime failures.
5. **Orphaned `call:` references** — If you rename this task, other tasks that `call:` it will fail. Search for the old name across all `.task` files.
6. **Cyclic includes** — If task A includes task B and task B includes task A, loading panics. Always check the include graph.
7. **Removing an `extract` that downstream actions depend on** — The downstream action won't error, it'll just use an empty string.
8. **Breaking `try`/`retry` pyramids** — If you remove the action that `retry` is supposed to retry, the block becomes empty and the task will stall.

> last audited 26-06-26 by docs-auditor
