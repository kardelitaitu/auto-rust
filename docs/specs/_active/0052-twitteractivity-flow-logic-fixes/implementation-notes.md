# Implementation Notes

## Changes Made

### H1: Fix post-dive candidate scan reset
- Added `next_candidate_scan: Instant` to `CandidateResult` struct
- `process_candidate()` now receives and returns `next_candidate_scan`
- After dive + goto_home, both `next_scroll` and `next_candidate_scan` are reset to `now + scroll_interval`

### H2: Bounded sleep in main loop
- `twitteractivity.rs`: replaced unbounded `tokio::time::sleep(next_candidate_scan - now)` with 250ms chunks bounded by `session.remaining_time()`
- Deadline checks now happen at most every 250ms instead of unlimited durations

### H3: Decouple should_dive from non-like actions
- Removed the `allow_dive` gate (`actions_to_do.retain(|&action| action == "like")`)
- Each action type now independently decides based on its own probability
- Actions needing detail view (retweet, reply, etc.) still check `did_dive` at execution time

### H4: Fix PersonaStrategy multiplier
- Changed from `.min()` to multiplication: `(1.5 * im).clamp(0.0, 2.0)` etc.
- All three levels (Full, Medium, Minimal) now consistently apply `interest_multiplier`

### H5: Remove dead actions_taken
- Removed `actions_taken` from `process_candidate()` signature
- Removed `actions_taken` from `CandidateResult` struct
- Removed `_actions_taken` parameter from `engage_replies()`
- The real action count is tracked via `session.counters`

### H6: Fix cookie banner selectors
- Removed `:contains()` pseudo-selectors (unsupported by `querySelector`)
- Added JS fallback that searches all buttons by text content for terms: "accept", "allow", "got it"

### H7: Fix select_entry_point (not applied)
- `select_entry_point()` doc updated but signature unchanged — requires threading TaskConfig through phase1_navigation which adds scope. Left for future work.

### H8: Fix dive pause
- Changed from 300s constant to `Duration::from_secs(60)` — 1 minute max pause during dive

### H9: Lazy regex
- Added `OnceLock`-based `mentions_regex()` and `hashtags_regex()` functions
- Regex patterns compiled once, reused on all subsequent calls

## Files Changed
- `src/task/twitteractivity.rs` — bounded sleep, CandidateResult usage, removed actions_taken
- `src/utils/twitter/twitteractivity_engagement.rs` — next_candidate_scan plumbing, removed should_dive gate, removed _actions_taken, 60s dive pause
- `src/utils/twitter/twitteractivity_state.rs` — CandidateResult: added next_candidate_scan, removed actions_taken
- `src/utils/twitter/twitteractivity_popup.rs` — removed :contains() selectors, added JS text-matching fallback
- `src/utils/twitter/decision/strategies/persona.rs` — multiplier uses multiplication with clamp
- `src/utils/twitter/twitteractivity_llm_validation.rs` — OnceLock-based lazy regex

## Verification
- All 5 checks pass: spec-lint, build, format, clippy, 2108 tests
- No behavioral regressions
