# Twitter Activity Task — Architecture Review (May 12, 2026)

## Overview

`src/task/twitteractivity.rs` is the orchestrator (~430 lines) for an automated Twitter/X engagement task. It delegates all implementation to 18 utility modules under `src/utils/twitter/`.

---

## High-Level Flow

```
run() → run_inner() → Phase 1 (navigation) → Phase 2 (feed scan loop)
              ↑                         ↓
         timeout wrapper          process_candidate() per tweet
```

1. **`run()`** — Entry point. Parses `TaskConfig` from JSON payload. If `simulate_only`, delegates to `run_simulation()`. Otherwise calls `run_inner()` wrapped in `run_with_timeout()`.
2. **Phase 1 — Navigation** — `phase1_navigation()`: selects a weighted random entry point (15 URLs, 59% home), navigates there, verifies login, dismisses popups.
3. **Phase 2 — Feed Scan Loop** — Scrolls → scans for candidate tweets → processes each candidate with `process_candidate()` → repeats until deadline or consecutive failure limits.

---

## Component Map

| Module | File | SLOC | Role |
|---|---|---|---|
| `twitteractivity.rs` | `src/task/twitteractivity.rs` | 433 | Orchestrator (this file) |
| `twitteractivity_state.rs` | `src/utils/twitter/` | 630 | `TaskConfig`, `SessionState`, `CandidateContext`, `CandidateResult`, `TweetActionTracker` |
| `twitteractivity_navigation.rs` | `src/utils/twitter/` | 680 | Entry points, login verify, popup detection, home nav |
| `twitteractivity_feed.rs` | `src/utils/twitter/` | 503 | Feed scrolling, candidate identification, scroll progress |
| `twitteractivity_engagement.rs` | `src/utils/twitter/` | ~1400 | `process_candidate()`, sentiment modulation, action execution |
| `twitteractivity_limits.rs` | `src/utils/twitter/` | 691 | `EngagementCounters`, `EngagementLimits`, rate limiting |
| `twitteractivity_persona.rs` | `src/utils/twitter/` | 589 | `PersonaWeights`, probability-based decision functions |
| `twitteractivity_simulation.rs` | `src/utils/twitter/` | 422 | Deterministic simulation engine (no browser) |
| `twitteractivity_constants.rs` | `src/utils/twitter/` | 11 | Timing constants |
| `twitteractivity_errors.rs` | `src/utils/twitter/` | 166 | Error classification (transient/permanent/fatal) |
| `twitteractivity_humanized.rs` | `src/utils/twitter/` | 343 | Human-like pauses, micro-movements, reading simulation |
| `twitteractivity_interact.rs` | `src/utils/twitter/` | 809 | DOM interaction: like, retweet, reply, follow, bookmark |
| `twitteractivity_dive.rs` | `src/utils/twitter/` | 625 | Thread diving, reply extraction, `ThreadCache` |
| `twitteractivity_popup.rs` | `src/utils/twitter/` | 498 | Cookie banner, signup nag, overlay dismissal |
| `twitteractivity_selectors.rs` | `src/utils/twitter/` | 311 | Centralized JS selector snippets (embedded `.js` files) |
| `twitteractivity_retry.rs` | `src/utils/twitter/` | 348 | Exponential backoff retry, circuit breaker |
| `twitteractivity_llm.rs` | `src/utils/twitter/` | 198 | LLM-powered reply/quote generation |
| `twitteractivity_llm_validation.rs` | `src/utils/twitter/` | 255 | LLM output sanitization, banned words filter |
| `twitteractivity_llm_execute.rs` | `src/utils/twitter/` | 268 | Quote tweet DOM interaction flow |
| **Total** | **19 files** | **~8200** | |

---

## Key Data Structures

| Struct | Defined In | Purpose |
|---|---|---|
| `TaskConfig` | `state.rs` | Parsed from JSON payload; holds duration, limits, flags |
| `SessionState` | `state.rs` | Groups counters + limits + action_tracker + deadline |
| `CandidateContext` | `state.rs` | Context for `process_candidate()` |
| `CandidateResult` | `state.rs` | Result from `process_candidate()` (replaces 5-tuple) |
| `TweetActionTracker` | `state.rs` | Prevents rapid action chains on same tweet |
| `EngagementCounters` | `limits.rs` | Per-action counters (likes, retweets, etc.) |
| `EngagementLimits` | `limits.rs` | Max per-action + total limits |
| `PersonaWeights` | `persona.rs` | Probability weights for each action type |
| `SentimentTemplates` | `state.rs` | Reply/quote text templates by sentiment |
| `SimulationReport` | `simulation.rs` | Deterministic simulation output |
| `ThreadCache` | `dive.rs` | Cached thread data for LLM processing |
| `RetryConfig` | `retry.rs` | Retry parameters (attempts, backoff, jitter) |
| `CircuitBreaker` | `retry.rs` | Prevents cascade failures (protocol/interaction lvl) |

---

## Flow Detail: `run_inner()`

### Initialization (lines 82-149)
1. Build persona weights from config + payload overrides + behavior profile
2. Create `EngagementLimits` from config
3. Create `SessionState` (counters + limits + tracker + deadline)
4. Set up scroll/candidate-scan timing intervals from config or profile

### Main Loop (lines 151-242)
```
while !session.is_expired():
    if now < next_candidate_scan: sleep, continue
    if now >= next_scroll: scroll, update next_scroll
    candidates = identify_engagement_candidates(api)
    for tweet in candidates (up to candidate_count):
        ctx = CandidateContext { tweet, persona, task_config, api, ... }
        result = process_candidate(ctx, ...)
        if result.should_break: break
    if consecutive_empty_scans >= 3: break
    if session.remaining_time() < 500ms: break
```

### Termination (lines 244-246)
- `log_summary()` — prints engagement summary + remaining limits

---

## Flow Detail: `process_candidate()` (engagement.rs)

1. **Budget check** — if per-scan quota reached, return `should_break: true`
2. **Sentiment analysis** — `modulate_persona_by_sentiment()`: enhances or suppresses engagement weights based on tweet sentiment
3. **Smart decision** — `handle_engagement_decision()`: rule-based (or LLM) decision engine; can skip tweet
4. **Action selection** — checks each action type against persona probability + limits + action tracker
5. **Thread dive** — if any non-like action is needed, `dive_into_thread()` → opens tweet detail view
6. **Action execution** — performs selected actions via DOM interaction functions with retry
7. **Depth-first engagement** — if dive succeeded and root action was taken, `engage_replies()` on the thread
8. **Navigation back** — returns to home feed after dive, resumes scroll timing

---

## Key Architectural Decisions

### ✅ Thin orchestrator pattern
`twitteractivity.rs` is ~430 lines of orchestration. All logic is in utility modules. This keeps the task entry point readable.

### ✅ Timeout boundary in `run()`
`run_with_timeout()` wraps `run_inner()` so timeout enforcement is at the outermost layer. No timeout logic leaks into business logic.

### ✅ SessionState consolidation
Previously counters, limits, tracker, and deadline were passed separately. Now they're groupped in `SessionState` — reduces parameter counts.

### ✅ CandidateContext/Result
`CandidateContext` and `CandidateResult` replaced loose tuples in `process_candidate()` — clearer API boundaries.

### ✅ Deterministic simulation
`run_simulation()` is a pure function with seeded RNG. No browser needed. Produces reproducible logs for testing.

### ✅ Error classification
`ErrorClassifier` trait on `anyhow::Error` / `io::Error` enables retry-or-fail decisions without ad-hoc error matching.

### ✅ Retry with CircuitBreaker
`retry_with_backoff()` respects error class. `CircuitBreaker` prevents cascade failures after N consecutive failures.

### ✅ Retry: retry_with_backoff uses human_pause instead of tokio::time::sleep
`retry_with_backoff()` uses `human_pause()` (profile-aware) between retries rather than raw `tokio::time::sleep`. This prevents rigid timing patterns that could look bot-like.

### ✅ LLM integration is fallback-graceful
`generate_reply()` / `generate_quote_commentary()` fall back to template text if LLM fails, times out, or produces empty output after sanitization. No hard dependency.

### ✅ LLM output sanitization
`validate_reply()` strips markdown formatting, emojis, mentions, hashtags, and truncates to 270 chars at word boundary. The banned-words list catches common AI-sounding phrases (52 entries).

### ✅ Action verification
After like/retweet/reply/bookmark, the code verifies the action took effect by checking DOM state changes (e.g., button text from "like" → "unlike", composer closed after reply).

---

## Potential Concerns & Observations

### 🔶 `process_candidate()` is very long (~630 lines)
Almost all action types (like, retweet, quote, follow, reply, bookmark) are handled in a single match block with deep nesting. Each branch has similar retry+increment+pause patterns. Could benefit from a strategy pattern.

### 🔶 `should_dive` gate limits non-like actions
If `should_dive()` returns false but the persona wants to retweet/follow/reply, those are dropped and only "like" is kept (line 329 `actions_to_do.retain(|&action| action == "like")`). This means retweet/quote/follow/reply/bookmark are all gated behind `thread_dive_prob`. Design intent: these actions need the detail view.

### 🔶 Duplicated limit checks
Both `run_inner()` and `process_candidate()` check limits. `SessionState::is_action_allowed()` is unused in `process_candidate()` (it uses raw limits methods instead). `SessionState::record_action()` is defined but `process_candidate()` manually increments counters + tracker.

### 🔶 LLM validation regexes compiled on every call
`remove_mentions()` and `remove_hashtags()` compile `regex::Regex` on each invocation via `regex::Regex::new()`. These should be `once_cell::sync::Lazy` statics.

### 🔶 Cookie banner selectors use `:contains()` pseudo-selector
`button:contains('Accept all')` — `:contains()` is not a standard CSS selector and may not work in all browser automation contexts. This may silently fail (already logged by `dismiss_cookie_banner()` returning `Ok(false)`).

### 🔶 `dismiss_signup_nag()` is disabled (returns `Ok(false)`)
The function exists but is hard-disabled, with a comment saying it causes hangs. This is a known gap.

### 🔶 `extract_tweet_context()` (llm.rs) JS has a bug
Line 128: `querySelectorAll('article [data-testid="tweet"] [dir="auto"]')` — this selects `[dir="auto"]` elements nested inside `[data-testid="tweet"]` inside `article`, but the structure is likely `article[data-testid="tweet"]`. The space before `[data-testid]` makes it a descendant selector, so it looks for `[data-testid="tweet"]` anywhere inside any `article`. Also, the author extracted via `var authorEl = document.querySelector('[data-testid="tweet"] [dir="auto"]')` — this always gets the *first* tweet's author, not each reply's author. So all replies get the root tweet's author.

### 🔶 Simulation and real execution share no persona building path
`simulation.rs::build_persona_weights()` is a near-duplicate of `persona.rs::select_persona_weights()`. They parse the same config + weights fields but are separate code paths.

### 🔶 `EngagementLimits::with_limits()` has 8 positional u32 params
Easy to mix up at call sites. A builder pattern or named-fields struct would be safer.

### 🔶 Selector constants mix escaped and unescaped quotes
`RETWEET_BUTTON_SELECTOR` uses `\"` (escaped for Rust raw string) while `REPLY_BUTTON_SELECTOR` on line 118 uses unescaped `"`. Both are `r#"..."#` raw strings but the quoting style is inconsistent.

### 🔶 No real integration tests for DOM interaction
All tests are unit tests (verifying JS strings, data structures, probability distributions). The actual browser interaction is untested at the unit level — only tested via end-to-end runs.

---

## Testing Coverage

| Test Module | File | Tests | Type |
|---|---|---|---|
| `config_tests` | `twitteractivity.rs` | 1 | Unit |
| `navigation_tests` | `twitteractivity.rs` | 1 | Unit |
| `summary_tests` | `twitteractivity.rs` | 1 | Unit |
| `timeout_tests` | `twitteractivity.rs` | 2 | Async unit |
| `display_tests` | `state.rs` | 1 | Unit |
| `read_u64_tests` | `state.rs` | 3 | Unit |
| `read_u32_tests` | `state.rs` | 2 | Unit |
| `payload_tests` | `state.rs` | 4 | Unit |
| `navigation tests` | `navigation.rs` | ~25 | Unit + property |
| `feed tests` | `feed.rs` | ~12 | Unit (JS verification) |
| `engagement tests` | `engagement.rs` | ~30 | Unit + async + property |
| `limits tests` | `limits.rs` | ~25 | Unit |
| `persona tests` | `persona.rs` | ~20 | Unit |
| `simulation tests` | `simulation.rs` | 4 | Unit |
| `classification_tests` | `errors.rs` | 2 | Unit |
| `detection_tests` | `errors.rs` | 2 | Unit |
| `duration_tests` | `humanized.rs` | ~10 | Unit |
| `selector_tests` | `humanized.rs` | ~12 | Unit |
| `interact tests` | `interact.rs` | ~12 | Unit |
| `dive tests` | `dive.rs` | ~10 | Unit |
| `popup tests` | `popup.rs` | ~20 | Unit |
| `selectors tests` | `selectors.rs` | ~12 | Unit |
| `retry config/delay/circuit tests` | `retry.rs` | 3 | Unit |
| `llm validation tests` | `llm_validation.rs` | ~12 | Unit |
