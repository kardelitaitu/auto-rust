# Release v0.1.0 - DSL Task System Complete

**Release Date:** May 4, 2026  
**Tag:** `v0.1.0`

## Overview

This is a **major milestone release** marking the completion of the full DSL Task System. The framework now supports declarative browser automation through YAML/TOML task definitions with 17 different action types, task composition, includes, parameters, and comprehensive observability.

---

## What's New (All 11 Phases)

### Phases 1-2: Foundation
- **Task Registry** - Unified registry for built-in and external tasks
- **Task Discovery** - Auto-discovery from configured directories
- **TaskDescriptor** - Metadata model with source tracking

### Phases 3-5: DSL Core
- **Parser** - YAML/TOML task file parsing with validation
- **Executor** - Bridge between DSL actions and TaskContext API
- **Variable Substitution** - `{{param_name}}` syntax
- **Control Flow** - If/else conditions, loops with safety limits

### Phase 6A: Validation CLI
- `--validate-tasks` flag for pre-flight validation
- Checks DSL syntax, name consistency, required fields

### Phase 6B: Hot Reload
- `--watch` flag for development auto-reload
- File creation, modification, deletion, rename events

### Phase 7: Task Parameters
- CLI `key=value` syntax for parameter passing
- Parameter validation (required, defaults, types)
- Variable substitution with `{{variable}}` syntax

### Phase 8: Task Composition (Call Action)
- Tasks can invoke other tasks
- Recursion limit (MAX_CALL_DEPTH = 10)
- Parameter inheritance and overrides

### Phase 9: Task Includes
- Modular task files with `include:`
- Recursive resolution
- Circular include detection
- Action and parameter merging

### Phase 10: More Actions
Added 6 new actions:
- Screenshot (full page or element)
- Clear (input field)
- Hover (element hover)
- Select (dropdown)
- RightClick
- DoubleClick

### Phase 11: Observability
- **ActionMetrics** - Per-action timing, success/failure
- **ExecutionReport** - Complete execution summary
- **JSON Export** - Machine-readable reports
- Enhanced logging with per-action duration

---

## Complete DSL Action Reference

| Action | Description | Parameters |
|--------|-------------|------------|
| `navigate` | Navigate to URL | `url` |
| `click` | Click element | `selector` |
| `type` | Type text | `selector`, `text` |
| `wait` | Pause execution | `duration_ms` |
| `wait_for` | Wait for element | `selector`, `timeout_ms` |
| `scroll_to` | Scroll to element | `selector` |
| `extract` | Extract element text | `selector`, `variable` |
| `execute` | Execute JavaScript | `script` |
| `if` | Conditional block | `condition`, `then`, `else` |
| `loop` | Repeat actions | `count` or `condition`, `actions` |
| `call` | Call another task | `task`, `parameters` |
| `log` | Log message | `message`, `level` |
| `screenshot` | Capture screenshot | `path`, `selector` |
| `clear` | Clear input | `selector` |
| `hover` | Hover element | `selector` |
| `select` | Select dropdown | `selector`, `value`, `by_value` |
| `right_click` | Right-click | `selector` |
| `double_click` | Double-click | `selector` |

---

## Example Task Files

### Simple Task
```yaml
name: simple_login
description: "Basic login flow"
policy: default

actions:
  - action: navigate
    url: "https://example.com/login"
  
  - action: wait_for
    selector: "#username"
    timeout_ms: 5000
  
  - action: type
    selector: "#username"
    text: "user@example.com"
  
  - action: type
    selector: "#password"
    text: "password123"
  
  - action: click
    selector: "#login-button"
  
  - action: wait
    duration_ms: 2000
  
  - action: screenshot
    path: "/tmp/post_login.png"
```

### Parameterized Task
```yaml
name: parameterized_login
description: "Login with configurable parameters"
policy: default

parameters:
  url:
    type: url
    description: "Login page URL"
    required: true
  username:
    type: string
    description: "Username"
    required: true
  password:
    type: string
    description: "Password"
    required: true
  remember_me:
    type: boolean
    description: "Remember login"
    default: false

actions:
  - action: navigate
    url: "{{url}}"
  
  - action: wait_for
    selector: "#username"
    timeout_ms: 5000
  
  - action: type
    selector: "#username"
    text: "{{username}}"
  
  - action: type
    selector: "#password"
    text: "{{password}}"
  
  - action: if
    condition:
      variable_equals:
        name: "remember_me"
        value: true
    then:
      - action: click
        selector: "#remember-me-checkbox"
  
  - action: click
    selector: "#login-button"
  
  - action: wait
    duration_ms: 2000
```

### Task with Includes
```yaml
name: full_workflow
description: "Complete workflow with reusable components"
policy: default

include:
  - path: common_setup.task
  - path: authentication.task

parameters:
  target_url:
    type: url
    required: true

actions:
  - action: navigate
    url: "{{target_url}}"
  
  - action: wait_for
    selector: "#dashboard"
    timeout_ms: 10000
  
  - action: extract
    selector: "#welcome-message"
    variable: "welcome_text"
  
  - action: log
    message: "Welcome message: {{welcome_text}}"
    level: info
  
  - action: screenshot
    path: "/tmp/dashboard.png"
```

### Task with Loops
```yaml
name: process_items
description: "Process multiple items with retry"
policy: default

parameters:
  max_items:
    type: integer
    default: 10

actions:
  - action: loop
    count: "{{max_items}}"
    actions:
      - action: click
        selector: ".item-button"
      
      - action: wait
        duration_ms: 500
      
      - action: if
        condition:
          element_visible:
            selector: ".success-message"
        then:
          - action: log
            message: "Item processed successfully"
            level: info
        else:
          - action: log
            message: "Item processing failed"
            level: warn
      
      - action: scroll_to
        selector: ".next-item"
```

### Task Composition
```yaml
name: complex_workflow
description: "Workflow using task composition"
policy: default

parameters:
  base_url:
    type: url
    default: "https://example.com"

actions:
  # Call login task
  - action: call
    task: parameterized_login
    parameters:
      url: "{{base_url}}/login"
      username: "admin"
      password: "{{admin_password}}"
  
  # Navigate to data page
  - action: navigate
    url: "{{base_url}}/data"
  
  # Process items in parallel (conceptual)
  - action: call
    task: process_items
    parameters:
      max_items: 5
  
  # Take final screenshot
  - action: screenshot
    path: "/tmp/final_state.png"
  
  - action: log
    message: "Complex workflow completed"
    level: info
```

---

## CLI Usage

```bash
# List all available tasks
cargo run -- --list-tasks

# Run a task
cargo run -- simple_login

# Run with parameters
cargo run -- parameterized_login url=https://example.com username=admin password=secret

# Validate tasks without execution
cargo run -- --validate-tasks

# Run with hot reload (auto-reload on file changes)
cargo run -- --watch simple_login

# Dry run (show what would execute)
cargo run -- --dry-run simple_login
```

---

## Configuration

```toml
[task_discovery]
enabled = true
roots = ["~/.config/auto/tasks", "./tasks"]
extensions = ["task", "yaml", "yml", "toml"]
```

---

## Execution Report Example

```rust
let mut executor = DslExecutor::new(api, &task_def);
executor.execute().await?;

let report = executor.execution_report(true);
println!("{}", report.summary());
// Task 'login' executed 5 actions in 1.23s (4 successful, 1 failed)

// Export to JSON
let json = report.to_json();
```

---

## Statistics

| Metric | Value |
|--------|-------|
| **Version** | 0.1.0 |
| **Total Tests** | 2117 |
| **Test Coverage** | Library: 85%+ |
| **Doc Tests** | 61 |
| **Lines of Code** | ~35,000 |
| **DSL Actions** | 18 |
| **Phases Completed** | 11 |
| **Commits** | 50+ |

---

## Breaking Changes

None. All changes are backward compatible.

---

## Migration from v0.0.x

No migration needed. All existing Rust tasks continue to work unchanged.

---

## What's Next (v0.2.0+)

| Feature | Priority |
|---------|----------|
| Parallel action execution | High |
| Plugin system | Medium |
| More built-in actions | Medium |
| Task marketplace | Low |
| Visual task builder | Low |

---

## Acknowledgments

This release represents 11 phases of iterative development, resulting in a complete declarative automation framework built on top of the solid Rust foundation.

---

*Released with 🦀 Rust 2021 edition*
