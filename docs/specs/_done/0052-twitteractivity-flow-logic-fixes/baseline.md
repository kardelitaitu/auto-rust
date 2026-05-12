# Baseline

## What I Find

### H1 -- Post-dive duplicate feed re-scan
Main loop checks `next_candidate_scan` before scanning. `engagement.rs:363` pauses scroll 300s during dive. After dive returns and navigates home, `next_scroll` is reset but `next_candidate_scan` is NOT reset -- it's in the past (dive took 5-30s). Loop immediately re-scans without scrolling, potentially re-processing same tweets.

### H2 -- Non-interruptible sleep bypasses deadline
`twitteractivity.rs:154-156`: `tokio::time::sleep(next_candidate_scan - now)` sleeps full duration regardless of session deadline. If `next_candidate_scan` is 60s away, sleeps 60s even with only 30s remaining.

### H3 -- should_dive() gates all non-like actions
`engagement.rs:327-330`: if `should_dive()` returns false, ALL non-like actions stripped to just "like". Default `thread_dive_prob=0.2` means 80% of candidates never get non-like engagement.

### H5 -- actions_taken is write-only
`twitteractivity.rs:129`: threaded through `process_candidate()`, returned via `CandidateResult`, reassigned. Never read or logged. Real count from `session.counters.total_actions()`.

### H6 -- PersonaStrategy multiplier inconsistency
`persona.rs:236-248`: Full/Medium use `min(1.5, interest)` / `min(1.2, interest)`. Minimal uses hardcoded `0.8`, ignoring interest_multiplier entirely. Negative-sentiment tweet gets higher multiplier at Minimal level than Medium.

### H7 -- Non-standard CSS :contains()
`popup.rs:98-99`: `button:contains("Accept all")` -- jQuery-only, not supported by `querySelector()`. Always returns null.

### M3 -- 300s dive pause constant
`engagement.rs:363`: `next_scroll = now + 300s` regardless of actual dive duration. If `goto_home` fails post-dive, scroll paused for 5 minutes.

### M1 -- select_entry_point uses global RNG
`navigation.rs:264`: `rand::random<u32>()` -- non-deterministic, cannot replay scenarios.

### M5 -- Regex compiled every call
`llm_validation.rs:107,113`: `regex::Regex::new()` inside `remove_mentions()` and `remove_hashtags()`.

## What I Claim

All issues are verifiable bugs or code smells. Each has a clear fix of 1-20 lines.

## What Is the Proof

- `should_dive` at 0.2 default: 80% of candidates never get non-like actions
- 300s pause is an order of magnitude longer than needed
- Two `Regex::new()` per LLM validation, each compiling same pattern
- `actions_taken` passes through 3 function boundaries for no effect
