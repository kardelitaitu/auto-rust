# TwitterActivity Scroll Timing Fixes

Status: `approved`

Owner: `spec-agent`

## Summary:

Two related issues in the main feed scanning loop of `src/task/twitteractivity.rs`:
1. **Content load delay missing** after `scroll_read()` - candidate scans run on stale DOM
2. **Initial scroll race** - `next_scroll` initialized to `Instant::now()` causes scroll before first candidate scan

## Issue #1: Content Load Delay Missing After Scroll

### Problem

In `src/task/twitteractivity.rs` lines 162-172, the flow is:

1. `scroll_read(1, scroll_amount, smooth, back_scroll)` - scrolls + built-in 300-900ms human pause
2. `identify_engagement_candidates()` - immediately scans DOM for tweets

The built-in 300-900ms pause in `scroll_read()` is for human behavior simulation, not content loading. The current profile default for `scroll_pause_ms` is 1000ms.

### Solution

**Option A**: Add `api.pause(scroll_pause_ms).await` after `scroll_read()` before `identify_engagement_candidates()`.

- `scroll_pause_ms` comes from `profile.scroll.pause_ms` (current default: 1000ms)
- Gives Twitter a second pause knob without changing the profile model
- Total pause after scroll: ~1300-1900ms with the current default (300-900ms built-in + 1000ms explicit)
- Simple, reliable, uses existing profile configuration

## Issue #2: Race in Initial Scroll Timing

### Problem

In `src/task/twitteractivity.rs` lines 150, 162-167:

```rust
let mut next_scroll = Instant::now();  // Line 150: initialized to NOW
...
if now >= next_scroll {               // First iteration: NOW >= NOW is TRUE
    // SCROLL HAPPENS BEFORE FIRST CANDIDATE SCAN
    let _ = api.scroll_read(...).await;
}
```

The first iteration always scrolls because `next_scroll` equals `Instant::now()`. This means:
- Page just loaded (Phase 1 complete)
- First scroll happens BEFORE scanning initial content
- Initial tweets are missed (scrolled away from)

### Solution

Initialize `next_scroll` to `Instant::now() + scroll_interval` so the first iteration does NOT scroll:

```rust
// BEFORE:
let mut next_scroll = Instant::now();

// AFTER:
let mut next_scroll = Instant::now() + scroll_interval;
```

**Result**:
- First iteration: scans INITIAL content (correct!)
- After `scroll_interval`: scroll + scan cycle begins
- No more wasted scroll on first iteration

## Acceptance Criteria

- [ ] `api.pause(scroll_pause_ms).await;` added after `scroll_read()` call
- [ ] `next_scroll` initialized to `Instant::now() + scroll_interval` (line 150)
- [ ] Pause uses `scroll_pause_ms` variable (not hardcoded value)
- [ ] First candidate scan happens BEFORE first scroll
- [ ] `./check.ps1` passes
- [ ] `cargo clippy` passes with no warnings

## Status

**Approved** - Ready for implementation