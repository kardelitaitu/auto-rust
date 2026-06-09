# Twitter Activity Task — End-to-End Flow Analysis

## Table of Contents
1. [Critical Bugs](#critical-bugs)
2. [High Severity](#high-severity)
3. [Medium Severity](#medium-severity)
4. [Low Severity & Code Quality](#low-severity)
5. [Dead Code Inventory](#dead-code)
6. [Summary Statistics](#summary)

---

<a name="critical-bugs"></a>
## 🔴 Critical (3)

### C1 — LLM API key never reaches the decision engine *(RESOLVED)*
**Status:** ✅ **RESOLVED** — The API key is now correctly threaded through to `DecisionEngineFactory::create()`.
**Fix:** `engagement.rs:293-297` passes `task_config.llm_api_key.clone()` to `handle_engagement_decision()`, which passes it to `DecisionEngineFactory::create(strategy, llm_api_key)` at line 109.

**Original finding (kept for regression awareness):**
**Files:** `engagement.rs:88`, `engine.rs:49-90` (original line numbers — code has since changed)
The second argument `llm_api_key` was hardcoded to `None`. Every strategy that requires LLM silently fell back to `PersonaStrategy`.

**Impact (original):** `llm_enabled` flag was a no-op for engagement decisions.

**Resolution:** API key now threaded from config through `process_candidate()` → `handle_engagement_decision()` → `DecisionEngineFactory::create()`.

---

### C2 — `extract_tweet_context()` JS returns wrong authors for replies *(RESOLVED)*
**Status:** ✅ **RESOLVED** — The JavaScript has been fixed.
**Fix:** Current code at `llm.rs:126-159` uses:
- `article[data-testid="tweet"]` (correct attribute selector on article)
- Per-reply author via `reply.querySelector('[dir="auto"]')` (proper scoping)
- Loop starts at `i=1` (skips root tweet, iterates replies)
- Each reply gets its own author variable

**Original finding (kept for regression awareness):**
**File:** `llm.rs:116-143` (original line numbers — code has since changed)

The JavaScript had three bugs:

1. Author extraction used `document.querySelector('[data-testid="tweet"] [dir="auto"]')` — always got the first tweet's author, not per-reply authors.
2. Reply extraction used `'article [data-testid="tweet"] [dir="auto"]'` — space before `[data-testid="tweet"]` treated it as a descendant, likely returning zero results.
3. `replies.push({ author: author, text: replyText })` — every reply got the root tweet's `author` instead of its own.

**Impact (original):** All LLM-generated replies and quotes received incorrect author context.

**Resolution:** JS restructured to properly scope per-reply author extraction.

---

### C3 — Popup interference with login detection *(RESOLVED)*
**Status:** ✅ **RESOLVED** — Popups are now dismissed before login verification.
**Fix:** `phase1_navigation()` (navigation.rs:172-190) dismisses cookie banners and active popups *before* calling `verify_login()`, with an explicit comment explaining the ordering.

**Original finding (kept for regression awareness):**
**File:** `navigation.rs:327-354` (original line numbers — code has since changed)
**Flow:** `run_inner()` → `phase1_navigation()` → (1) navigate → (2) `verify_login()` → (3) dismiss popups

Dismissal order was wrong. `verify_login()` checks `is_feed_visible()` which depends on the feed being unobstructed. If a cookie banner, signup nag, or overlay is covering the feed, `is_feed_visible()` returns `false` even when the user IS logged in. Then `verify_login()` returns `false`, and the code logs "User appears not logged in; task may fail". Popups were dismissed *after* this check.

**Impact (original):** Every task logged a false-positive "not logged in" warning when popups were present.

**Resolution:** Popup dismissal moved before `verify_login()` call.

---

<a name="high-severity"></a>
## 🟠 High (8)

### H1 — Duplicate feed re-scan immediately after thread dive
**File:** `twitteractivity.rs:151-241` (main loop), `engagement.rs:847-848`
**Flow:** process_candidate dive → return → loop top → next_candidate_scan check

When `process_candidate()` dives into a thread and navigates back to home, it resets `next_scroll = Instant::now() + scroll_interval` (engagement.rs:847). But `next_candidate_scan` is NOT reset — it still holds the pre-dive timestamp. Since the dive took 5-30 seconds, `next_candidate_scan` is now in the past. The loop immediately calls `identify_engagement_candidates()` without scrolling first, potentially finding the same candidates that were just processed.

**Impact:** Duplicate processing of tweets after every thread dive. Same tweets may be re-identified and engagement re-attempted (though `action_tracker` prevents rapid re-engagement on the same tweet).

**Fix:** Reset `next_candidate_scan = Instant::now() + candidate_scan_interval` after returning from a dive, similar to how `next_scroll` is reset.

---

### H2 — Main loop `continue` bypasses deadline check during long sleeps
**File:** `twitteractivity.rs:153-157`
```rust
if now < next_candidate_scan {
    tokio::time::sleep(next_candidate_scan - now).await;
    continue;  // <-- goes back to top, checks session.is_expired()
}
```
While this DOES re-check the deadline at the top, the `tokio::time::sleep` itself is not interruptible by the session deadline. If `next_candidate_scan` is set to 60s in the future, the task sleeps for 60 seconds — even if the session has only 30 seconds remaining. The timeout wrapper (`run_with_timeout`) would eventually kill it, but internal deadline checks are bypassed.

**Impact:** Task can overshoot its internal deadline by up to `candidate_scan_interval_ms` (default 2500ms, but could be longer). The external timeout wrapper prevents runaway, but the internal session state becomes stale.

**Fix:** Use `tokio::time::sleep_until(next_candidate_scan.min(session.deadline))` or sleep in shorter increments with deadline checks.

---

### H3 — `should_dive` gates ALL non-like actions
**File:** `engagement.rs:327-330`
```rust
let allow_dive = should_dive(&candidate_persona) && status_url.is_some();
if !allow_dive {
    actions_to_do.retain(|&action| action == "like");
}
```

If the persona decides NOT to dive (based on `thread_dive_prob`), retweet, follow, reply, bookmark, and quote are all silently dropped and replaced with just "like". The decision to open a detail view is conflated with the decision to perform non-trivial engagement. A tweet could have high retweet probability but zero `thread_dive_prob`, resulting in a like instead of a retweet.

**Impact:** Non-like engagement only happens when both `should_dive()` AND the specific `should_{action}()` return true. This significantly reduces variety in engagement actions. With default weights (`thread_dive_prob = 0.2`), there's only a 20% chance per candidate that any non-like actions are even considered.

**Fix:** Separate the dive decision from the action selection. Or document this coupling explicitly.

---

### H4 — `enhanced_sentiment` uses hardcoded dummy data
**File:** `sentiment/analyzer.rs:910-929`
```rust
pub fn extract_user_reputation(_tweet_obj: &Value) -> Option<UserReputation> {
    Some(UserReputation {
        follower_count: 1000,
        is_verified: false,
        account_age_days: 365,
        // ... all hardcoded
    })
}
pub fn extract_temporal_factors(_tweet_obj: &Value) -> Option<TemporalFactors> {
    Some(TemporalFactors {
        hour_of_day: 12,
        day_of_week: 1,
        hours_since_post: 24.0,
        // ... all hardcoded
    })
}
```

Both functions ignore their input and return fixed values. `analyze_enhanced()` uses these to compute `reputation_score` and `temporal_score` — but since the inputs are always the same, these scores contribute constant noise rather than meaningful signal.

**Impact:** Enhanced sentiment analysis is placebo. It produces different numbers but they don't reflect real tweet characteristics.

**Fix:** Either implement real extraction from tweet data, or remove the enhanced mode and simplify to basic keyword analysis only.

---

### H5 — Token `actions_taken` is write-only dead code
**File:** `twitteractivity.rs:129,211-221`, `engagement.rs:232,408,792`
**Flow:** `run_inner()` maintains `actions_taken`, passes it to `process_candidate()`, receives updated value in `CandidateResult`, assigns it back. But it's never read.

`actions_taken` is initialized to 0, threaded through `process_candidate()`, incremented inside (clone of parameter `_actions_taken` on line 232), returned via `CandidateResult`, and reassigned on line 221. It is never logged, checked in conditions, or used for any decision. The real action count comes from `session.counters.total_actions()`.

**Impact:** Confusing code that suggests the counter is meaningful. Risk that someone relies on it in the future without realizing it's disconnected from `session.counters`.

**Fix:** Remove `actions_taken` from the call chain and `CandidateResult`.

---

### H6 — `PersonaStrategy` multiplier uses `.min()` instead of meaningful combination
**File:** `decision/strategies/persona.rs:236-237`
```rust
EngagementLevel::Full => 1.5f64.min(ctx.persona.interest_multiplier),
EngagementLevel::Medium => 1.2f64.min(ctx.persona.interest_multiplier),
EngagementLevel::Minimal => 0.8, // <-- interest_multiplier ignored!
```

For Full and Medium, `interest_multiplier` can only *reduce* the cap (never amplify beyond 1.5/1.2). For Minimal, `interest_multiplier` is entirely ignored. So a negative-sentiment tweet (interest_multiplier ≈ 0.3) with Minimal level gets `multiplier = 0.8`, while a negative-sentiment tweet with Medium level gets `multiplier = 0.3`. Counterintuitively, minimal engagement gets a *higher* multiplier.

**Impact:** Sentiment-based weight modulation is inconsistent across engagement levels. Can produce wrong engagement intensity.

**Fix:** Multiply `base_multiplier * interest_multiplier` consistently across all levels, with appropriate clamping.

---

### H7 — Cookie banner uses non-standard `:contains()` pseudo-selector
**File:** `popup.rs:96-99`
```rust
let cookie_selectors = [
    "button[aria-label*='Accept']",
    "button[data-testid*='accept']",
    "button:contains('Accept all')",            // <-- NOT standard CSS
    "div[role='button']:contains('Accept')",    // <-- NOT standard CSS
];
```
`:contains()` is a jQuery pseudo-selector, not a standard CSS selector. `document.querySelector()` doesn't support it. These two selectors silently return `null` on every call.

**Impact:** Two of four cookie banner selectors never match, reducing dismissal coverage.

**Fix:** Replace with attribute-selector equivalents or iterate buttons in JS.

---

### H8 — `is_on_tweet_page` false positive on home feed with open modal
**File:** `interact.rs:88-113`
```rust
pub async fn is_on_tweet_page(api: &TaskContext) -> Result<bool> {
    let url = get_current_url(api).await?;
    if url.contains("/status/") {
        return Ok(true);
    }
    // Also check for tweet detail modal visibility
    // ...
}
```
If a tweet detail modal is open on the home page (e.g., user clicked a tweet), URL still shows `/home` but the function returns `true` because of the modal check. The subsequent retweet/follow/reply/bookmark actions would then run on the modal, not knowing they're in a modal context.

**Impact:** Actions may operate on the wrong DOM context (modal vs. dedicated page), potentially clicking wrong elements.

**Fix:** Check both URL AND modal status, and pass context info to action functions.

---

<a name="medium-severity"></a>
## 🟡 Medium (10)

### M1 — `select_entry_point` uses `rand::random` without seed
**File:** `navigation.rs:263-274`

`select_entry_point()` uses `rand::random::<u32>()` which is not deterministic. Tests verify correct distribution statistically (1000 samples with 5% tolerance) but the production path cannot be reproduced — making bug reports hard to replay.

**Impact:** Non-reproducible navigation start, debugging harder.

**Fix:** Seed the random choice from `TaskConfig.seed`.

---

### M2 — `consecutive_empty_scans` doesn't reset after scroll failures
**File:** `twitteractivity.rs:165-181,190,227-237`

When scroll succeeds, `consecutive_scroll_failures` resets (good). But `consecutive_empty_scans` and `consecutive_scroll_failures` are independent. If the feed is legitimately empty (all content consumed), scroll failures might eventually trigger exit, but empty scans are the correct exit path. After 3 consecutive scroll failures, the task breaks. But `consecutive_empty_scans` could also be at 3, and whichever triggers first wins.

Not exactly a bug, but the two failure counters have overlapping semantics: both break the loop at 3. If scrolling fails, there's no point continuing since we can't load new candidates anyway. The scroll failure exit (at 3) makes the empty scan exit (also at 3) redundant in some cases.

**Impact:** Task may exit via scroll failures even if the feed still has content (if scrolling temporarily fails for other reasons like network timeout).

---

### M3 — `dive_into_thread` pauses scrolling for 300 seconds (constant)
**File:** `engagement.rs:363`
```rust
next_scroll = Instant::now() + Duration::from_secs(300);
```
Five-minute pause regardless of how long the dive actually takes. If the dive finishes in 15 seconds, scrolling is paused for the remaining 285 seconds unnecessarily. Then line 847 resets it after nav back to home, so the 300s pause is effectively ignored. But if `goto_home` fails (line 838-844), `next_scroll` stays at 300s, and scrolling is paused for the rest of the session.

**Impact:** If navigation back to home fails after a dive, the rest of the session does no scrolling and finds no new candidates.

**Fix:** Use `Duration::from_secs(60)` or use the actual dive duration.

---

### M4 — LLM client created fresh for every reply/quote
**Files:** `llm.rs:43,87`
```rust
let llm = Llm::new().context("Failed to initialize LLM client")?;
```
Both `generate_reply()` and `generate_quote_commentary()` create a new `Llm::new()` client on every invocation. If multiple replies/quotes are generated in a session, each call incurs connection setup overhead.

**Impact:** Unnecessary latency. Could be mitigated by passing a shared client reference.

---

### M5 — Regex compiled on every LLM validation call
**File:** `llm_validation.rs:107,113`
```rust
fn remove_mentions(text: &str) -> String {
    let re = regex::Regex::new(r"@\w+").expect("...");
    re.replace_all(text, "").to_string()
}
```
Both `remove_mentions` and `remove_hashtags` compile regexes on every invocation. Since these are called for every LLM-generated reply/quote, regex compilation overhead accumulates.

**Impact:** Minor performance cost per call.

**Fix:** Use `once_cell::sync::Lazy` or `std::sync::OnceLock` to compile once.

---

### M6 — `run_inner` re-fetches `behavior_runtime()` per scan but `profile` already fetched
**File:** `twitteractivity.rs:132`

`behavior_runtime()` is called inside the while loop. The returned profile is used for scroll parameters (amount, pause, smooth). This line is executed on every loop iteration, but `config.twitter_activity.scroll_amount_pixels` (if > 0) bypasses it entirely. So `behavior_runtime()` is fetched but ignored if config has a scroll override.

**Minor waste**, not a correctness bug.

---

### M7 — `simulate.rs` has its own `build_persona_weights()` — duplicates `select_persona_weights()`
**Files:** `simulation.rs:290-342`, `persona.rs:108-172`

Both functions parse the same `like_prob`, `retweet_prob`, etc. fields from `task_config.weights` and fall back to `config.probabilities`. Any change to one must be mirrored in the other.

**Fix:** Delegate to `select_persona_weights()` from simulation.

---

### M8 — `EngagementLimits::with_limits()` has 8 positional `u32` params
**File:** `limits.rs:214-234`
```rust
pub fn with_limits(
    max_likes: u32,
    max_retweets: u32,
    max_follows: u32,
    max_replies: u32,
    max_thread_dives: u32,
    max_bookmarks: u32,
    max_quote_tweets: u32,
    max_total: u32,
) -> Self { ... }
```
Two call sites (`twitteractivity.rs:95-104`, `simulation.rs:158-167`) pass all 8 arguments positionally. Easy to transpose two adjacent params (e.g., `max_replies` and `max_thread_dives`).

**Fix:** Use a builder pattern or named fields struct.

---

### M9 — `extract_thread_context()` reports hardcoded `is_reply: false`, `is_quote: false`, `thread_depth: 0`
**File:** `sentiment/analyzer.rs:903-906`

Regardless of actual tweet structure, the extracted context always says it's not a reply, not a quote, and depth 0. This feeds into `analyze_enhanced()` which uses these values for context scoring. The `thread_depth` modifier (line 613-619) always returns 0.0 because of hardcoded depth.

**Impact:** `is_reply` and `is_quote` modifiers (0.08 and 0.12 respectively) are always skipped. Thread depth never adds to score.

---

### M10 — `dismiss_signup_nag()` is hard-disabled
**File:** `popup.rs:139-141`
```rust
pub async fn dismiss_signup_nag(_api: &TaskContext) -> Result<bool> {
    Ok(false)
}
```
Disabling is fine, but the comment says "DISABLED: Causing hangs, skip for now" — suggesting an unresolved issue. The function is still called in `phase1_navigation()` (navigation.rs:344) — it returns `Ok(false)` quickly, so there's no hang. But the signup nag remains undismissed.

**Fix:** Either fix the root cause or remove the call.

---

<a name="low-severity"></a>
## 🔵 Low & Code Quality (12)

### L1 — `RETWEET_BUTTON_SELECTOR` and `REPLY_BUTTON_SELECTOR` use inconsistent quoting
**File:** `selectors.rs:83,118`
```rust
pub const RETWEET_BUTTON_SELECTOR: &str = r#"button[data-testid=\"retweet\"]"#;
pub const REPLY_BUTTON_SELECTOR: &str = r#"button[data-testid="reply"]"#;
```
RETWEET uses escaped quotes (`\"`), REPLY uses raw quotes (`"`). Both are raw string literals (`r#"..."#`). The escaped form in RETWEET means the quotes inside are literal, while REPLY's are also literal. They should be consistent.

### L2 — `like_at_position` JS uses escaped template variables not raw strings
**File:** `engagement.rs:893-927`

The verification JS is built with `format!()` which embeds `{x}` and `{y}` in the JS code. But the JS itself has template syntax conflicts: the inner code uses `{` and `}` (e.g., `'rgb('`). This works because they're in a `format!()` where only `{x}` and `{y}` are substitution markers, but it's fragile. If someone adds a JS block with `{}` that looks like a format placeholder, it won't compile.

### L3 — `actions_this_scan` initialized both in loop and at inner scope
**File:** `twitteractivity.rs:196`

`actions_this_scan` on line 129 (`let mut actions_taken = 0u32;`) is the outer variable. Then line 196 (`let mut actions_this_scan = 0u32;`) creates a *new* local in the `.iter().take()` block that shadows the outer one. The outer `actions_this_scan` is never used — only the inner `actions_this_scan` matters. This is confusing variable shadowing.

### L4 — `Selector constants` include `HOME_LOGO_SELECTOR` with escaped quotes
**File:** `selectors.rs:68`
```rust
pub const HOME_LOGO_SELECTOR: &str = r#"a[aria-label=\"X\"]"#;
```
The `\"` in a raw string literal `r#"..."#` actually produces literal `\"` in the string — meaning the string contains `a[aria-label=\"X\"]` with backslashes. This is used in `navigation.rs:66` where it's passed to `api.wait_for_any_visible_selector()`. If the API passes this to `document.querySelector()`, the backslash-escaped quotes would fail to match.

Wait actually, I'm not sure about this. Let me reconsider. In Rust, `r#"a[aria-label=\"X\"]"#` — the content between the `r#"` and `"#` is literal. So `\"` is literally `\"` (two characters: backslash and quote). The resulting string is `a[aria-label=\"X\"]`.

But `REPLY_BUTTON_SELECTOR` uses `r#"button[data-testid="reply"]"#` which produces `button[data-testid="reply"]` — no backslashes.

So `HOME_LOGO_SELECTOR` contains literal backslashes before the quotes, which would be wrong for CSS selectors. `aria-label=\"X\"` is NOT valid CSS — it should be `aria-label="X"`.

This is a bug! The `HOME_LOGO_SELECTOR` constant produces a broken CSS selector because of unwanted escaped quotes in a raw string.

Wait, but it's used in `navigation.rs:66` and the task presumably works. Let me re-check: `r#"a[aria-label=\"X\"]"#` in Rust. Rust `r#"..."#` raw strings treat everything literally. So `\"` IS `\` followed by `"`. The resulting string value is: `a[aria-label=\"X\"]`.

But `querySelector('a[aria-label=\"X\"]')` — in CSS, attribute values don't need backslash escaping of double quotes inside single quotes. This would be `a[aria-label=\"X\"]` which matches elements with `aria-label` equal to `\"X\"` (including backslashes). This is WRONG.

Actually wait, I need to look at how it's used. In `navigation.rs:66`:
```rust
let selector = r#"a[aria-label="X"]"#;
```
It's used with `r#"..."#` raw string LITERAL directly, not using the `HOME_LOGO_SELECTOR` constant.

But `HOME_LOGO_SELECTOR` is defined in `selectors.rs:68` as:
```rust
pub const HOME_LOGO_SELECTOR: &str = r#"a[aria-label=\"X\"]"#;
```
```

And it's exported. If anyone uses `HOME_LOGO_SELECTOR`, they'd get the broken selector. But `navigation.rs:66` doesn't use the constant — it uses its own raw string. So the constant is dead code with a bug.

Actually, this is a real catch: the constant `HOME_LOGO_SELECTOR` has a wrong value because of unnecessary backslash-escaping inside a raw string.

### L5 — `js_confirm_retweet_click.js` embedded file structure not reviewed
The `.js` files are embedded via `include_str!()`. If any of them has a JS syntax error, the first runtime call fails. There's no compile-time validation of JS. If Twitter changes their DOM, these fail silently.

### L6 — Error messages inconsistently prefixed
Some log messages use `[twitter]` prefix (`twitteractivity.rs:172,175`), most don't. Makes log filtering harder.

### L7 — `clustered_engagement_pause` vs `human_pause` API inconsistency
`human_pause(api, base_ms)` is profile-aware (uses `behavior_runtime()` variance). `clustered_engagement_pause(api)` is also profile-aware. But `engagement_pause(api)`, `reply_pause(api)`, and `scroll_pause(api)` directly fetch `behavior_runtime()` themselves. Human pause behavior is spread across two parallel implementations.

### L8 — `ensure_feed_populated` returns `bool` but result often ignored
**File:** `feed.rs:328-336`

The function checks for tweet visibility but is never called in the main flow. It's exported but unused in the task.

### L9 — `get_tweet_engagement_buttons` type confusion
**File:** `feed.rs:242-247`
```rust
pub async fn get_tweet_engagement_buttons(api: &TaskContext) -> Result<Value> {
    let js = selector_engagement_buttons();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value().cloned().unwrap_or_default();
    Ok(value)
}
```
Returns `Value` (generic JSON) — the caller must know the structure. No typed return.

### L10 — `CandidateContext` destructuring in `process_candidate` unnecessary
**File:** `engagement.rs:234-242`
```rust
let tweet = ctx.tweet;
let persona = ctx.persona;
let task_config = ctx.task_config;
// ... all 8 fields destructured
```
The context is a reference, and all fields are re-bound as locals. This works but defeats the purpose of the context struct (which was to reduce parameter count). If a new field is added to `CandidateContext`, it won't cause a destructuring error here — it'll be silently ignored.

### L11 — `log_summary` computes duration as `(task_config.duration_ms - remaining)`
**File:** `twitteractivity.rs:271-273`
```rust
let last_remaining = session.remaining_time();
let duration_secs = (Duration::from_millis(task_config.duration_ms) - last_remaining).as_secs_f64();
```
If the task overshoots (runs past deadline), `last_remaining` is 0, and `duration_secs` = `task_config.duration_ms / 1000`. This is correct but assumes the `-` operation doesn't underflow. Since `Duration` subtraction `saturating_sub`s in Rust, this is safe.

### L12 — `engagement.rs` has extensive test coverage but no async integration tests for DOM interaction
All 1000+ lines of tests are unit tests (verifying JS strings, data structures, probability distributions). The actual browser interaction (like clicking, typing, navigating) is entirely untested at the unit level.

---

<a name="dead-code"></a>
## 💀 Dead Code Inventory

| Item | File | Reason |
|---|---|---|
| `actions_taken` | `twitteractivity.rs:129` | Write-only, `session.counters` used instead |
| `profile` (first binding) | `twitteractivity.rs:86` | Shadowed by second `profile` at line 132 |
| `read_full_thread()` | `dive.rs:263-317` | Never called from any active code path |
| `ThreadCache` | `dive.rs:57-61` | Never populated or used by caller |
| `navigate_to_tweet()` | `interact.rs:150-160` | Not called in `process_candidate()` or `run_inner()` |
| `check_selector_health()` | `navigation.rs:222-257` | Never called in task flow |
| `retry_with_fallback()` | `retry.rs:278-295` | Not called anywhere |
| `get_tweet_engagement_buttons()` | `feed.rs:242-247` | Not called in task flow |
| `ensure_feed_populated()` | `feed.rs:328-336` | Not called in task flow |
| `scroll_to_bottom_feed()` | `feed.rs:340-345` | Not called in task flow |
| `scroll_feed()` | `feed.rs:89-126` | Not called (main loop uses `api.scroll_read` directly) |
| `read_content_for()` | `humanized.rs:78-104` | Not called in task flow |
| `verify_element_hover()` | `humanized.rs:68-73` | Not called in task flow |
| `HOME_LOGO_SELECTOR` | `selectors.rs:68` | Not used (navigation.rs has its own literal) |
| `config.persona_file_path` | `config/mod.rs:251` | Not referenced by any task code |
| `EngagementCheck` enum | `limits.rs:351-376` | Defined but never used (limits are checked via `can_*` methods) |
| `DEFAULT_TWITTERACTIVITY_DURATION_MS` | `constants.rs:5` | Defined but not referenced |

---

<a name="summary"></a>
## Summary Statistics

| Category | Count |
|---|---|
| 🔴 Critical | 0 (3 original — C1, C2, C3 resolved) |
| 🟠 High | 8 |
| 🟡 Medium | 10 |
| 🔵 Low/Quality | 12 |
| 💀 Dead Code Items | 17 |

### Top 5 most impactful fixes (by effort/impact ratio)

1. **C1** — Thread LLM API key to decision engine *(RESOLVED)*: ~5 lines changed, unblocks a major feature
2. **C2** — Fix `extract_tweet_context()` JS *(RESOLVED)*: ~15 lines changed, fixes reply author attribution
3. **C3** — Swap popup dismissal before login check *(RESOLVED)*: ~3 lines changed, eliminates false warnings
4. **H1** — Reset `next_candidate_scan` after dive return: ~2 lines changed, prevents duplicate scans
5. **H2** — Interruptible sleep in main loop: ~3 lines changed, respects deadline during idle

### Files with most issues

| File | Issues |
|---|---|
| `engagement.rs` | C1 (❗resolved), H1, H3, H5, M3, L2, L10 |
| `analyzer.rs` | H4, M9 |
| `llm.rs` | C2 (❗resolved), M4 |
| `navigation.rs` | C3, M1 |
| `persona.rs` | H6 |
