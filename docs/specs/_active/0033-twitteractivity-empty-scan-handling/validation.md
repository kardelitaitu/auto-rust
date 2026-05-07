# Validation Checklist

## Automated Checks

Run the repo CI gate:
```powershell
./check.ps1
```

Expected: All 5 checks pass (SpecLint, Build, Format, Clippy, Tests)

## Manual Verification

### 1. Code Review
- [ ] `consecutive_empty_scans` initialized to `0u32` (around line 132)
- [ ] On non-empty scan: counter reset to 0
- [ ] On empty scan: counter incremented, warning logged with attempt number
- [ ] After 3+ consecutive empty scans: error logged and `break` executed
- [ ] `next_candidate_scan` update still happens regardless of empty/non-empty

### 2. Gate Run
```powershell
./check.ps1
```

- [ ] Repo CI gate passes with no regressions
- [ ] No new build, format, clippy, or test failures

### 3. Live Test (Optional)

Run the twitteractivity task with logging enabled:

```powershell
$env:RUST_LOG="auto::task::twitteractivity=info,auto::utils::twitter::twitteractivity_feed=info" cargo run -- twitteractivity --config config.toml
```

Watch logs for:
- [ ] Normal scans show candidates (if feed is working)
- [ ] After 1st empty scan: `[twitter] No candidates found (attempt 1)`
- [ ] After 2nd empty scan: `(attempt 2)`
- [ ] After 3rd empty scan: `(attempt 3)` then `[twitter] Too many empty scans, stopping task`
- [ ] After task stops (empty feed), verify it stopped due to empty scans (not timeout)

### 4. Edge Cases

- [ ] Non-empty scan after empty scan resets counter to 0
- [ ] Task continues normally when feed has content
- [ ] Single empty scan doesn't stop task (threshold = 3)
- [ ] Multiple empty scans followed by success works correctly

## Regression Check

- [ ] Other tasks using `identify_engagement_candidates()` are not affected (change is local to twitteractivity.rs)
- [ ] Normal feed scanning behavior unchanged when content is available
- [ ] `candidate_scan_interval` timing not impacted
- [ ] Log output format consistent with existing patterns