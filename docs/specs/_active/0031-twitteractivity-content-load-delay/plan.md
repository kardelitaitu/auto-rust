# Implementation Plan

## Overview

Add content load delay after scroll and fix initial scroll timing in `src/task/twitteractivity.rs` main loop.

## Steps

### Step 1: Fix Initial Scroll Timing (Issue #2)

**Location**: Line 150 in `run_inner()`

**Current code**:
```rust
let mut next_scroll = Instant::now();
let mut next_candidate_scan = Instant::now();
```

**New code**:
```rust
let mut next_scroll = Instant::now() + scroll_interval;  // Don't scroll on first iteration
let mut next_candidate_scan = Instant::now();
```

**Why**: Ensures first iteration scans initial content BEFORE any scrolling happens.

### Step 2: Add Content Load Delay After Scroll (Issue #1)

**Location**: Lines 162-172 (scroll + candidate scan section)

**Current code**:
```rust
if now >= next_scroll {
    let _ = api
        .scroll_read(1, scroll_amount, smooth, profile.scroll.back_scroll)
        .await;
    next_scroll = now + scroll_interval;
}

// Identify candidate tweets
let candidates = identify_engagement_candidates(api).await?;
```

**New code**:
```rust
if now >= next_scroll {
    let _ = api
        .scroll_read(1, scroll_amount, smooth, profile.scroll.back_scroll)
        .await;
    next_scroll = now + scroll_interval;

    // Wait for Twitter to lazy-load new content after scroll
    api.pause(scroll_pause_ms).await;
}

// Identify candidate tweets
let candidates = identify_engagement_candidates(api).await?;
```

### Step 3: Verify Variables Are Available in Scope

- `scroll_pause_ms` is already defined at line 140:
```rust
let scroll_pause_ms = profile.scroll.pause_ms;
```

- `scroll_interval` is already defined at line 142:
```rust
let scroll_interval = Duration::from_millis(scroll_pause_ms);
```

No additional imports or variable definitions needed.

### Step 4: Run checks

```powershell
./check.ps1
```

Expected results:
- Repo CI gate passes
- Spec lint: PASS
- Build: PASS
- Format: PASS
- Clippy: PASS
- Tests: PASS

## Files Modified

- `src/task/twitteractivity.rs` (+2 lines: 1 for pause, 1 for next_scroll init)

## Risks Mitigation

- **Double pause**: The built-in 300-900ms in `scroll_read()` plus new `scroll_pause_ms` (2000-5000ms) is intentional - one for human behavior, one for content loading
- **Timing**: If `scroll_pause_ms` is too short, can be adjusted in behavior profile configuration
- **Initial scan**: Fix ensures initial content is scanned before any scrolling occurs
