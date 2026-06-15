# Twitteractivity Analysis — Group 5: Utilities & Processor

## File: twitteractivity_humanized.rs

**Lines:** 305 (2 test modules)
**Status:** CLEAN

---

### Functions

| Function | Lines | Verdict | Notes |
|----------|-------|---------|-------|
| `human_pause` | 17-21 | OK | Profile-aware base + variance |
| `micro_pause` | 25-33 | OK | ±30% jitter around `min_ms` |
| `after_navigation_pause` | 36-40 | OK | Fixed 1–3s |
| `after_click_pause` | 43-48 | OK | `reaction_delay_ms` + 30% variance |
| `fixed_sleep` | 52-54 | OK | Direct tokio sleep |
| `random_duration` | 59-66 | OK | Gaussian distribution clamped to bounds |
| `scroll_pause` / `engagement_pause` | 69-82 | OK | Factor × action_delay |
| `reply_pause` | 85-90 | OK | Factor ×4 action_delay |
| `clustered_engagement_pause` | 95-101 | OK | 2–3 clusters |
| `clustered_reply_pause` | 105-111 | OK | 1–3 clusters |
| `click_prep_pause` / `click_post_pause` | 113-127 | OK | Factor ×4 / ×8 reaction_delay |
| `attempt_close_popup` | 132-153 | OK | Finds X button, moves, clicks |

**No bugs found.** Minor: `random_duration(0, 0)` returns `Duration::ZERO` (correct).

### Tests

- `duration_tests` (7 tests): bounds, identical bounds, zero, large, distribution, variance, small bounds.
- `selector_tests` (8 tests): all selector strings valid, non-empty, contain expected JS.

---

## File: twitteractivity_helpers.rs

**Lines:** 156
**Status:** CLEAN (no tests)

---

### Functions

| Function | Lines | Verdict | Notes |
|----------|-------|---------|-------|
| `calc_rate` | 17-25 | OK | Safe for total=0 |
| `action_allowed_by_limits` | 29-43 | OK | Dispatches 6 actions |
| `validate_tweet_page` | 46-67 | OK | Checks `did_dive` + `is_on_tweet_page` |
| `selected_candidate_actions` | 69-116 | OK | 6 actions: persona × tracker × limits |
| `filter_detail_actions_for_gate` | 118-126 | OK | Only "like" if no dive path |
| `filter_actions_for_decision_level` | 128-144 | OK | Full/Medium/Minimal/None |
| `should_engage_replies_after_root_action` | 146-152 | OK | |
| `should_navigate_home_after_dive` | 154-156 | OK | |

**No bugs found.** Pure utility — no tests, but low risk.

---

## File: twitteractivity_errors.rs

**Lines:** 412 (4 test modules)
**Status:** CLEAN

---

### Types & Traits

| Item | Lines | Verdict | Notes |
|------|-------|---------|-------|
| `ErrorClass` enum | 14-31 | OK | Transient / Permanent / Fatal |
| `ErrorClassifier` trait | 34-37 | OK | Single method |
| `impl for anyhow::Error` | 39-74 | OK | String matching with `to_lowercase()` |
| `impl for io::Error` | 76-96 | OK | ErrorKind dispatch |
| `is_rate_limit_error` | 100-105 | OK | String heuristic |
| `is_auth_error` | 109-116 | OK | String heuristic |

**No bugs found.** Well-tested with 4 test modules (tdd, classification, detection, gap — 30+ tests).

---

## File: twitteractivity_simulation.rs

**Lines:** 757 (3 test modules)
**Status:** CLEAN

---

### Key Types

| Type | Lines | Verdict | Notes |
|------|-------|---------|-------|
| `SimulationReport` | 24-30 | OK | Fields accessible |
| `SimulationStopReason` | 32-61 | OK | 5 variants with Display + as_str |
| `SimAction` | 63-141 | OK | 7 actions with probability, limit check, increment |

### Key Function

| Function | Lines | Verdict | Notes |
|----------|-------|---------|-------|
| `simulate` | 152-293 | OK | Deterministic seeded RNG, scan loop, limit/duration/exhaustion stop |
| `run_simulation` | 143-149 | OK | Logs report lines via `info!` |

**No bugs found.** Deterministic, well-tested with 3 test modules (20+ tests). Seeded `StdRng` ensures reproducibility.

---

## File: unified_processor.rs

**Lines:** 796 (1 test module, ~30 tests)
**Status:** 2 MEDIUM notes

---

### Types

| Type | Lines | Verdict | Notes |
|------|-------|---------|-------|
| `SentimentAnalysis` | 10-15 | OK | sentiment + confidence + indicators |
| `UnifiedLLMProcessor` | 17-445 | OK | Batch reply + single quote processing |
| `UnifiedReplyResponse` | 447-452 | OK | reply_index + sentiment + content |
| `UnifiedQuoteResponse` | 454-459 | OK | sentiment + content + confidence |

### Findings

| ID | Severity | Description | Location |
|----|----------|-------------|----------|
| UNIFIED-1 | MEDIUM | `clean_reply_content` strips `@` and `#` characters, turning `@user` into `user` and `#topic` into `topic`. Intentional for sanitization but silently loses semantic info. Also strips emoji, parentheses, colons, semicolons — may degrade reply quality for tweets that reference usernames or hashtags | L347-363 |
| UNIFIED-2 | MEDIUM | `extract_content_from_quote` (L441-444) is a no-op: `Ok(response.to_string())` — it returns the raw LLM response unchanged. The function name implies extraction logic. The downstream consumer (`analyze_sentiment_from_text`) works correctly since it operates on the full response, but the extracted content isn't cleaned/filtered before sentiment analysis | L441-444 |
| UNIFIED-3 | LOW | `extract_sentiment_indicators` uses hardcoded English words only (great/amazing/excellent/bad/terrible/awful) — no i18n support | L383-412 |
| UNIFIED-4 | LOW | `calculate_confidence` returns 0.5 base even for empty/whitespace text — test at L677 confirms this. Minor: empty text gets non-zero confidence | L416-431 |

### Tests

~30 tests covering batch parsing (JSON array, single object, line-based, malformed, empty), content cleaning (@/#/unicode stripping), sentiment analysis (positive/negative/neutral), confidence calculation, quote extraction.
