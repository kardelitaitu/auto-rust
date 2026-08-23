last audited 24-06-26 by Buffy
# API Reference

Complete reference for the TaskContext API and interaction patterns.

## TaskContext

The primary API surface for browser automation tasks. Accessed as `api` in task functions:

```rust
pub async fn my_task(api: &TaskContext, payload: Value) -> anyhow::Result<()> {
    api.navigate("https://example.com", 30000).await?;
    api.click("button").await?;
    Ok(())
}
```

## Navigation

```rust
api.navigate(url: &str, timeout_ms: u64) -> Result<()>
api.url() -> Result<String>
api.title() -> Result<String>
```

> **DSL equivalent:** [`navigate` action](TASKS/dsl.md#navigate) in declarative task files.

### api.iframe(selector, timeout_ms)

"Enter" an iframe by navigating the tab directly to its `src` URL.

Cross-origin iframes (e.g. Telegram mini-apps) cannot be reached by selectors
from the top document. Reading the iframe's `src` attribute is allowed
cross-origin, so this navigates to it, making the frame's content the top
document where normal selectors and clicks work. Returns the entered URL.

```rust
let miniapp_url = api.iframe("iframe", 30_000).await?;
```

### api.iframe_click(iframe_selector, element_selector, timeout_ms)

Click an element **inside** an iframe, in place — no navigation and no new tab.

Cross-origin iframes (e.g. Telegram mini-apps) cannot be reached by selectors
from the top document. This method resolves the iframe's own execution context
via CDP (`Page.getFrameTree` + `Runtime.evaluate` with the frame's context),
finds the element inside the frame, computes its absolute viewport position,
then dispatches a cursor-simulating click (CDP `Input` pointer + mouse events,
which are browser-level and work across origins).

```rust
let outcome = api.iframe_click("iframe", "#btn-go", 30_000).await?;
```

## Clicking

### api.click(selector)

Browser-level click via CDP (Chromium DevTools Protocol):

```rust
api.click("button.submit").await?;
api.click_and_wait("button", ".success").await?;
```

**Pipeline:** scroll into view → move cursor → pointerenter → pointerdown → 80ms press → pointerup → pointerleave

### api.nativeclick(selector)

OS-level native click via `enigo`:

```rust
api.nativeclick("button.submit").await?;
```

**Pipeline:** scroll into view → native move → native click → verification

**Failure semantics:**

| Error | Cause | Retry Strategy |
|-------|-------|----------------|
| `stable` | Target didn't stabilize | Retry once after increasing `NATIVE_INTERACTION_STABILITY_WAIT_MS` |
| `clickable` | Target not clickable (hidden/disabled/obscured) | Retry once after waiting for overlay dismissal |
| `mapping` | Content-to-screen mapping failed | Do not blind-retry; recalibrate session |
| `verify` | Post-click verification failed | Retry once only if page mutates on click |

**When to use which:**

- **Default:** `api.nativeclick()` for reliable OS-level interaction
- **Fallback:** `api.click()` when native input unavailable or page blocks native assumptions
- **Never:** Silently switch between click types in the same step

> **DSL equivalent actions:** [`click`](TASKS/dsl.md#click), [`double_click`](TASKS/dsl.md#doubleclick), [`right_click`](TASKS/dsl.md#rightclick) in declarative task files.

## Mouse Interactions

```rust
api.click(selector: &str) -> Result<ClickOutcome>
api.nativeclick(selector: &str) -> Result<ClickOutcome>
api.click_and_wait(click_selector: &str, wait_selector: &str) -> Result<ClickOutcome>
api.double_click(selector: &str) -> Result<ClickOutcome>
api.middle_click(selector: &str) -> Result<ClickOutcome>
api.right_click(selector: &str) -> Result<ClickOutcome>
api.drag(from: &str, to: &str) -> Result<ClickOutcome>
api.hover(selector: &str) -> Result<ClickOutcome>
```

> **DSL equivalent:** [`hover` action](TASKS/dsl.md#hover). The other mouse methods (`nativeclick`, `click_and_wait`, `middle_click`, `drag`) are programmatic-only with no direct DSL equivalent.

> **Tip:** To emulate `click_and_wait` in DSL, compose a [`click`](TASKS/dsl.md#click) action followed by a [`wait_for`](TASKS/dsl.md#waitfor).

## Native Cursor

> **DSL equivalent:** No direct DSL equivalent — cursor positioning is a programmatic-only operation.

```rust
api.nativecursor(x: i32, y: i32) -> Result<()>
api.nativecursor_query(query: &str, timeout_ms: u64) -> Result<()>
api.nativecursor_selector(selector: &str) -> Result<()>
api.randomcursor() -> Result<()>
```

## Keyboard Input

> **DSL equivalent:** No direct DSL equivalent — `select_all()` is a programmatic-only helper.

```rust
api.keyboard(selector: &str, text: &str) -> Result<()>
api.clear(selector: &str) -> Result<()>
api.select_all(selector: &str) -> Result<()>
```

Note: `api.r#type()` is available as the Rust-keyword-safe alias.

> **DSL equivalent actions:** [`type`](TASKS/dsl.md#type), [`clear`](TASKS/dsl.md#clear), [`press`](TASKS/dsl.md#press) in declarative task files.

## Element Queries

```rust
api.exists(selector: &str) -> Result<bool>
api.visible(selector: &str) -> Result<bool>
api.text(selector: &str) -> Result<String>
api.html(selector: &str) -> Result<String>
api.attr(selector: &str, attribute: &str) -> Result<String>
```

> **DSL equivalent:** [`extract` action](TASKS/dsl.md#extract) reads element text using the browser API. The `element_exists` condition in [If/Else](TASKS/dsl.md#ifelse) and [While](TASKS/dsl.md#while) actions uses [`api.exists()`](#element-queries). The `visible` method is available in DSL as the [`element_visible` condition](TASKS/dsl.md#element-conditions). Queries `html()` and `attr()` are programmatic-only with no DSL equivalent.

## Waiting

```rust
api.wait_for(selector: &str) -> Result<()>
api.wait_for_visible(selector: &str) -> Result<()>
api.pause(base_ms: u64) -> ()
```

> **DSL equivalent actions:** [`wait`](TASKS/dsl.md#wait) (timed pause) and [`wait_for`](TASKS/dsl.md#waitfor) (selector-based wait) in declarative task files.

> **Note:** `wait_for_visible()` has no direct DSL equivalent — the DSL [`wait_for`](TASKS/dsl.md#waitfor) checks element existence only. For visibility checks, compose [`wait_for`](TASKS/dsl.md#waitfor) with an [`element_visible` condition](TASKS/dsl.md#element-conditions) in a [While](TASKS/dsl.md#while) loop.

**Timing:** `api.pause()` uses uniform 20% deviation. High-level verbs include post-action settle pause automatically.

## Scrolling

```rust
api.scroll_to(selector: &str) -> Result<()>
```

> **DSL equivalent:** [`scroll_to` action](TASKS/dsl.md#scrollto) in declarative task files.

## Focus

```rust
api.focus(selector: &str) -> Result<()>
```

> **DSL equivalent:** No direct DSL equivalent — `focus()` is a low-level DOM operation available only through the programmatic API. For clicking/interacting with elements in DSL, use [`click`](TASKS/dsl.md#click) or [`type`](TASKS/dsl.md#type), which focus the element automatically.

## Configuration

### LLM Configuration

Environment variables for the LLM client (read from `config/llm.toml` with env overrides):

| Variable | Default | Description |
|----------|---------|-------------|
| `LLM_PROVIDER` | `ollama` | LLM provider to use (`ollama`, `nvidia`, or `openrouter`) |
| `OLLAMA_URL` | `http://localhost:11434` | Ollama API base URL |
| `OLLAMA_MODEL` | `llama3` | Ollama model name |
| `OPENROUTER_API_KEY` | — | OpenRouter API key (required for OpenRouter provider) |
| `OPENROUTER_MODEL` | — | Primary OpenRouter model (e.g., `tencent/hy3-preview:free`) |
| `OPENROUTER_MODEL_FALLBACK` | — | First fallback model |
| `OPENROUTER_MODEL_FALLBACK_2` | — | Second fallback model |
| `OPENROUTER_MODEL_FALLBACK_3` | — | Third fallback model |
| `OPENROUTER_MODEL_FALLBACK_4` | — | Fourth fallback model |

> **Note:** For NVIDIA env vars (`NVIDIA_API_KEY`, `NVIDIA_MODEL`, `NVIDIA_BASE_URL`, etc.), see [.bacon/workflow.md](.bacon/workflow.md).

### Native Interaction

Environment variables for native interaction:

| Variable | Default | Description |
|----------|---------|-------------|
| `NATIVE_INPUT_BACKEND` | `enigo` | Backend for native input (`enigo` only for now) |
| `NATIVE_INTERACTION_STABILITY_WAIT_MS` | 1000 | Wait for element stabilization |

### Cursor Overlay

Environment variables for the visual cursor overlay (injected into browser pages):

| Variable | Default | Description |
|----------|---------|-------------|
| `CURSOR_OVERLAY_MS` | `0` | Sync interval in ms (`0` = disabled, `50` = ~20fps) |
| `CURSOR_OVERLAY_COLOR` | `#ff6600` | Accent color for cursor dot, ring, and ripple (CSS hex: `#rgb`, `#rrggbb`, `#rgba`, `#rrggbbaa`) |
| `CURSOR_OVERLAY_SHOW_TRAIL` | `true` | Show/hide fading ghost trail dots behind the cursor |

**Validation:** `cursor_overlay_color` is validated as a hex color at startup when `cursor_overlay_ms > 0`. Invalid formats produce a validation warning.

### Twitter Activity — Consecutive Failure Thresholds

Config fields that control when the feed loop stops on repeated failures:

**TOML (`config/default.toml`):**
```toml
[twitter_activity]
# Maximum consecutive scroll failures before stopping the feed loop (default: 3)
max_consecutive_scroll_failures = 3
# Maximum consecutive empty candidate scans before stopping the feed loop (default: 3)
max_consecutive_empty_scans = 3
```

**Environment variable overrides:**

| Variable | Default | Description |
|----------|---------|-------------|
| `TWITTER_MAX_CONSECUTIVE_SCROLL_FAILURES` | `3` | Override max consecutive scroll failures |
| `TWITTER_MAX_CONSECUTIVE_EMPTY_SCANS` | `3` | Override max consecutive empty scans |

**Validation rules** (applied at startup via `validate_config`):

| Condition | Severity | Message |
|-----------|----------|---------|
| Value = 0 | Warning | The feed loop will stop on the first failure. Consider setting >= 1. |
| Value > 20 | Warning | Very high threshold — may cause excessive retries or extended loops. |
| Invalid env var (non-numeric) | Silently ignored | Falls back to config/default value. |

## Logging

Native clicks log in format:
```
clicked (<selector>) at x,y
```

## v0.1.1 API Reference

26 new APIs with permission-based security. All APIs require explicit `TaskPolicy` permissions - see [Task Policy Documentation](TASK_POLICY_IMPLEMENTED.md).

### Cookie Management

**Requires:** `allow_export_cookies` permission

```rust
// Export cookies for specific domain
api.export_cookies_for_domain(domain: &str) -> Result<Vec<serde_json::Value>>

// Export only session (non-persistent) cookies
api.export_session_cookies(url: &str) -> Result<Vec<serde_json::Value>>

// Check if specific cookie exists
api.has_cookie(name: &str, domain: Option<&str>) -> Result<bool>
```

### Session Management

**Requires:** `allow_export_session` / `allow_import_session` permissions

```rust
// Export localStorage data
api.export_local_storage(url: &str) -> Result<HashMap<String, String>>

// Import localStorage data
api.import_local_storage(url: &str, data: &HashMap<String, String>) -> Result<()>

// Validate session data before import
api.validate_session_data(data: &SessionData) -> Result<Vec<String>>
```

### Clipboard Management

**Requires:** `allow_session_clipboard` permission

```rust
// Clear clipboard content
api.clear_clipboard() -> Result<()>

// Check if clipboard has content
api.has_clipboard_content() -> Result<bool>

// Append text to existing clipboard content
api.append_clipboard(text: &str, separator: Option<&str>) -> Result<()>
```

### Data File Management

**Requires:** `allow_read_data` / `allow_write_data` permissions

```rust
// List files in data directory
api.list_data_files(subdir: Option<&str>) -> Result<Vec<String>>

// Check if file exists
api.data_file_exists(relative_path: &str) -> Result<bool>

// Delete data file
api.delete_data_file(relative_path: &str) -> Result<()>

// Read and parse JSON file
api.read_json_data<T>(relative_path: &str) -> Result<T>

// Write data as JSON
api.write_json_data<T>(relative_path: &str, data: &T) -> Result<()>

// Append bytes to file
api.append_data_file(relative_path: &str, content: &[u8]) -> Result<()>

// Get file metadata
api.data_file_metadata(relative_path: &str) -> Result<FileMetadata>
```

### Network / HTTP

**Requires:** `allow_http_requests` permission

```rust
// HTTP GET request
api.http_get(url: &str) -> Result<HttpResponse>

// HTTP POST with JSON body
api.http_post_json<T>(url: &str, body: &T) -> Result<HttpResponse>

// Download file from URL
api.download_file(url: &str, relative_path: &str) -> Result<u64>
```

**HttpResponse struct:**
```rust
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}
```

### DOM Inspection

**Requires:** `allow_dom_inspection` permission

```rust
// Get computed CSS style property
api.get_computed_style(selector: &str, property: &str) -> Result<String>

// Get element position and size
api.get_element_rect(selector: &str) -> Result<Rect>

// Get page scroll position
api.get_scroll_position() -> Result<(u32, u32)>

// Count elements matching selector
api.count_elements(selector: &str) -> Result<usize>

// Check if element is in viewport
api.is_in_viewport(selector: &str) -> Result<bool>
```

**Rect struct:**
```rust
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
```

### Browser Management

**Requires:** `allow_browser_export` / `allow_browser_import` permissions

```rust
// Export complete browser state (cookies, localStorage, sessionStorage, IndexedDB)
api.export_browser(url: &str) -> Result<BrowserData>

// Import complete browser state
api.import_browser(data: &BrowserData) -> Result<()>
```

**BrowserData struct:**
```rust
pub struct BrowserData {
    pub cookies: Vec<serde_json::Value>,
    pub local_storage: HashMap<String, HashMap<String, String>>,
    pub session_storage: HashMap<String, HashMap<String, String>>,
    pub indexeddb_names: HashMap<String, Vec<String>>,
    pub exported_at: DateTime<Utc>,
    pub source: String,
    pub browser_version: Option<String>,
}
```

### Permission Requirements Summary

| API Category | Read Permission | Write Permission |
|--------------|-----------------|------------------|
| Cookie Management | `allow_export_cookies` | `allow_import_cookies` |
| Session Management | `allow_export_session` | `allow_import_session` |
| Clipboard | - | `allow_session_clipboard` |
| Data File | `allow_read_data` | `allow_write_data` |
| Network/HTTP | - | `allow_http_requests` |
| DOM Inspection | `allow_dom_inspection` | - |
| Browser Mgmt | `allow_browser_export` | `allow_browser_import` |

> **Note:** All v0.1.1 APIs (Cookie, Session, Clipboard, Data, Network, DOM, Browser) are programmatic-only and have no DSL equivalents.

## DSL Executor APIs

The `DslExecutor` provides programmatic control over declarative task execution.

### Basic Usage

```rust
use auto::task::dsl::{DslExecutor, TaskDefinition};

let task_def = TaskDefinition::from_yaml_file("tasks/my_task.yaml")?;
let mut executor = DslExecutor::new(api, &task_def);
executor.execute().await?;
```

### Debugging APIs

#### Enable Debug Mode
```rust
pub fn with_debug_mode(mut self, enabled: bool) -> Self
```

#### Add Breakpoints
```rust
use auto::task::dsl::Breakpoint;

// Break at specific action index
executor.add_breakpoint(Breakpoint::on_action(5));

// Break on action type
executor.add_breakpoint(Breakpoint::on_action_type("click"));

// Break on variable change
executor.add_breakpoint(Breakpoint::watch_variable("user_id"));

// Break with custom condition
executor.add_breakpoint(
    Breakpoint::on_action_type("extract")
        .with_condition(|vars| vars.get("status") == Some(&"error".to_string()))
);
```

#### Execution Control
```rust
// Check if paused
pub fn is_paused(&self) -> bool

// Resume execution
pub fn resume(&mut self)

// Step through (execute one action, then pause)
pub fn step(&mut self)
```

#### Inspect State
```rust
// Get execution state as JSON
pub fn inspect_state(&self) -> serde_json::Value

// Get debug event log
pub fn get_debug_events(&self) -> &[DebugEvent]
```

### Performance APIs

#### Cache Control
```rust
// Enable selector caching (enabled by default)
pub fn enable_caching(&mut self)

// Disable caching and clear cache
pub fn disable_caching(&mut self)

// Clear cache without disabling
pub fn clear_cache(&mut self)

// Get cache statistics
pub fn get_cache_stats(&self) -> CacheStats
```

**CacheStats struct:**
```rust
pub struct CacheStats {
    pub size: usize,           // Current cache size
    pub hits: u64,             // Cache hit count
    pub misses: u64,           // Cache miss count
    pub evictions: u64,        // Total evictions
    pub hit_rate: f64,         // Hit rate (0.0 - 1.0)
}
```

#### Action Profiling
```rust
// Get profiling statistics
pub fn get_profiler_stats(&self) -> HashMap<String, serde_json::Value>
```

**Example profiler output:**
```json
{
  "click": {
    "action_type": "click",
    "total_executions": 42,
    "total_duration_ms": 8400,
    "average_duration_ms": 200,
    "min_duration_ms": 150,
    "max_duration_ms": 350,
    "failures": 2
  }
}
```

### Execution Report

```rust
// Get execution report (after execute())
let report = executor.execution_report(true);
println!("{}", report.summary());
// Task 'login' executed 5 actions in 1.23s (4 successful, 1 failed)

// Export to JSON
let json = report.to_json();
```

**ExecutionReport fields:**
```rust
pub struct ExecutionReport {
    pub task_name: String,
    pub total_actions: u32,
    pub actions_executed: u32,
    pub actions_succeeded: u32,
    pub actions_failed: u32,
    pub max_call_depth: u32,
    pub variables_defined: usize,
    pub success: bool,
    pub action_metrics: Vec<ActionMetrics>,
}
```

> **See also:** The [DSL Action Reference](TASKS/dsl.md#action-reference) documents all available action types, their fields, and behavior for declarative task files.

## Best Practices

1. **Use high-level verbs** - Prefer `api.click()` over low-level mouse utilities
2. **Prefer selectors** - Use CSS selectors over coordinates when possible
3. **Let verbs handle pauses** - Don't duplicate settle pauses; use `api.pause()` only for explicit waits
4. **Keep one interaction path** - Don't switch between `click` and `nativeclick` in the same task step

> **See also:** The [DSL task tutorial](TASKS/dsl.md#tutorial-building-a-custom-task-from-scratch) walks through building a complete declarative task from scratch, and the [end-to-end example](TASKS/dsl.md#end-to-end-example-login-flow) shows a login flow combining multiple actions.
