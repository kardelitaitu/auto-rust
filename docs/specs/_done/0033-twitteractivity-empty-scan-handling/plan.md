# Implementation Plan

## Overview

Add empty scan detection with consecutive failure tracking in `src/task/twitteractivity.rs` main loop.

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
let mut consecutive_empty_scans = 0u32;  // Track empty scan health
```

### Step 2: Add Empty Scan Tracking

**Location**: Lines 169-211 (after candidate scan, before wait)

**Current code**:
```rust
// Identify candidate tweets
let candidates = identify_engagement_candidates(api).await?;
info!("Candidate scan | candidates={}", candidates.len());
next_candidate_scan = Instant::now() + candidate_scan_interval;

if !candidates.is_empty() {
    let to_consider = candidates
        .iter()
        .take(task_config.candidate_count as usize)
        .collect::<Vec<_>>();
    // ... process candidates
}

// Wait for next scan
if now < next_candidate_scan {
    tokio::time::sleep(next_candidate_scan - now).await;
    continue;
}
```

**New code**:
```rust
// Identify candidate tweets
let candidates = identify_engagement_candidates(api).await?;
info!("Candidate scan | candidates={}", candidates.len());
next_candidate_scan = Instant::now() + candidate_scan_interval;

if !candidates.is_empty() {
    consecutive_empty_scans = 0;  // Reset on success
    
    let to_consider = candidates
        .iter()
        .take(task_config.candidate_count as usize)
        .collect::<Vec<_>>();
    // ... process candidates (existing code)
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

// Wait for next scan
if now < next_candidate_scan {
    tokio::time::sleep(next_candidate_scan - now).await;
    continue;
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

- **False positives**: Single or double empty scan won't break task (threshold = 3)
- **Log noise**: Warning level appropriate, only logs on actual empty scans
- **Threshold tuning**: Can be adjusted in code if 3 is too aggressive (change `>= 3` to `>= 5`, etc.)
- **Still scrolls on empty**: `scroll_read()` still happens regardless of empty scans - page might load content later
