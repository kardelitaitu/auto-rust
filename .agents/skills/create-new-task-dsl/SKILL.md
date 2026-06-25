# create-new-task-dsl

> last audited 26-06-26 by docs-auditor

Expert skill for creating new DSL (external) task files in the auto-rust framework.

## When to use

- User says "create a new DSL task for X"
- User wants a reusable YAML-based automation task that doesn't need Rust compilation
- User has a simple browser workflow (navigate, click, type, extract data)

Do **not** use this skill when the task needs complex logic, error handling, or integration with Rust-specific modules — use `create-new-task-rs` instead.

## How DSL tasks work

DSL tasks are YAML files (`.task` extension) that describe a sequence of browser actions. They are:

- **Discovered automatically** from directories configured in `config/default.toml` under `task_discovery.roots`
- **Executed by `DslExecutor`** — no Rust compilation needed
- **Portable** — can be shared across projects
- **Validated** at load time by `validate_task_definition()`
- **Support variable substitution** with `{{variable_name}}` syntax

## File format

A `.task` file has this structure:

```yaml
name: my-task
description: "What this task does"
policy: default

parameters:
  url:
    type: url
    required: true
    description: "Target URL"
  timeout_ms:
    type: integer
    required: false
    default: 5000
    description: "Timeout in ms"

actions:
  - navigate:
      url: "{{url}}"

  - wait_for:
      selector: "#main-content"
      timeout_ms: "{{timeout_ms}}"

  - click:
      selector: "#start-button"

  - extract:
      selector: ".result-text"
      variable: "result"

  - log:
      message: "Result: {{result}}"
      level: info
```

## Template for new DSL tasks

Use this minimal template:

```yaml
# Task Name
# Description of what this task does

name: my-new-task
description: "Brief description"
policy: default

parameters:
  url:
    type: url
    required: true
    description: "Target URL to navigate to"

actions:
  - log:
      message: "Starting my-new-task"
      level: info

  - navigate:
      url: "{{url}}"

  - wait_for:
      selector: "body"
      timeout_ms: 10000

  - log:
      message: "Task complete"
      level: info
```

## Policy system

Every DSL task declares a `policy` field in its YAML header. This maps to a
`TaskPolicy` defined in `src/task/policy.rs` that controls runtime budget and permissions.

**Available policy names:**

| Policy Name | Duration | Permissions | Use Case |
|---|---|---|---|
| `default` | 3 min | All off | Simple browsing |
| `pageview` | 2 min | All off | Page loading |
| `cookiebot` | 30s | screenshot, export_cookies | Cookie consent |
| `demo-keyboard` | varies | All off | Keyboard demo |
| `demo-mouse` | varies | All off | Mouse demo |
| `demoqa` | varies | All off | QA demo |
| `task-example` | varies | All off | Example template |
| `twitteractivity` | varies | cookies, clipboard, read_data, screenshot, dom_inspection | Full Twitter automation |
| Twitter sub-tasks | 45s | screenshot, export_cookies, clipboard, dom_inspection | Like, retweet, reply, follow, etc.

**Policy resolution flow:**

The DSL executor doesn't resolve policies directly. When a DSL task has
`policy: twitterdive`, the DSL framework calls `get_policy("twitterdive")` via the
`match_policy_by_name` helper — same lookup used by Rust tasks. If the policy name
isn't registered, the task falls back to `DEFAULT_TASK_POLICY` (3 min, no permissions).

**To add a custom policy for a DSL task**, follow the Rust task policy creation
steps in `create-new-task-rs`: add a `pub static` in `src/task/policy.rs` and a
match arm in `match_policy_by_name`, then reference it with `policy: my-policy-name`
in the DSL YAML.

## All available actions

| Action | Syntax | Description |
|---|---|---|
| `navigate` | `url: \"...\"` | Navigate to URL |
| `click` | `selector: \"...\"` | Click an element |
| `type` | `selector, text` | Type text into field |
| `wait` | `duration_ms: 1000` | Wait for duration |
| `wait_for` | `selector, timeout_ms?` | Wait for element visibility |
| `scroll_to` | `selector: \"...\"` | Scroll to element |
| `extract` | `selector, variable?` | Extract element text |
| `execute` | `script: \"...\"` | Run JavaScript |
| `clear` | `selector: \"...\"` | Clear input field |
| `hover` | `selector: \"...\"` | Hover over element |
| `select` | `selector, value, by_value?` | Select dropdown option |
| `right_click` | `selector: \"...\"` | Right-click element |
| `double_click` | `selector: \"...\"` | Double-click element |
| `screenshot` | `path?, selector?` | Take screenshot |
| `log` | `message, level?` | Log a message |
| `if` | `condition, then, else?` | Conditional branch |
| `loop` | `count?, condition?, actions` | Loop over actions |
| `foreach` | `variable, collection, actions` | Iterate over collection |
| `while` | `condition, actions` | While loop |
| `try` | `try_actions, catch_actions?, finally_actions?` | Error handling |
| `retry` | `actions, max_attempts?, delay?` | Retry on failure |
| `call` | `task, parameters?` | Call another task |
| `parallel` | `actions, max_concurrency?` | Run actions concurrently |

## Conditions

Available condition types for `if`, `while`, and `loop`:

| Type | Syntax |
|---|---|
| `element_exists` | `selector: \"...\"` |
| `element_visible` | `selector: \"...\"` |
| `text_equals` | `selector, value` |
| `text_matches` | `selector, pattern` |
| `variable_equals` | `name, value` |
| `variable_defined` | `name` |
| `true` / `false` | (no params) |
| `and` / `or` | `conditions: [...]` |
| `not` | `condition: {...}` |
| `numeric_greater_than` | `name, value` |
| `numeric_less_than` | `name, value` |
| `array_contains` | `name, value` |

## Variable syntax

- `{{variable_name}}` — reference a parameter or extracted variable
- `{{path.to.nested}}` — nested access (if the variable is a map/object)
- Variables are substituted at execution time by the `evaluator` module

## Include other tasks

```yaml
include:
  - path: ../lib/shared-setup.task
  - path: common/teardown.task
```

Includes merge actions from other task files. Cyclic includes are detected and blocked.

## Placement

DSL task files live wherever `task_discovery.roots` points in `config/default.toml`.
Common locations:

| Location | Purpose |
|---|---|
| `./tasks/` | Project-level tasks |
| `docs/tutorials/` | Example/tutorial tasks |
| External paths | Fully qualified paths via `TaskDiscoveryConfig` |

## Registration

DSL tasks do **not** need registration in `src/task/mod.rs` — they are auto-discovered
at startup by `TaskRegistry::load_external_tasks()`.

## Validation

After creating a DSL task:

```powershell
# Validate all external task files
cargo run -- --validate-tasks

# Full validation
cargo check
cargo test --lib
```

## Common pitfalls

1. **Missing `name` field** — the task file must declare a name
2. **Name doesn't match filename** — the file stem is the canonical name, not the declared name inside the file
3. **Required parameter not provided** — validation will fail at runtime
4. **Variable name mismatch** — `{{url}}` won't match a parameter named `target_url`
5. **Missing policy** — defaults to `"default"` if omitted
6. **Using unsupported action** — check the table above for all 23 actions
