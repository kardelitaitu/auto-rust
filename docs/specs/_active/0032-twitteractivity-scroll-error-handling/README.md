# TwitterActivity Scroll Error Handling

Status: `approved`
Owner: `spec-agent`

## Dependency:

Implement after `0031-twitteractivity-content-load-delay`. This package assumes the initial-scan timing fix and post-scroll pause already exist.

## Summary:

Scroll errors in the main feed scanning loop are silently ignored via `let _ = api.scroll_read(...)`. If scrolling fails repeatedly (e.g., page navigated away, browser disconnected), the loop continues indefinitely without making any engagements, wasting the entire task duration.

## Problem:

In `src/task/twitteractivity.rs` lines 162-167:

```rust
if now >= next_scroll {
    let _ = api  // ← Result<()> discarded!
        .scroll_read(1, scroll_amount, smooth, profile.scroll.back_scroll)
        .await;
    next_scroll = now + scroll_interval;
}
```

**Issues**:
1. `scroll_read()` returns `Result<()>` - errors are completely silenced
2. No tracking of failure count - transient vs. persistent failures indistinguishable
3. Task continues even if page is clearly broken (3+ consecutive failures)
4. No visibility into scroll health during task execution

## Solution: Option B (Track & Break)

Add consecutive failure tracking and break after threshold:

```rust
// Near variable declarations (around line 132)
let mut consecutive_scroll_failures = 0u32;

// In the scroll block (lines 162-167)
if now >= next_scroll {
    match api.scroll_read(1, scroll_amount, smooth, profile.scroll.back_scroll).await {
        Ok(()) => {
            consecutive_scroll_failures = 0;
        }
        Err(e) => {
            consecutive_scroll_failures += 1;
            log::warn!(
                "[twitter] Scroll failed (attempt {}): {}",
                consecutive_scroll_failures, e
            );
            if consecutive_scroll_failures >= 3 {
                log::error!("[twitter] Too many consecutive scroll failures, stopping task");
                break;
            }
        }
    }
    next_scroll = now + scroll_interval;
}
```

**Why Option B**:
- **Balanced**: Doesn't overreact to single transient failure
- **Visible**: Logs warnings so operators can see issues
- **Protective**: Stops task when page is clearly broken (>3 failures)
- **Simple**: Only ~10 lines added

## Acceptance Criteria:

- [ ] `consecutive_scroll_failures` counter added (initialized to 0)
- [ ] `match` expression replaces `let _ = ...`
- [ ] On `Ok(())`: reset counter to 0
- [ ] On `Err(e)`: increment counter, log warning with attempt number
- [ ] After 3 consecutive failures: log error and `break`
- [ ] `./check.ps1` passes
- [ ] `cargo clippy` passes with no warnings

## Status:

**Approved** - Ready for implementation