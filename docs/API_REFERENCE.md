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

## Mouse Interactions

```rust
api.click(selector: &str) -> Result<()>
api.nativeclick(selector: &str) -> Result<()>
api.click_and_wait(click_selector: &str, wait_selector: &str) -> Result<()>
api.double_click(selector: &str) -> Result<()>
api.middle_click(selector: &str) -> Result<()>
api.right_click(selector: &str) -> Result<()>
api.drag(from: &str, to: &str) -> Result<()>
api.hover(selector: &str) -> Result<()>
```

## Native Cursor

```rust
api.nativecursor(x: i32, y: i32) -> Result<()>
api.nativecursor_query(query: &str, timeout_ms: u64) -> Result<()>
api.nativecursor_selector(selector: &str) -> Result<()>
api.randomcursor() -> Result<()>
```

## Keyboard Input

```rust
api.keyboard(selector: &str, text: &str) -> Result<()>
api.clear(selector: &str) -> Result<()>
api.select_all(selector: &str) -> Result<()>
```

Note: `api.r#type()` is available as the Rust-keyword-safe alias.

## Element Queries

```rust
api.exists(selector: &str) -> Result<bool>
api.visible(selector: &str) -> Result<bool>
api.text(selector: &str) -> Result<String>
api.html(selector: &str) -> Result<String>
api.attr(selector: &str, attribute: &str) -> Result<String>
```

## Waiting

```rust
api.wait_for(selector: &str) -> Result<()>
api.wait_for_visible(selector: &str) -> Result<()>
api.pause(base_ms: u64) -> ()
```

**Timing:** `api.pause()` uses uniform 20% deviation. High-level verbs include post-action settle pause automatically.

## Scrolling

```rust
api.scroll_to(selector: &str) -> Result<()>
```

## Focus

```rust
api.focus(selector: &str) -> Result<()>
```

## Configuration

Environment variables for native interaction:

| Variable | Default | Description |
|----------|---------|-------------|
| `NATIVE_INPUT_BACKEND` | `enigo` | Backend for native input (`enigo` only for now) |
| `NATIVE_INTERACTION_STABILITY_WAIT_MS` | 1000 | Wait for element stabilization |

## Logging

Native clicks log in format:
```
clicked (<selector>) at x,y
```

## v0.0.3 API Reference

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

## Best Practices

1. **Use high-level verbs** - Prefer `api.click()` over low-level mouse utilities
2. **Prefer selectors** - Use CSS selectors over coordinates when possible
3. **Let verbs handle pauses** - Don't duplicate settle pauses; use `api.pause()` only for explicit waits
4. **Keep one interaction path** - Don't switch between `click` and `nativeclick` in the same task step
