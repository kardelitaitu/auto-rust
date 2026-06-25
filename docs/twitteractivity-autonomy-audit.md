# Twitter Activity — Autonomy Audit

*Generated: June 25, 2026*

Deep investigation of the entire `twitteractivity` system (~10K lines across 31 modules) for gaps that prevent safe, unattended autonomous operation. Documents what's already fixed, what's still broken, and what needs to change.

---

## Navigation

1. [Status of Previously Documented Issues](#status-of-previously-documented-issues)
2. [Finding 1: The `should_dive` Bottleneck (H3)](#finding-1-the-should_dive-bottleneck-h3)
3. [Finding 2: Duplicate Feed Re-Scan After Dive (H1)](#finding-2-duplicate-feed-re-scan-after-dive-h1)
4. [Finding 3: Entry Point Diversity Is Narrow](#finding-3-entry-point-diversity-is-narrow)
5. [Finding 4: Session Timing Is Predictable](#finding-4-session-timing-is-predictable)
6. [Finding 5: Scroll Patterns Are Uniform](#finding-5-scroll-patterns-are-uniform)
7. [Finding 6: No Inter-Session Memory](#finding-6-no-inter-session-memory)
8. [Finding 7: Enhanced Sentiment — Partially Fixed, Partially Placebo (H4)](#finding-7-enhanced-sentiment--partially-fixed-partially-placebo-h4)
9. [Finding 8: Remaining Existing Issues Still Current](#finding-8-remaining-existing-issues-still-current)
10. [Finding 9: Dead Code Inventory](#finding-9-dead-code-inventory)
11. [Finding 10: Issues Already Resolved by Code Changes](#finding-10-issues-already-resolved-by-code-changes)
12. [Implementation Priorities](#implementation-priorities)

---

## Status of Previously Documented Issues

The existing docs at `docs/review/twitteractivity-end-to-end-flow-analysis.md` cataloged 33 items (3 critical resolved, 8 high, 10 medium, 12 low). Since that analysis was written, several have been fixed by code changes:

| Item | Severity | Status | Notes |
|---|---|---|---|
| C1 — LLM API key never reaches decision engine | 🔴 Critical | **Resolved** | Now threaded through to `DecisionEngineFactory::create()` |
| C2 — `extract_tweet_context()` JS wrong authors | 🔴 Critical | **Resolved** | JS restructured, proper per-reply scoping |
| C3 — Popup interference with login detection | 🔴 Critical | **Resolved** | Popup dismissal moved before `verify_login()` |
| H7 — Cookie banner `:contains()` pseudo-selector | 🟡 Medium | **Resolved** | Replaced with `aria-label` + `data-testid` selectors + text-content fallback JS |
| M4 — LLM client created per-call | 🟡 Medium | **Resolved** | Now uses `OnceLock<Llm>` singleton in `llm_instance()` |
| M5 — Regex compiled per-validation call | 🟡 Medium | **Resolved** | All regexes use `OnceLock<Regex>` via lazy static functions |
| M7 — Simulation `build_persona_weights` duplicates | 🟡 Medium | **Resolved** | Now delegates to `select_persona_weights()` from persona module |
| M10 — `dismiss_signup_nag()` hard-disabled | 🟡 Medium | **Resolved** | Function removed from codebase entirely |
| L4 — `HOME_LOGO_SELECTOR` escaped-quotes bug | 🔵 Low | **Resolved** | Current value `r#"a[aria-label="X"]"#` is correct. It is unused dead code. |

**Remaining current: 8 high, 9 medium, 11 low** (see [Finding 8](#finding-8-remaining-existing-issues-still-current)).

---

## Finding 1: The `should_dive` Bottleneck (H3)

**Severity:** High — this is the single largest behavioral constraint in the system.

### Flow Traced

```
process_candidate() in engagement/mod.rs:234
  → selected_candidate_actions() in helpers.rs:66
    → generates full actions list (like/retweet/quote/follow/reply/bookmark)
    → each action independently passes should_{}() check + limit check + tracker check
  → needs_detail_view check at engagement/mod.rs:342
    → actions_to_do.iter().any(|a| a != "like")
  → filters_detail_actions_for_gate() in helpers.rs:107
    → if !has_status_url || !dive_allowed → retain only "like"
```

### The Gate

In `engagement/mod.rs:313-331`:
```rust
let needs_detail_view = actions_to_do.iter().any(|&action| action != "like");
if needs_detail_view {
    let has_status_url = status_url.is_some();
    let dive_allowed = has_status_url && should_dive(&candidate_persona);
    ...
    filter_detail_actions_for_gate(&mut actions_to_do, has_status_url, dive_allowed);
}
```

In `helpers.rs:107-112`:
```rust
pub fn filter_detail_actions_for_gate(
    actions_to_do: &mut Vec<&'static str>,
    has_status_url: bool,
    dive_allowed: bool,
) {
    if actions_to_do.iter().any(|&action| action != "like") && (!has_status_url || !dive_allowed) {
        actions_to_do.retain(|&action| action == "like");
    }
}
```

### Impact

With default `thread_dive_prob = 0.2` and `interest_multiplier = 1.0`:

- `effective_probability(0.2, persona)` = `(0.2 * 1.0).clamp(0, 1)` = **0.2**
- 80% of candidates → only "like" survives
- Even if `should_retweet()` returns true, the retweet is **silently dropped** before execution
- Same applies to: retweet, quote, follow, reply, bookmark — all gated behind `should_dive()`

This means the engagement profile is: 80% likes, ~4% retweets, ~1% each for other actions. An account doing this will look like a like-bot in Twitter's telemetry.

### Root Cause

The design conflates two separate decisions:
1. "Should I open the detail view?" (dive decision)  
2. "Should I perform this action?" (action decision)

Not all non-like actions need a thread dive. Retweets, follows, and bookmarks can work from the feed if the button is visible. Only replies and quotes genuinely need the detail view.

### Candidates

- **Option A**: Separate action types into feed-safe (retweet/follow/bookmark) vs. detail-needed (reply/quote). Gate only the latter behind `should_dive()`.
- **Option B**: Remove the dive gate entirely. Attempt feed-safe actions from feed position data, and only dive when a detail-needed action passes its probability check.
- **Option C**: Add a `status_url` check as an alternative path — if the tweet data includes button coordinates for retweet/follow/bookmark, use them from the feed without diving.

---

## Finding 2: Duplicate Feed Re-Scan After Dive (H1)

**Severity:** High

### Flow

`process_candidate()` dives → engages → navigates back to home → returns to main loop → loop checks `next_candidate_scan`:

In `engagement/mod.rs:847-848`:
```rust
next_candidate_scan = Instant::now() + scroll_interval;
```

This resets `next_candidate_scan`, but only AFTER `goto_home()` succeeds. If navigation home fails (line 838-844), `next_candidate_scan` is NOT reset, and the loop immediately calls `identify_engagement_candidates()` with the old (now-past) timestamp.

### Impact

Same tweets re-scanned immediately after every dive return. The `action_tracker` prevents rapid re-engagement on the same tweet (cooldown = `MIN_ACTION_CHAIN_DELAY_MS` = 3s), but the duplicate scan wastes time within the session window.

### Fix

Reset `next_candidate_scan` immediately at the top of the dive-return path, not only after `goto_home()`. Or use a single atomic timestamp reset that covers both success and failure paths.

---

## Finding 3: Entry Point Diversity Is Narrow

**Severity:** Medium

### Current Weights

15 URLs, 4 weight tiers:

| Weight | Destinations | Aggregate |
|---|---|---|
| 59% | `x.com/` (home) | 59% |
| 4% × 8 | explore, notifications, bookmarks, trending, chat | 32% |
| 2% × 2 | connect_people variants | 4% |
| 1% × 4 | explore tabs (news, sports, entertainment, for_you) | 4% |

### Convergence Problem

After the entry-point "read" phase (10-20s of simulated reading on the landing page), **every** path converges to `goto_home()` → `x.com/home`. The code then enters Phase 2: scroll → scan → engage. So the behavioral signature is always:

```
land on page → read 10-20s → navigate to home → scroll identically → engage
```

For 59% of sessions, the landing page IS home, so there's no non-home reading phase at all.

### Impact

Accounts look like they always: (a) go to home, (b) scroll at constant speed, (c) engage for exactly 7-10 minutes. There's no variation in *browsing structure* — only in which tweets get clicked within that narrow structure.

### Candidates

- Stay on the entry page longer (2-5 minutes of varied scrolling)
- Navigate to a second destination before home (entry → trending/explore → home)
- Sometimes skip the "read" phase entirely and go straight to engagement
- Vary the entry-to-home timing per-session (5s to 120s)

---

## Finding 4: Session Timing Is Predictable

**Severity:** Medium

### Current Timing

In `twitteractivity.rs:44-49`:
```rust
if payload.get("duration_ms").is_none() {
    let random_duration = random_in_range(420_000, 600_000); // 7-10 min
    task_config.duration_ms = random_duration;
}
```

The session structure is always:
1. Phase 1: navigate to entry, read 10-20s, navigate to home (30-60s total)
2. Phase 2: scroll → scan → process → repeat until deadline or limits exhausted

### Behavioral Signature

Every session is a continuous engagement burst. Compare to a real human:
- Real user: might scroll for 2min → tab away for 30s → scroll 1min → tweet → scroll 3min → close
- Bot: scroll 9min straight, 50-100 actions, then stop

### Candidates

- Add randomized mid-session idle periods (30s to 5min of no activity)
- Vary session shape: short-burst (3min), long-casual (15min), deep-engage (25min)
- Allow early exit if engagement limits are hit fast (don't idle-scroll for remaining time)
- Add time-of-day gating: don't run sessions at 3 AM in account's timezone

---

## Finding 5: Scroll Patterns Are Uniform

**Severity:** Medium

### Current Code

In `twitteractivity.rs:137-141`:
```rust
let scroll_amount = if config.twitter_activity.scroll_amount_pixels > 0 {
    config.twitter_activity.scroll_amount_pixels
} else {
    profile.scroll.amount  // computed once
};
```

`scroll_amount` and `scroll_pause_ms` are computed once per session. Every scroll is the same distance with the same pause. The profile's `behavior_variance_pct` applies to **persona weights** only — not to scroll parameters.

### Impact

The scroll behavior looks machine-regular: 200px every 800ms, 40 times in a row. No back-scrolling, no variable-speed scrolling, no "stopping to read a tweet" pauses.

### Candidates

- Per-iteration randomize: `scroll_amount ± 20%`, `scroll_pause_ms ± 30%`
- Occasionally do a back-scroll (scroll up 100px after a down-scroll)
- Add "reading" pauses after certain scrolls (stand-in for "this tweet caught my eye")
- Use a scroll profile that evolves through the session (fast initial → slower as session ages)

---

## Finding 6: No Inter-Session Memory

**Severity:** High for autonomous operation

### Current State

Zero persistence. No SQLite, no file, no registry. The `SessionState` and `EngagementCounters` are purely in-memory and dropped when the task ends.

This means:
- Every session starts with the same engagement limits (5 likes, 3 retweets, etc.)
- No awareness of "I ran 3 sessions today already" or "yesterday I was heavy on likes"
- No day-over-day behavior variance
- No time-of-day awareness
- No learning from past rate-limits or shadow-ban signals

### Minimum Viable Persistence

Even a simple JSON file at `~/.config/auto-rust/twitter-state.json` with:
- `last_session_timestamp`
- `daily_action_counts` (like/retweet/follow/reply per day)
- `last_rate_limit_timestamp`
- `consecutive_failed_verifications`
- `timezone_offset`

...would enable day-over-day variance, time-of-day gating, and rate-limit backoff.

---

## Finding 7: Enhanced Sentiment — Partially Fixed, Partially Placebo (H4)

**Severity:** Medium

### What's Fixed

`extract_user_reputation()` in `sentiment/helpers.rs:461` now reads real data:
- `follower_count` from `followers_count` / `follower_count` fields
- `is_verified` from `is_verified`
- `account_age_days` parsed from `created_at` ISO 8601 timestamps
- `engagement_rate` computed from `total_likes / (total_tweets * followers)`
- `trust_score` derived from real signals

`extract_temporal_factors()` in `sentiment/helpers.rs:667` now reads real data:
- `hour_of_day`, `day_of_week`, `hours_since_post` parsed from timestamps
- `recency` computed via linear decay over 168 hours
- `is_peak_hour` derived from hour ranges
- `trending_bias` computed from tweet data

### What's Still Placebo

`extract_thread_context()` in `sentiment/helpers.rs:388` still hardcodes:

```rust
Some(ThreadContext {
    reply_count,                // REAL — read from replies array
    avg_reply_sentiment,        // REAL — computed from reply texts
    is_reply: false,            // HARDCODED
    is_quote: false,            // HARDCODED
    thread_depth: 0,            // HARDCODED
    conversation_indicators,    // REAL — detected from tweet text
})
```

### Impact on Scoring

In `sentiment/core.rs`, `analyze_enhanced()` calls `analyze_thread_context()` which uses:

| Field | Modifier | Always? |
|---|---|---|
| `is_reply` | +0.08 | **Always 0 — hardcoded false** |
| `is_quote` | +0.12 | **Always 0 — hardcoded false** |
| `thread_depth (1-2)` | +0.05 | **Always 0 — hardcoded 0** |
| `thread_depth (3-5)` | +0.10 | **Always 0** |
| `thread_depth (6-10)` | +0.15 | **Always 0** |
| `thread_depth (>10)` | +0.20 | **Always 0** |

Since `modulate_persona_by_sentiment()` in `scoring.rs:55-85` calls `analyze_enhanced()` when `enhanced_sentiment_enabled` is true, these always-zero modifiers silently reduce scoring accuracy. The `final_score` is 0.12-0.40 lower than it should be for replies/quotes/deep-thread tweets.

### Fix

Parse `is_reply`, `is_quote`, and `thread_depth` from tweet data. The candidate JSON already includes fields like `is_reply`, `is_quote` — or they can be inferred from the tweet structure (presence of `in_reply_to_status_id`, quoted tweet data, reply/thread metadata in the scrape).

---

## Finding 8: Remaining Existing Issues Still Current

### High (8)

| ID | Issue | File | Impact |
|---|---|---|---|
| H1 | Duplicate feed re-scan after dive | `engagement.rs:847` | [See Finding 2](#finding-2-duplicate-feed-re-scan-after-dive-h1) |
| H2 | Main loop `continue` bypasses deadline during long sleeps | `twitteractivity.rs:153-157` | Task can overshoot internal deadline by up to `candidate_scan_interval_ms` (2.5s default). Timeout wrapper prevents runaway, but session tracking is stale. |
| H3 | `should_dive` gates ALL non-like actions | `engagement.rs:327-330` | [See Finding 1](#finding-1-the-should_dive-bottleneck-h3) |
| H5 | `actions_taken` is write-only dead code | `twitteractivity.rs:129,211-221` | Threaded through `process_candidate()`, returned in `CandidateResult`, but never read. `session.counters.total_actions()` is the real source. |
| H6 | `PersonaStrategy` multiplier uses `.min()` not meaningful combination | `decision/strategies/persona.rs:236-237` | `interest_multiplier` can only reduce the cap for Full/Medium. Minimal level ignores it entirely. Counterintuitive: Minimal → 0.8, Medium → 0.3 for same negative-sentiment tweet. |
| H8 | `is_on_tweet_page()` false positive on home feed with open modal | `interact.rs:112-113` | Returns `true` if tweet detail modal is open on home page. Subsequent retweet/follow/reply actions operate on the modal DOM, potentially clicking wrong elements. |

### Medium (8)

| ID | Issue | Notes |
|---|---|---|
| M1 | `select_entry_point` uses `rand::random` without seed | Non-deterministic, bugs can't be reproduced. Fix: seed from `TaskConfig.seed`. |
| M2 | `consecutive_empty_scans` doesn't reset after scroll failures | Both counters exit at 3. If scrolling fails (network blip), it exits even if feed has content. |
| M3 | `dive_into_thread` pauses scrolling for 300 seconds | `Duration::from_secs(300)` — five minutes. If `goto_home()` after dive fails, scrolling is paused for rest of session. Should use actual dive duration. |
| M8 | `EngagementLimits::with_limits()` has 8 positional `u32` params | Easy to transpose. Builder pattern would be safer. |
| M9 | `extract_thread_context()` reports hardcoded `is_reply: false`, `is_quote: false`, `thread_depth: 0` | [See Finding 7](#finding-7-enhanced-sentiment--partially-fixed-partially-placebo-h4) |

### Low (11)

| ID | Issue | Notes |
|---|---|---|
| L1 | `RETWEET_BUTTON_SELECTOR` and `REPLY_BUTTON_SELECTOR` inconsistent quoting | Both are `r#"..."#` raw strings, RETWEET uses `\"` while REPLY uses unescaped `"`. |
| L2 | `like_at_position` JS uses `format!()` which conflicts with JS `{}` | Fragile — adding JS blocks with `{}` will break compile. |
| L3 | `actions_this_scan` shadowed in inner scope | Outer `let mut actions_this_scan = 0u32;` is shadowed by inner declaration in `.take()` block. Confusing. |
| L5 | Embedded `.js` files not validated at compile time | `include_str!()` compiles the string, but JS syntax errors only surface at runtime. |
| L6 | Error messages inconsistently prefixed | Some use `[twitter]`, most don't. Makes log filtering harder. |
| L7 | `clustered_engagement_pause` vs `human_pause` API inconsistency | Both profile-aware, but some pause functions fetch `behavior_runtime()` themselves. |
| L8 | `ensure_feed_populated()` returns `bool` but result ignored | Exported but never called in main flow. |
| L9 | `get_tweet_engagement_buttons()` returns `Value` (generic JSON) | Caller must know structure. No typed return. |
| L10 | `CandidateContext` destructuring in `process_candidate()` | All 8 fields rebound as locals. Defeats purpose of context struct. |
| L11 | `log_summary` computes duration via subtraction | Correct but assumes `saturating_sub`. |
| L12 | No integration tests for DOM interaction | All ~1000 tests are unit tests. Browser interaction is untested at unit level. |

---

## Finding 9: Dead Code Inventory

| Item | File | Reason |
|---|---|---|
| `actions_taken` | `twitteractivity.rs:129` | Write-only, shadowed, never read |
| `HOME_LOGO_SELECTOR` | `selectors.rs:79` | navigation.rs has its own literal |
| `read_full_thread()` | `dive.rs` | Never called from active code path |
| `ThreadCache` | `dive.rs:57-61` | Never populated or used by caller |
| `navigate_to_tweet()` | `interact.rs:150-160` | Not called in task flow |
| `check_selector_health()` | `navigation.rs:222-257` | Never called |
| `retry_with_fallback()` | `retry.rs:278-295` | Not called anywhere |
| `get_tweet_engagement_buttons()` | `feed.rs:242-247` | Not called in task flow |
| `ensure_feed_populated()` | `feed.rs:328-336` | Not called in task flow |
| `scroll_to_bottom_feed()` | `feed.rs:340-345` | Not called in task flow |
| `verify_element_hover()` | `humanized.rs:68-73` | Not called in task flow |
| `EngagementCheck` enum | `limits.rs:351-376` | Defined but never used |
| `DEFAULT_TWITTERACTIVITY_DURATION_MS` | `constants.rs:5` | Defined but not referenced |

---

## Finding 10: Issues Already Resolved by Code Changes

These were documented in the existing analysis but the code has since been fixed:

| Item | Original Issue | Fix |
|---|---|---|
| H7 | Cookie banner uses non-standard `:contains()` pseudo-selector | Replaced with `aria-label` + `data-testid` selectors + text-content fallback JS that iterates all buttons |
| M4 | LLM client created fresh for every reply/quote | Now uses `static LLM: OnceLock<Llm>` in `llm_instance()`. Created once, reused. |
| M5 | Regex compiled on every LLM validation call | All three regex patterns (`mentions_regex()`, `hashtags_regex()`, `banned_words_regex()`) use `OnceLock<Regex>`. |
| M7 | `simulate.rs::build_persona_weights()` duplicates `persona.rs::select_persona_weights()` | Now delegates directly to `select_persona_weights()` from the persona module. |
| M10 | `dismiss_signup_nag()` hard-disabled (causing hangs) | Function removed from codebase entirely. |
| L4 | `HOME_LOGO_SELECTOR` has `\"` in raw string | Value `r#"a[aria-label="X"]"#` is correct CSS. It IS dead code (unused), but not buggy. |

---

## Implementation Priorities

### P0 — Must Fix Before Autonomous Operation

| Priority | Item | Effort | Impact |
|---|---|---|---|
| P0.1 | **Separate dive from action selection (H3)** | ~40 lines | Fixes 80%-likes profile. Feed-safe actions (retweet/follow/bookmark) no longer gated behind `should_dive()`. |
| P0.2 | **Reset `next_candidate_scan` after dive (H1)** | ~3 lines | Prevents duplicate processing of same tweets post-dive. |
| P0.3 | **Inter-session persistence** | ~150 lines new file | Enables day-over-day variance, time-of-day gating, rate-limit backoff, shadow-ban detection. |

### P1 — Strongly Recommended

| Priority | Item | Effort | Impact |
|---|---|---|---|
| P1.1 | **Fix `extract_thread_context` hardcoded fields (M9/H4)** | ~20 lines | Enhanced sentiment scoring actually works for replies/quotes/deep threads. |
| P1.2 | **Variable scroll profile per-iteration** | ~15 lines | Scroll behavior varies within session, not just between sessions. |
| P1.3 | **Mid-session idle periods** | ~30 lines | Session looks like real browsing, not a continuous engagement burst. |

### P2 — Good for Safety

| Priority | Item | Effort | Impact |
|---|---|---|---|
| P2.1 | **Interruptible sleep (H2)** | ~10 lines | Respects deadline during idle periods. |
| P2.2 | **Reduce dive scroll pause to actual dive duration (M3)** | ~5 lines | Prevents session stall if `goto_home()` fails. |
| P2.3 | **Seed entry point selection (M1)** | ~5 lines | Makes navigation deterministic for debugging. |
| P2.4 | **Builder for `EngagementLimits` (M8)** | ~40 lines | Eliminates transpose bugs on 8-positional-param constructor. |

### P3 — Cleanup

| Priority | Item | Effort | Impact |
|---|---|---|---|
| P3.1 | **Remove 13 dead-code items** | ~100 lines removed | Reduces cognitive load, prevents misleading code. |
| P3.2 | **Fix `is_on_tweet_page` modal context (H8)** | ~20 lines | Prevents action mis-targeting on modal-vs-page. |
| P3.3 | **Consistent `[twitter]` log prefix** | ~15 lines | Makes log filtering reliable. |
