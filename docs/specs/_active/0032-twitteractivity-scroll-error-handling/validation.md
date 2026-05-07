# Validation Checklist

## Automated Checks

Run the repo CI gate:
```powershell
./check.ps1
```

Expected: All 5 checks pass (SpecLint, Build, Format, Clippy, Tests)

## Manual Verification

### 1. Code Review
- [ ] `consecutive_scroll_failures` initialized to `0u32` (around line 132)
- [ ] `match` expression used instead of `let _ = ...`
- [ ] On `Ok(())`: counter reset to 0
- [ ] On `Err(e)`: counter incremented, warning logged with attempt number
- [ ] After 3 consecutive failures: error logged and `break` executed
- [ ] `next_scroll` update still happens regardless of success/failure

### 2. Gate Run
```powershell
./check.ps1
```

- [ ] Repo CI gate passes with no regressions
- [ ] No new build, format, clippy, or test failures

### 3. Live Test (Optional)

Run the twitteractivity task and artificially cause scroll failures:

```powershell
$env:RUST_LOG="auto::task::twitteractivity=info,auto::task::twitteractivity=warn" cargo run -- twitteractivity --config config.toml
```

Watch logs for:
- [ ] First scroll failure logs: `[twitter] Scroll failed (attempt 1): ...`
- [ ] Second failure: `(attempt 2)`
- [ ] Third failure: `(attempt 3)` then `[twitter] Too many consecutive scroll failures, stopping task`
- [ ] After task stops, verify it stopped due to scroll failures (not timeout)

### 4. Edge Cases

- [ ] Single transient failure doesn't stop task (counter resets on success)
- [ ] Successful scroll after failure resets counter to 0
- [ ] Task continues normally when all scrolls succeed (no log spam)

## Regression Check

- [ ] Other tasks using `scroll_read()` are not affected (change is local to twitteractivity.rs)
- [ ] Normal scrolling behavior unchanged when successful
- [ ] `scroll_interval` timing not impacted
- [ ] Log output format consistent with existing patterns