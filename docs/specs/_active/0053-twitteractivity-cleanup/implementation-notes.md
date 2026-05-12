# Implementation Notes

## Changes Made

### Removed dead functions (13 items)

| Function | File | Reason |
|---|---|---|
| `read_full_thread()` | dive.rs | Never called |
| `ThreadCache` struct + methods | dive.rs | Only used by dead read_full_thread |
| `extract_initial_thread_data()` | dive.rs | Only relevant if read_full_thread called |
| `extract_visible_replies()` | dive.rs | Depended on removed ThreadCache |
| `navigate_to_tweet()` | interact.rs | Not called |
| `check_selector_health()` | navigation.rs | Never invoked in task flow |
| `retry_with_fallback()` | retry.rs | Not called |
| `scroll_feed()` | feed.rs | Not called (uses api.scroll_read directly) |
| `get_tweet_engagement_buttons()` | feed.rs | Not called |
| `get_scroll_progress()` | feed.rs | Only called inside dead read_full_thread |
| `ensure_feed_populated()` | feed.rs | Not called |
| `scroll_to_bottom_feed()` | feed.rs | Not called |
| `read_content_for()` | humanized.rs | Not called |
| `verify_element_hover()` | humanized.rs | Not called |

### Removed unused enum
- `EngagementCheck` enum + `is_allowed()` / `reason()` removed from `limits.rs`
- `DEFAULT_TWITTERACTIVITY_DURATION_MS` kept — used in `policy.rs`
- `config.persona_file_path` kept — used in config validation

### Deduplicated persona building
- `simulation.rs::build_persona_weights()` now delegates to `select_persona_weights()` instead of duplicating parser logic

### LLM client once per process
- Added `llm_instance()` with `OnceLock<Llm>` in `llm.rs`
- Both `generate_reply()` and `generate_quote_commentary()` use this shared instance
- First call initializes, subsequent calls reuse

### Fixed selector quoting consistency
- `REPLY_BUTTON_SELECTOR` changed from `r#"button[data-testid="reply"]"#` to `r#"button[data-testid=\"reply\"]"#` to match all other selector constants

### Cleaned up unused imports
- `twitteractivity_humanized::*` removed from `dive.rs` and `feed.rs`
- `Instant` removed from `humanized.rs`

### Updated tests
- Removed tests in `feed.rs` that verified removed functions
- Removed tests in `limits.rs` that verified removed `EngagementCheck`

## Verification
- All 5 checks pass: spec-lint, build, format, clippy, 2099 tests
- 9 fewer tests than before (removed dead-code tests)
