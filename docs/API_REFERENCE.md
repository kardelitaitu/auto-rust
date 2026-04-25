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

## Best Practices

1. **Use high-level verbs** - Prefer `api.click()` over low-level mouse utilities
2. **Prefer selectors** - Use CSS selectors over coordinates when possible
3. **Let verbs handle pauses** - Don't duplicate settle pauses; use `api.pause()` only for explicit waits
4. **Keep one interaction path** - Don't switch between `click` and `nativeclick` in the same task step
