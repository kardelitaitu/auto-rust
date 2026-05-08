# Validation Checklist

## Automated Checks

Run the repo CI gate:
```powershell
./check.ps1
```

Expected: All 5 checks pass (SpecLint, Build, Format, Clippy, Tests)

## Manual Verification

### 1. Code Review
- [ ] `api.pause(scroll_pause_ms).await;` added after `scroll_read()` call (line ~166)
- [ ] Pause uses `scroll_pause_ms` variable (not hardcoded value)
- [ ] Pause is inside the `if now >= next_scroll` block (only after scrolling)
- [ ] `identify_engagement_candidates()` still runs after the pause

### 2. Gate Run
```powershell
./check.ps1
```

- [ ] Repo CI gate passes with no regressions
- [ ] No new build, format, clippy, or test failures

### 3. Live Test (Optional)
Run the twitteractivity task with logging enabled:

```powershell
$rust_log="auto::task::twitteractivity=info,auto::utils::twitter::twitteractivity_feed=info" cargo run -- twitteractivity --config config.toml
```

Watch logs for:
- [ ] `Candidate scan | candidates=N` shows N > 0 after scrolls
- [ ] Candidate count increases after the fix (compared to before)
- [ ] No new warnings or errors in log output

### 4. Performance Check
- [ ] Task duration not significantly impacted (< 5% increase)
- [ ] `scroll_pause_ms` value appropriate for content loading (current default is 1000ms)

## Regression Check

- [ ] Other tasks using `scroll_read()` are not affected (change is local to twitteractivity.rs)
- [ ] Behavior profile settings still respected
- [ ] `scroll_interval` timing not impacted