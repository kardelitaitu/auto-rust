# Twitteractivity Analysis — Group 2: Browser Automation

## File: twitteractivity_feed.rs

**Lines:** 317 (2 prod functions, 14 test functions)
**Status:** 1 MEDIUM bug found

---

### Functions

| Function | Lines | Verdict | Notes |
|----------|-------|---------|-------|
| `identify_engagement_candidates` | 90-158 | OK | JS eval + filters (height>50, y<90% viewport, non-empty ID). Viewport fallback to 1920x1080. Debug counters |
| `is_following_user_at_position` | 163-170 | **BUG** | Position params unused, global DOM query — see below |

### Bugs Found

| ID | Severity | Location | Description |
|----|----------|----------|-------------|
| FEED-1 | **MEDIUM** | Line 163-170 | `is_following_user_at_position(api, _x, _y)` ignores `_x`/`_y`. JS scans ALL buttons on page for "Following" text. If multiple users visible, can return wrong state for intended tweet. Follow actions may be incorrectly skipped |

### Tests

14 test functions — all are compile-time/string-inspection only (check JS snippets contain expected selectors). No runtime integration tests.

---

## File: twitteractivity_dive.rs

**Lines:** 376 (4 prod functions, 22 test functions)
**Status:** CLEAN (2 minor notes)

---

### Functions

| Function | Lines | Verdict | Notes |
|----------|-------|---------|-------|
| `status_id_from_url` | 61-67 | OK | Duplicates `StatusUrl::tweet_id()` on raw `&str` |
| `dive_into_thread` | 107-174 | OK | Fix applied (click error via `?`). URL matching handles full/relative/query. 5-selector thread detection |
| `identify_thread_replies` | 181-207 | OK | Filters visible + `like_pos` object, caps at 8 |
| `get_thread_depth` | 222-230 | OK | Counts tweet elements via JS |

### Minor Notes

| ID | Severity | Description |
|----|----------|-------------|
| DIVE-1 | INFO | `used_fallback_target` in `DiveIntoThreadOutcome` always `false` — dead field |
| DIVE-2 | INFO | `status_id_from_url` duplicates `types.rs::StatusUrl::tweet_id()` |

### Tests

22 test functions — status ID extraction (edge cases: query, fragment, trailing slash, www, special chars, malformed URLs).

---

## File: twitteractivity_interact.rs

**Lines:** 732 (12 prod functions, 10 test functions)
**Status:** 1 MEDIUM bug found

---

### Functions

| Function | Lines | Verdict | Notes |
|----------|-------|---------|-------|
| `get_current_url` | 64-76 | OK | JS `window.location.href`, err on missing |
| `is_on_home_feed` | 80-83 | OK | URL contains `x.com/home` or `twitter.com/home` |
| `is_on_tweet_page` | 87-115 | OK | URL + DOM check (modal + tweetDetail selectors) |
| `click_root_tweet_button` | 121-145 | OK | Core helper: JS eval → mouse move → click → pause |
| `like_tweet` | 176-185 | OK | Wraps `click_root_tweet_button` |
| `click_retweet_button` | 218-227 | OK | Same pattern |
| `retweet_tweet` | 254-283 | OK | Click + random 1-2s pause + confirm click |
| `click_reply_button` | 315-321 | OK | Same pattern |
| `send_reply` | 361-499 | OK | 3-step: focus textarea → type → click submit. All steps have timeouts. Verification after send |
| `reply_to_tweet` | 526-531 | OK | `click_reply_button` + `send_reply` |
| `follow_from_tweet` | 566-624 | **BUG** | — see below |
| `bookmark_tweet` | 654-663 | OK | Same pattern |

### Bugs Found

| ID | Severity | Location | Description |
|----|----------|----------|-------------|
| INTERACT-1 | **MEDIUM** | Line 578 | `follow_from_tweet` scans ALL buttons on page for "Following" text (same pattern as FEED-1). In thread view, could match following state from a different user's buttons. Less severe than FEED-1 (thread view has fewer competing elements) but still imprecise |

### Tests

10 test functions — string inspection of `root_tweet_button_center_js` output (visibility check, center function, status ID extraction, selector escaping, null returns).

---

## File: twitteractivity_popup.rs

**Lines:** 365 (3 prod functions, 10 test functions)
**Status:** CLEAN

---

### Functions

| Function | Lines | Verdict | Notes |
|----------|-------|---------|-------|
| `detect_popup` | 19-46 | OK | 3 ordered checks: overlay → follow_confirm → login_flow |
| `close_active_popup` | 51-95 | OK | Dispatches by type: follow_confirm (cancel button) or generic overlay |
| `dismiss_cookie_banner` | 100-173 | OK | 2 selectors + text-content fallback. Quote escaping on selectors |

**No bugs found.** Well-structured popup detection and dismissal with graceful fallbacks.

### Tests

10 test functions — string inspection of JS (IIFE structure, query selectors, null returns, coordinate calc, quote escaping).

---

## File: twitteractivity_selectors.rs

**Lines:** 423 (17 JS functions, 22 CSS constants, 22 test functions)
**Status:** CLEAN

---

Centralized registry for all JS snippets and CSS selectors.

- JS snippets via `include_str!("js/...")` — separate files for maintainability
- `selector_element_center` and `js_root_tweet_button_center`: double-quote escaping for template safety
- `js_verify_like`: `{X}`/`{Y}` replacement for coordinate verification
- 22 CSS constant selectors (like/retweet/reply/follow/bookmark buttons, textarea, popup, login, etc.)
- 17 JS snippet functions (engagement candidates, tweet extraction, reply textarea, quote posting, like verification, health check)

**No bugs found.**

### Tests

22 test functions — backslash check on all CSS constants, string inspection on all JS functions.

---

## File: twitteractivity_navigation.rs

**Lines:** 714 (30 test functions + 33 prod functions)
**Status:** CLEAN (2 minor notes)

---

### Functions

| Function | Lines | Verdict | Notes |
|----------|-------|---------|-------|
| `goto_home` | 67-98 | OK | 3-layer fallback: logo click → URL list → silent OK. Feed-verified after each attempt |
| `get_element_center` | 102-128 | OK (latent) | `'{selector}'` unescaped in inline JS — currently only called with safe literal |
| `goto_home_fallback` | 131-153 | OK | 4-URL fallback list; returns `Ok` even after all fail (conservative) |
| `goto_notifications` | 157-173 | OK | URL nav + wait (`.ok()` allows timeout without failure) |
| `is_feed_visible` | 177-182 | OK | JS eval, defaults to `false` on missing |
| `is_login_flow` | 186-192 | OK | JS eval, non-empty string = login detected |
| `verify_login` | 197-206 | OK | `is_feed_visible && !is_login_flow` |
| `wait_for_page_ready` | 210-219 | OK | Thin wrapper around `wait_for_any_visible_selector` |
| `select_entry_point` | 226-238 | OK | Weighted random selection (modulo bias negligible with 100 total) |
| `navigate_and_read` | 240-289 | OK | Entry nav + humanized scroll simulation on non-home pages |
| `phase1_navigation` | 291-315 | OK | Orchestrates entry nav, popup dismissal, login check |

### Entry Points

15 weighted URLs matching Node.js implementation, weights sum to 100:

| Weight | Count | URLs |
|--------|-------|------|
| 59 | 1 | `x.com/` (home) |
| 4 | 8 | trending, explore, bookmarks, notifications, etc. |
| 2 | 3 | connect_people, explore/for_you |
| 1 | 3 | explore tabs for news, sports, entertainment |

Verified by property tests (distributions, uniqueness, positive weights).

### Bugs Found

| ID | Severity | Location | Description |
|----|----------|----------|-------------|
| NAV-1 | LOW | Line 224-226 | `select_entry_point` doc says *"If seed is 0, uses non-deterministic random"* but fn takes no args. Stale doc drift |
| NAV-2 | LOW | Line 103 | `get_element_center` uses `'{selector}'` in inline JS without quote escaping — latent, not triggered by current callers |

### Tests

30 tests covering:
- Timeout constant verification (4)
- Selector string format checks (11)
- Entry point properties: weights sum to 100, distribution tolerance, uniqueness, positive weights (7)
- TDD-style: weight structure, count checks, HTTPS scheme (8)
