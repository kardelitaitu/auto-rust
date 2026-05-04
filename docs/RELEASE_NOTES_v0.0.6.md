# Release v0.0.6 - DSL Task System & Hot Reload

**Release Date:** May 4, 2026  
**Tag:** `v0.0.6`

## Overview

This release introduces the **complete DSL Task System** for external task definitions, enabling declarative browser automation through YAML/TOML files. It also adds hot reload for development workflows and comprehensive CLI tooling for task management.

---

## New Features

### Phase 3-5: DSL Foundation & Execution (v0.0.6)

| Component | Description | Status |
|-----------|-------------|--------|
| **DSL Parser** | YAML/TOML task file parsing with validation | ✅ |
| **DSL Executor** | Bridge between DSL actions and TaskContext API | ✅ |
| **Variable Substitution** | `{{param_name}}` syntax in actions | ✅ |
| **Control Flow** | If/else conditions, loops with safety limits | ✅ |
| **External Task Loading** | Auto-discovery from configured directories | ✅ |
| **Task Registry Integration** | Unified registry for built-in and DSL tasks | ✅ |

### Phase 6A: Task Validation CLI

| Flag | Purpose | Example |
|------|---------|---------|
| `--validate-tasks` | Validate all external tasks without execution | `cargo run -- --validate-tasks` |

**Validation checks:**
- DSL syntax (YAML/TOML)
- Name consistency (file vs task name)
- Required fields (name, actions)
- Action validity

### Phase 6B: Hot Reload

| Flag | Purpose | Example |
|------|---------|---------|
| `--watch` | Auto-reload tasks when files change | `cargo run -- --watch my_task` |

**Supported events:**
- File creation
- File modification
- File deletion
- File rename

### Phase 7: Task Parameters

| Feature | Syntax | Example |
|---------|--------|---------|
| CLI parameters | `key=value` | `my_login url=https://example.com` |
| Variable substitution | `{{param}}` | `url: "{{target_url}}"` |
| Parameter validation | Required/optional with defaults | See ParameterDef |

**Parameter types:** String, Integer, Boolean, Url, Selector

---

## Supported DSL Actions

| Action | Description | Parameters |
|--------|-------------|------------|
| `navigate` | Navigate to URL | `url` |
| `click` | Click element | `selector` |
| `type` | Type text into element | `selector`, `text` |
| `wait` | Pause execution | `duration_ms` |
| `wait_for` | Wait for element existence | `selector`, `timeout_ms` |
| `scroll_to` | Scroll element into view | `selector` |
| `extract` | Extract element text to variable | `selector`, `variable` |
| `log` | Log message | `message`, `level` |
| `if` | Conditional block | `condition`, `then`, `else` |
| `loop` | Repeat actions | `count` or `condition`, `actions` |
| `call` | Call another task (stub) | `task`, `parameters` |

---

## CLI Enhancements

### New Flags

```bash
# List all available tasks
cargo run -- --list-tasks

# Validate external tasks
cargo run -- --validate-tasks

# Run with hot reload
cargo run -- --watch task_name

# Dry run (show what would execute)
cargo run -- --dry-run task_name
```

### Task Execution with Parameters

```bash
# Pass parameters to DSL tasks
cargo run -- my_login url=https://example.com wait_ms=5000

# Multiple parameters
cargo run -- parameterized_search query="rust automation" results=10
```

---

## Example DSL Task

```yaml
name: parameterized_login
description: "Login with configurable URL"
parameters:
  url:
    type: url
    description: "Login page URL"
    required: true
  username:
    type: string
    description: "Username"
    default: "guest"
  delay_ms:
    type: integer
    description: "Wait after login"
    default: 1000

actions:
  - action: navigate
    url: "{{url}}"
  
  - action: wait_for
    selector: "#username"
    timeout_ms: 5000
  
  - action: type
    selector: "#username"
    text: "{{username}}"
  
  - action: click
    selector: "#login-button"
  
  - action: wait
    duration_ms: "{{delay_ms}}"
  
  - action: log
    message: "Login completed for {{username}}"
    level: info
```

---

## Configuration

### Task Discovery

```toml
[task_discovery]
enabled = true
roots = ["~/.config/auto/tasks", "./tasks"]
extensions = ["task", "yaml", "yml", "toml"]
```

---

## Statistics

| Metric | Value |
|--------|-------|
| **Total Tests** | 2103 |
| **Test Coverage** | Library: 85%+ |
| **Doc Tests** | 61 |
| **Lines of Code** | ~30,000 |
| **New Modules** | 5 (dsl, dsl_executor, watcher, etc.) |
| **Phases Completed** | 7 |

---

## Migration Notes

- No breaking changes to existing Rust tasks
- All built-in tasks continue to work unchanged
- External tasks are opt-in via configuration
- CLI interface remains backward compatible

---

## What's Next (v0.0.7+)

| Phase | Feature | Priority |
|-------|---------|----------|
| 8 | **Task Composition** - Call action implementation | High |
| 9 | **Task Includes** - Modular task files | Medium |
| 10 | **More Actions** - Screenshot, file operations | Medium |
| 11 | **Performance** - Metrics, tracing, reports | Low |

---

## Full Changelog

See git log for detailed commit history:
```bash
git log v0.0.5..v0.0.6 --oneline
```

**Key commits:**
- feat: Phase 3 - DSL foundation
- feat: Phase 4 - DSL executor
- feat: Phase 5 - External task integration
- feat: Phase 6A - Task validation CLI
- feat: Phase 6B - Hot reload
- feat: Phase 7 - Task parameters
- fix: Broken doc tests

---

*Released with 🦀 Rust 2021 edition*
