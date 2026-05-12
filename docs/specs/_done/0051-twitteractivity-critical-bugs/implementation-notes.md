# Implementation Notes

## Changes Made

### C1: Thread LLM API key to decision engine

- Added `llm_api_key: Option<String>` to `TaskConfig` struct in `state.rs`
- Reads `OPENROUTER_API_KEY` env var during `TaskConfig::from_payload()` (matching existing pattern in `llm/client.rs`)
- Added `llm_api_key` parameter to `handle_engagement_decision()` in `engagement.rs`
- Replaced hardcoded `None` with the threaded key in `DecisionEngineFactory::create()`
- Updated all call sites: `process_candidate()`, `engage_replies()`, and 3 test functions

### C2: Fix extract_tweet_context() JS

- Fixed `llm.rs` JS selector: changed `'[data-testid="tweet"] [dir="auto"]'` → `'article[data-testid="tweet"] [dir="auto"]'` (was missing `article` element)
- Fixed reply extraction: changed from broken `'article [data-testid="tweet"] [dir="auto"]'` (descendant selector, space before `[data-testid]`) to iterating over `'article[data-testid="tweet"]'` elements and extracting each reply's own author independently
- Each reply now correctly gets its own `replyAuthor` instead of sharing the root tweet's `author`

### C3: Reorder popup dismissal before login check

- Moved popup dismissal (cookie banner + generic overlay) BEFORE `verify_login()` call in `phase1_navigation()`
- Removed `dismiss_signup_nag()` call (was hard-disabled, always returning `Ok(false)`)
- Login detection now sees unobstructed feed, eliminating false "not logged in" warnings

## Files Changed

- `src/utils/twitter/twitteractivity_state.rs` — TaskConfig field + env read
- `src/utils/twitter/twitteractivity_engagement.rs` — API key param, call sites, tests
- `src/utils/twitter/twitteractivity_llm.rs` — JS selectors and reply author extraction
- `src/utils/twitter/twitteractivity_navigation.rs` — popup dismissal reorder, removed signup nag call

## Verification

- All 5 checks pass: spec-lint, build, format, clippy, 2108 tests
- No behavioral regressions
