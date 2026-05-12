# Baseline

## What I Find

### Dead Code (17 items)

| # | Item | File | Reason |
|---|---|---|---|
| 1 | `read_full_thread()` | dive.rs:263-317 | Not called anywhere |
| 2 | `ThreadCache` struct | dive.rs:57-75 | Never populated or consumed |
| 3 | `navigate_to_tweet()` | interact.rs:150-160 | Not called |
| 4 | `check_selector_health()` | navigation.rs:222-257 | Not invoked in task flow |
| 5 | `retry_with_fallback()` | retry.rs:278-295 | Not called |
| 6 | `get_tweet_engagement_buttons()` | feed.rs:242-247 | Not called |
| 7 | `ensure_feed_populated()` | feed.rs:328-336 | Not called |
| 8 | `scroll_to_bottom_feed()` | feed.rs:340-345 | Not called |
| 9 | `scroll_feed()` | feed.rs:89-126 | Not called (uses api.scroll_read directly) |
| 10 | `read_content_for()` | humanized.rs:78-104 | Not called |
| 11 | `verify_element_hover()` | humanized.rs:68-73 | Not called |
| 12 | `HOME_LOGO_SELECTOR` | selectors.rs:68 | Not used (navigation.rs has own literal) |
| 13 | `config.persona_file_path` | config/mod.rs:251 | Never referenced by task code |
| 14 | `EngagementCheck` enum | limits.rs:351-376 | Never used |
| 15 | `DEFAULT_TWITTERACTIVITY_DURATION_MS` | constants.rs:5 | Not referenced |
| 16 | `get_scroll_progress()` | feed.rs:287-298 | Only called inside dead read_full_thread |
| 17 | `extract_initial_thread_data()` | dive.rs:377-426 | Only relevant if read_full_thread called |

### Quality Issues (12 items)

| # | Issue | File | Detail |
|---|---|---|---|
| Q1 | HOME_LOGO_SELECTOR spurious backslashes | selectors.rs:68 | `r#"a[aria-label=\"X\"]"#` -- backslash in raw string produces literal `\"` |
| Q2 | Inconsistent selector quoting | selectors.rs:83,118 | RETWEET uses escaped `\"`, REPLY uses raw `"` |
| Q3 | like_at_position format! fragility | engagement.rs:893-927 | `{x}` `{y}` near JS `{}` syntax |
| Q4 | Variable shadowing `profile` | twitteractivity.rs:86,132 | Two different types, same name |
| Q5 | Duplicated persona builder | simulation.rs:290-342 vs persona.rs:108-172 | Same logic in two files |
| Q6 | Llm::new() on every call | llm.rs:43,87 | Fresh client per reply/quote |
| Q7 | extract_thread_context hardcoded | analyzer.rs:903-906 | Always is_reply=false, is_quote=false |
| Q8 | empty selector to hover_before_click | engagement.rs:886 | `hover_before_click(page, "", ...)` |

## What I Claim

Removing dead code eliminates 17 items (~400 lines). Fixing quality improves maintainability and consistency.

## What Is the Proof

- Each dead item verified via grep: no callers outside own definition
- read_full_thread calls get_scroll_progress -- both dead
- HOME_LOGO_SELECTOR: Rust raw strings don't process escapes, so `\"` produces literal backslash+quote -- invalid CSS
- select_persona_weights and build_persona_weights parse identical fields
