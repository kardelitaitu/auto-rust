# Implementation Plan

## Overview

Add scroll error handling with consecutive failure tracking in `src/task/twitteractivity.rs` main loop.

## Steps

### Step 1: Add Counter Variable

**Location**: Near line 132 (with other loop variables)

**Current code**:
```rust
let mut actions_taken = 0u32;
let mut last_remaining;
```

**New code**:
```rust
let mut actions_taken = 0u32;
let mut last_remaining;
let mut consecutive_scroll_failures = 0u32;  // Track scroll health
```

### Step 2: Replace Silent Error Discard with Match

**Location**: Lines 162-167 (scroll block)

**Current code**:
```rust
if now >= next_scroll {
    let _ = api
        .scroll_read(1, scroll_amount, smooth, profile.scroll.back_scroll)
        .await;
    next_scroll = now + scroll_interval;
}
```

**New code**:
```rust
if now >= next_scroll {
    match api
        .scroll_read(1, scroll_amount, smooth, profile.scroll.back_scroll)
        .await
    {
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

### Step 3: Run Checks

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

- `src/task/twitteractivity.rs` (+~10 lines)

## Risks Mitigation

- **False positives**: Single transient failure won't break task (threshold = 3)
- **Log noise**: Warning level appropriate, only logs on actual failures
- **Threshold tuning**: Can be adjusted in code if 3 is too aggressive (change `>= 3` to `>= 5`, etc.)
- **Still scans on failure**: `identify_engagement_candidates()` still runs even if scroll fails - page might still have content
