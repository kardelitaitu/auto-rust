# TwitterActivity Empty Scan Early Exit

Status: `approved`
Owner: `spec-agent`

## Dependency:

Implement after `0031-twitteractivity-content-load-delay`. This package assumes the revised loop order already exists so empty-scan handling sits on the updated cadence.

## Summary:

If `identify_engagement_candidates()` returns empty repeatedly, the loop waits `candidate_scan_interval` each time with no backoff or shortcut. The task burns its full duration with zero engagements when the feed is empty or broken.

## Problem:

In `src/task/twitteractivity.rs` lines 169-215:

```rust
// Identify candidate tweets
let candidates = identify_engagement_candidates(api).await?;
info!("Candidate scan | candidates={}", candidates.len());
next_candidate_scan = Instant::now() + candidate_scan_interval;

if !candidates.is_empty() {
    // Process candidates...
}

// Wait for next scan time
if now < next_candidate_scan {
    tokio::time::sleep(next_candidate_scan - now).await;
    continue;
}
```

**Issues**:
1. `candidate_scan_interval` is **2500ms** (2.5s) from `MIN_CANDIDATE_SCAN_INTERVAL_MS`
2. If feed is empty/broken, loops every 2.5s doing nothing
3. No tracking of consecutive empty scans
4. No early exit - runs until `session.is_expired()` (wastes full duration)

### Scenario: Broken Feed

```
T=0s:    Scan → 0 candidates
T=2.5s:  Scan → 0 candidates  
T=5.0s:  Scan → 0 candidates
T=7.5s:  Scan → 0 candidates
...
T=120s:  Task timeout, 0 engagements happened
```

Wasted: **120 seconds** doing nothing.

## Solution: Option A (Track & Break)

Add consecutive empty scan tracking and break after threshold:

```rust
// Near variable declarations (around line 132)
let mut consecutive_empty_scans = 0u32;

// After candidate scan (lines 169-173)
let candidates = identify_engagement_candidates(api).await?;
info!("Candidate scan | candidates={}", candidates.len());
next_candidate_scan = Instant::now() + candidate_scan_interval;

if !candidates.is_empty() {
    consecutive_empty_scans = 0;  // Reset on success
    // Process candidates...
} else {
    consecutive_empty_scans += 1;
    log::warn!(
        "[twitter] No candidates found (attempt {})",
        consecutive_empty_scans
    );
    if consecutive_empty_scans >= 3 {
        log::error!("[twitter] Too many empty scans, stopping task");
        break;
    }
}
```

**Why Option A**:
- **Balanced**: Doesn't overreact to single empty scan (threshold = 3)
- **Visible**: Logs warnings so operators can see issues
- **Protective**: Stops task when feed is clearly broken (>3 empty)
- **Simple**: Only ~10 lines added

## Acceptance Criteria:

- [ ] `consecutive_empty_scans` counter added (initialized to 0)
- [ ] On non-empty scan: reset counter to 0
- [ ] On empty scan: increment counter, log warning with attempt number
- [ ] After 3+ consecutive empty scans: log error and `break`
- [ ] `./check.ps1` passes
- [ ] `cargo clippy` passes with no warnings

## Status:

**Approved** - Ready for implementation