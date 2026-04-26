# Task Authoring Guide

This guide explains how to author tasks for the Auto-Rust orchestrator framework.

## Overview

Tasks are the building blocks of automation in Auto-Rust. Each task is a Rust function that receives a `TaskContext` and performs browser automation actions.

## Task Structure

### Basic Task Template

```rust
use crate::runtime::task_context::TaskContext;

pub async fn my_task(context: &TaskContext) -> Result<()> {
    // Your task logic here
    Ok(())
}
```

### Task File Location

Tasks should be placed in `src/task/` with the filename matching the task name:
- Task: `my-task` → File: `src/task/my_task.rs`
- Task: `twitterfollow` → File: `src/task/twitterfollow.rs`

## TaskContext API

The `TaskContext` provides access to browser automation capabilities. Always prefer using these methods over direct browser control.

### Navigation Methods

```rust
context.url() -> Result<String>              // Get current URL
context.navigate_to(url: &str) -> Result<()>  // Navigate to URL
```

### Element Interaction Methods

```rust
context.click(selector: &str) -> Result<()>           // Click an element
context.double_click(selector: &str) -> Result<()>     // Double-click
context.right_click(selector: &str) -> Result<()>      // Right-click
context.hover(selector: &str) -> Result<()>           // Hover over element
context.focus(selector: &str) -> Result<()>           // Focus an element
context.clear(selector: &str) -> Result<()>           // Clear input field
context.select_all(selector: &str) -> Result<()>      // Select all text
```

### Keyboard Methods

```rust
context.r#type(selector: &str, text: &str) -> Result<()>  // Type text
context.keyboard(key: &str) -> Result<()>                  // Press key
context.randomcursor() -> Result<()>                      // Random cursor movement
```

### Element Query Methods

```rust
context.exists(selector: &str) -> Result<bool>     // Check if element exists
context.visible(selector: &str) -> Result<bool>    // Check if element visible
context.text(selector: &str) -> Result<String>      // Get element text
context.html(selector: &str) -> Result<String>      // Get element HTML
context.attr(selector: &str, attr: &str) -> Result<String>  // Get attribute
```

### Wait Methods

```rust
context.wait_for(selector: &str) -> Result<()>              // Wait for element to appear
context.wait_for_visible(selector: &str) -> Result<()>      // Wait for element to be visible
context.scroll_to(selector: &str) -> Result<()>           // Scroll to element
```

### Page Methods

```rust
context.title() -> Result<String>  // Get page title
```

## Best Practices

### 1. Use Short Task Verbs

The framework provides short, readable verbs for common actions:

```rust
// Good
context.click(selector)
context.r#type(selector, text)
context.hover(selector)

// Avoid (internal utilities)
context.utils.mouse.click(...)
context.internal.text.truncate(...)
```

### 2. Prefer Selector-Based Interaction

Always use selectors for element interaction rather than coordinates:

```rust
// Good
context.click("#submit-button")?

// Avoid (coordinate-based)
context.utils.mouse.click_at(x, y)?
```

### 3. Use Deterministic Verification

Verify the same target element that was clicked or inspected:

```rust
// Good
context.click("#button")?;
context.wait_for_visible("#button")?;

// Avoid
context.click("#button")?;
context.wait_for_visible(".button")?;  // Different selector
```

### 4. Keep Task Names Canonical

Use kebab-case for task names (e.g., `twitter-follow`, `page-view`).

### 5. Add Task Validation

Define validation rules in `src/validation/task.rs`:

```rust
impl TaskPayload {
    fn validate_my_task(&self) -> Result<()> {
        if !self.payload.is_object() {
            return Err(TaskError::ValidationFailed {
                task_name: "my-task".to_string(),
                reason: "payload must be an object".to_string(),
            });
        }
        // Additional validation logic
        Ok(())
    }
}
```

## Example Tasks

### Simple Click Task

```rust
use crate::runtime::task_context::TaskContext;

pub async fn demo_click(context: &TaskContext) -> Result<()> {
    context.click("#demo-button")?;
    Ok(())
}
```

### Form Filling Task

```rust
use crate::runtime::task_context::TaskContext;

pub async fn fill_form(context: &TaskContext) -> Result<()> {
    context.r#type("#username", "my_username")?;
    context.r#type("#password", "my_password")?;
    context.click("#submit")?;
    Ok(())
}
```

### Navigation Task

```rust
use crate::runtime::task_context::TaskContext;

pub async fn pageview(context: &TaskContext) -> Result<()> {
    let url = "https://example.com";
    context.navigate_to(url)?;
    context.wait_for_visible("body")?;
    Ok(())
}
```

## Task Payloads

Tasks can accept JSON payloads for configuration:

```json
{
  "task": "my-task",
  "payload": {
    "url": "https://example.com",
    "timeout": 5000
  }
}
```

Access payload values in your task:

```rust
pub async fn my_task(context: &TaskContext) -> Result<()> {
    // Access payload through context if needed
    // (Note: payload access pattern depends on task implementation)
    Ok(())
}
```

## Error Handling

Use the structured error types from `crate::error`:

```rust
use crate::error::{TaskError, Result};

pub async fn my_task(context: &TaskContext) -> Result<()> {
    context.click("#button").map_err(|e| {
        TaskError::ExecutionFailed {
            task_name: "my-task".to_string(),
            reason: format!("Failed to click button: {}", e),
        }
    })?;
    Ok(())
}
```

## Task Policy System

All tasks run under a `TaskPolicy` that defines execution constraints and permissions. The policy system ensures tasks cannot exceed their granted capabilities.

### Policy Basics

Every task has a policy that specifies:
- **Timeout**: Maximum execution time (`max_duration_ms`)
- **Permissions**: What capabilities the task can use

```rust
use auto::task::policy::get_policy;

// Get the policy for a task
let policy = get_policy("twitteractivity");
println!("Timeout: {}ms", policy.max_duration_ms);
println!("Allow screenshot: {}", policy.permissions.allow_screenshot);
```

### Available Permissions

| Permission | Purpose | Used By |
|------------|---------|---------|
| `allow_screenshot` | Capture page screenshots (auto-saved as JPG) | `context.screenshot()` |
| `allow_export_cookies` | Read browser cookies | `context.export_cookies()` |
| `allow_import_cookies` | Set browser cookies | `context.import_cookies()` |
| `allow_export_session` | Export cookies + localStorage | `context.export_session()` |
| `allow_import_session` | Import cookies + localStorage | `context.import_session()` |
| `allow_session_clipboard` | Read/write session clipboard | `context.read_clipboard()`, `write_clipboard()` |
| `allow_read_data` | Read files from config/data | `context.read_data_file()` |
| `allow_write_data` | Write files to config/data | `context.write_clipboard()` |

### Permission Implications

Some permissions automatically grant related permissions:
- `allow_screenshot` → `allow_write_data` (screenshots must be saved)
- `allow_export_session` → `allow_export_cookies` (uses same CDP call)
- `allow_import_session` → `allow_import_cookies` (uses same CDP call)

### Policy-Required Operations

The following operations require explicit permissions:

```rust
// Requires allow_screenshot
// Returns file path: "data/screenshot/2026-04-26-15-30-session-123.jpg"
let screenshot_path = context.screenshot().await?;

// Requires allow_export_cookies
let cookies = context.export_cookies("").await?;

// Requires allow_session_clipboard
let text = context.read_clipboard()?;
context.write_clipboard("copied text")?;

// Requires allow_read_data
let content = context.read_data_file("personas/bot.json")?;

// Requires allow_write_data (implied by allow_screenshot)
context.write_data_file("output.txt", b"data")?;
```

### Handling Permission Denied

When a task attempts an operation without the required permission:

```rust
use crate::error::TaskError;

match context.screenshot().await {
    Ok(data) => Ok(data),
    Err(e) if e.to_string().contains("Permission denied") => {
        // Log the denial and continue without screenshot
        log::warn!("Screenshot permission denied, skipping");
        Ok(vec![])
    }
    Err(e) => Err(e),
}
```

### Timeout Enforcement

Tasks exceeding their `max_duration_ms` are automatically terminated:

```rust
// twitteractivity policy: 300_000ms (5 minutes)
// If this task runs longer, it receives TaskErrorKind::Timeout
pub async fn twitteractivity(context: &TaskContext) -> Result<()> {
    // Long-running feed scanning
    loop {
        // ... process tweets ...
        // Killed automatically if exceeds 5 minutes
    }
}
```

### Default Policy (Safe Defaults)

New tasks use `DEFAULT_TASK_POLICY` unless explicitly configured:
- 60 second timeout
- All permissions = false

```rust
use auto::task::policy::DEFAULT_TASK_POLICY;

let api = TaskContext::new(
    "session-1",
    page,
    profile,
    runtime,
    native_interaction,
    &DEFAULT_TASK_POLICY, // Safe defaults
);
```

### Creating Custom Policies

For one-off tasks with different constraints:

```rust
use auto::task::policy::{TaskPolicy, TaskPermissions};

let custom_policy = TaskPolicy {
    max_duration_ms: 120_000, // 2 minutes
    permissions: TaskPermissions {
        allow_screenshot: true,
        allow_export_cookies: true,
        ..Default::default()
    },
};
```

### Policy Registry

15 built-in policies are registered by task name:
- `COOKIEBOT_POLICY` (30s) - Cookie consent tasks
- `PAGEVIEW_POLICY` (30s) - Page verification tasks
- `TWITTERACTIVITY_POLICY` (5min) - Feed scanning with full permissions
- `TWITTER_*_POLICY` (45s) - Individual Twitter actions
- `DEMO_*_POLICY` (60s) - Demo tasks with no permissions

Access via `get_policy(task_name)` or use default.

## Testing Tasks

Add tests to verify task behavior:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_validation() {
        // Test payload validation
        assert!(validate_task("my-task", json!({})).is_ok());
    }
}
```

## Common Patterns

### Retry Pattern

The orchestrator automatically handles retries based on config. Tasks should focus on single attempts.

### Timeout Pattern

Use `wait_for_visible` with implicit timeouts from config:

```rust
context.wait_for_visible("#element")?;  // Uses configured timeout
```

### Scroll Pattern

For pages with lazy-loaded content:

```rust
context.scroll_to("footer")?;
context.wait_for_visible("#load-more")?;
```

## Adding New Tasks

1. Create file: `src/task/my_task.rs`
2. Implement task function with `TaskContext`
3. Add validation in `src/validation/task.rs` if needed
4. Register task in CLI if needed (see `src/cli.rs`)

## See Also

- [API Reference](#) - Full TaskContext API documentation
- [Configuration](#) - Config file structure and validation
- [Examples](#) - Example tasks in `src/task/`
