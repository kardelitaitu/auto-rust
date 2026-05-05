# Twitter Activity — Implementation Plan

This document covers implementation milestones, rollout strategy, known gaps, references, and decisions log.

---

## 9. Dependencies & Risks

### 9.1 Dependencies (No new crates expected)

**Reuse existing utilities:**
- `crate::utils::navigation::{goto, wait_for_load}` (used internally by `TaskContext::navigate`)
- `crate::utils::scroll::{random_scroll, scroll_to_bottom, scroll_into_view}`
- `crate::utils::mouse::{cursor_move_to, click_at}`
- `crate::utils::timing::human_pause`
- `crate::utils::page_size::{get_viewport, get_element_center}`
- `crate::utils::block_heavy_resources` — NOT used for Twitter (see scope)
- `crate::utils::profile::{BrowserProfile, randomize_profile}`
- `crate::config`
- `crate::task::perform_task`

No new third-party dependencies. All functionality achievable with `chromiumoxide`, `serde_json`, `rand`, `tokio`, `log`, `anyhow` which are already in `Cargo.toml`.

### 9.2 Risk Matrix

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **Twitter DOM volatility** (selector breakage) | High | Task failure | • 3–4 fallback selectors per element<br>• Centralized selector definitions (`selectors.rs`)<br>• Log which selector succeeded; monitor failure patterns |
| **Rate-limit / ban** | Medium | Task fail, IP/profile ban | • Conservative limits (5L/3RT/2F per session)<br>• Random entry points<br>• 9–14min session duration<br>• Human-like cursor curves |
| **Popup/modal flood** | High | Interaction blocked | • `close_all_modals()` after every navigation<br>• ESC key fallback<br>• Log count of modals encountered |
| **Slow network / timeout** | Medium | Navigation fails | • 20s per-cycle navigation budget (15+5)<br>• Graceful degradation: navigation failure → skip cycle, continue |
| **Element stale / DOM mutation** | Medium | Click fails | • Re-query element immediately before click<br>• After scroll/expand, wait 500–1000ms for DOM reflow |
| **"Like" already engaged** | High | Duplicate action error | • Pre-check button state: if `[data-testid="unlike"]` visible → skip |
| **"Follow" already in progress** | Medium | Button shows "Pending" | • Check for `[data-testid="following"]` or `[data-testid="pending"]` → skip |
| **Retweet confirmation modal changes** | Medium | RT fails | • Confirm button has fallback selectors; verify modal closes |
| **Bookmark UI complexities** | High | Skipped V1 | • Bookmark feature disabled V1 (limit = 0, probability = 0) |

### 9.3 Selector Maintenance Strategy

Centralize all Twitter CSS selectors in `src/utils/twitter/twitteractivity_selectors.rs`. Implement a **selector health check** at startup (optional):

```rust
pub async fn validate_selectors(ctx: &TaskContext) -> Result<()> {
    use crate::utils::twitter::twitteractivity_selectors::*;
    let critical = [
        (BTN_LIKE, "like button"),
        (BTN_RETWEET, "retweet button"),
        (BTN_FOLLOW, "follow button"),
    ];

    for (selector, name) in &critical {
        let js = format!("document.querySelector({}) !== null", serde_json::to_string(selector)?);
        let exists = ctx.page().evaluate(js).await?.value()
            .and_then(|v| v.as_bool()).unwrap_or(false);
        if !exists {
            warn!("Twitter selector health: {name} ({}) not found", selector);
        }
    }
    Ok(())
}
```

**Update cadence**: Manual update to selectors as needed when x.com UI changes.

---

## 10. Implementation Milestones

### Phase 0 — Foundations (Day 1)

**M0.1: Config Extension**
- [ ] Add `TwitterConfig` struct to `src/config.rs`
- [ ] Add TOML deserialization for `[twitter]` section
- [ ] Add `apply_env_overrides()` handlers for all `TWITTER_*` vars
- [ ] Add `default_twitter_config()` fallback in `load_code_config()`
- [ ] Add `validate_config()` checks for Twitter fields (weights sum ~100, probs ≤ 1.0, limits ≥ 0)
- [ ] Write unit test: `load_config()` + env override integration

**M0.2: Task Registration**
- [ ] Create `task/twitteractivity.rs` (stub: `run()` returning `TaskResult::success(0)`)
- [ ] Register in `task/mod.rs` (`pub mod twitteractivity;`)
- [ ] Add match arm in `perform_task()` (in `task/mod.rs`)
- [ ] Add validation schema in `src/validation/task.rs` (any object OK for V1)
- [ ] Verify task discovered: `cargo run twitteractivity` reaches stub

**M0.3: Profile Extension (Twitter-specific)**
- [ ] Add `dive_probability: ProfileParam` field to `BrowserProfile` in `src/utils/profile.rs`
- [ ] Add `#[serde(default)]` to maintain backward compatibility with existing configs
- [ ] Provide default of `p(0.35, 20.0)` in `BrowserProfile::default()` (or in each preset)
- [ ] Initialize `dive_probability` in all 21 BrowserProfile preset constructors (average(), teen(), senior(), etc.)
- [ ] Unit test: `BrowserProfile::from_preset()` returns profile with non-zero `dive_probability`

---

### Phase 1 — Core Infrastructure (Day 1–2)

**M1: Navigation & Entry Points**
- [ ] Implement `select_entry_point()` in `src/utils/twitter/twitteractivity_navigation.rs`
- [ ] `TwitterAgent::run_cycle()` calls `crate::utils::twitter::select_entry_point(&config).await` → URL
- [ ] Then calls `ctx.navigate(&url, 15000)` + `ctx.wait_for_load(5000)`

**M2: Feed Scanning & Tweet Extraction**
- [ ] Create `src/utils/twitter/twitteractivity_feed.rs`
- [ ] Implement `find_random_tweet(ctx: &TaskContext) -> Result<Option<TweetMetadata>>` using JS evaluation
- [ ] Add `TweetMetadata` struct (id, author_handle, author_name, text_preview, has_media, selector_used)
- [ ] Create `src/utils/twitter/twitteractivity_selectors.rs` with all selectors
- [ ] Integration: `TwitterAgent` verifies `find_random_tweet` returns at least 1 tweet per cycle

**M3: Popup Handler**
- [ ] Create `src/utils/twitter/twitteractivity_popup.rs`
- [ ] Implement `close_all_modals(ctx: &TaskContext)` using JS clicks + ESC key
- [ ] Inject in `run_cycle()` after navigation arrives

---

### Phase 2 — Engagement Actions (Day 2)

**M4: Like Action**
- [ ] `src/utils/twitter/twitteractivity_interact.rs` already exists with `like_tweet()` implementation
- [ ] Use JS to check for `[data-testid="unlike"]` to skip if already liked
- [ ] Hover → click via `ctx.move_mouse_to(x, y)` + `ctx.click(x, y)`
- [ ] Verify state change by polling for `unlike` button (max 2s)
- [ ] Integration test: run 1 cycle → verify like appears on test account (manual check)

**M5: Retweet Action**
- [ ] Implement `retweet_tweet()` in same module
- [ ] Click retweet → modal → click confirm (native RT, no quote)
- [ ] Verify modal closes (best-effort)
- [ ] Integration: RT a known tweet, verify on account

**M6: Follow Action**
- [ ] Implement `follow_user()` in same module
- [ ] Check for `[data-testid="following"]` to skip if already following
- [ ] Click follow button, verify state change (best-effort)
- [ ] Integration: follow a test user, verify state change

**M7: Bookmark Stub (disabled V1)**
- [ ] Implement `bookmark_tweet()` returning `Err("disabled in V1")`
- [ ] Ensure config `max_bookmarks=0` blocks this action at limit guard

---

### Phase 3 — Agent Logic & Orchestration (Day 2–3)

**M8: TwitterAgent State Machine**
- [ ] Implement `TwitterAgent` inside `task/twitteractivity.rs`
- [ ] Struct: `TwitterAgent { ctx: TaskContext, profile: BrowserProfile, config: TwitterConfig, state: AgentState, persona: TwitterPersona }`
- [ ] `run_session()`: loop cycles until `max_cycles` or `max_duration_sec` reached
- [ ] `run_cycle()` full flow:
  1. Budget check → return `SessionComplete` if done
  2. `crate::utils::twitter::select_entry_point(&config).await?` → URL
  3. `ctx.navigate(&url, 15000)` + `ctx.wait_for_load(5000)` with timeout wrapper
  4. `crate::utils::twitter::close_all_modals(&ctx).await`
  5. Determine phase (warmup/active/cooldown)
  6. Dive decision: `rand::random::<f64>() < profile.dive_probability.random() * persona.dive_probability_multiplier()`
  7. `crate::utils::twitter::find_random_tweet(ctx)` → `TweetMetadata`
  8. `crate::utils::twitter::click_tweet(ctx, tweet)`
  9. `simulate_reading(tweet)` — internal method with `ctx.page().evaluate` scrolls + `ctx.pause`
  10. `crate::utils::twitter::load_tweet_context(ctx, tweet)` → `TweetContext`
  11. Sentiment guard: if config enabled, `crate::utils::twitter::contains_negative_sentiment(&text, &blocklist)` → skip
  12. Build available actions via counters vs limits
  13. Roll engagement using weighted probabilities
  14. Execute selected action via `crate::utils::twitter::{like_tweet, retweet_tweet, follow_user}` (others disabled)
  15. Update counters, log outcome, `ctx.pause(3000, 50)`
- [ ] Helper methods: `increment_counter`, `available_engagement_actions`, `roll_engagement`

**M9: Limits & Counters**
- [ ] `EngagementCounters` as member of `AgentState`; `can_engage()` and `increment()` methods
- [ ] Log when an action skipped due to limit

**M10: Sentiment Guard**
- [ ] `src/utils/twitter/twitteractivity_sentiment.rs` with `contains_negative_sentiment(text, &blocklist) -> bool`
- [ ] Config flag `block_negative_engagement` controls check

**M11: Persona Mapping**
- [ ] `src/utils/twitter/twitteractivity_persona.rs` with `TwitterPersona::from_profile(profile)` and `dive_probability_multiplier()`

---

### Phase 4 — Config & Validation (Day 3)

**M12: Config Integration**
- [ ] Extend `Config` struct with `twitter: TwitterConfig`
- [ ] Add `default_twitter_config()` in `src/config.rs`
- [ ] Merge TOML + env overrides in `apply_env_overrides()`
- [ ] Add validation in `validate_config()`:
  - `max_cycles >= min_cycles`
  - `max_duration_sec >= min_duration_sec`
  - engagement probabilities sum ≤ 1.0 (warning only)
  - entry point weights sum to ~100 (±5 tolerance)

**M13: Validation Schema**
- [ ] Extend `src/validation/task.rs` with `validate_twitteractivity()` accepting any JSON object for V1
- [ ] Future: validate optional fields like `cycles` or `max_likes`
- [ ] Unit test: valid payload accepted, invalid rejected

---

### Phase 5 — Metrics & Logging (Day 3)

**M14: Task Metrics**
- [ ] `TwitterAgent` accumulates `TwitterMetrics` (defined in `05-metrics.md`)
- [ ] At task end, log final summary line with cycles, engagement counts, duration, persona
- [ ] Ensure all log lines include `[twitter]` tag

**M15: Run Summary Compatibility**
- [ ] `perform_task()` returns `TaskResult::success()` for any successful session (even if zero engagements)
- [ ] `run-summary.json` from `MetricsCollector` captures top-level success count

---

### Phase 6 — Integration & Polish (Day 3)

**M16: End-to-End Dry Run**
- [ ] `cargo run twitteractivity` on machine with browser connected
- [ ] Observe logs: entry URL, tweet found, actions, pauses
- [ ] Verify no panics / unwraps
- [ ] Check counters never exceed limits
- [ ] Monitor modal dismissals; add selectors to `twitteractivity_popup` if needed

**M17: Error Path Verification**
- [ ] Simulate network timeout (block x.com) → verify navigation timeout handling, cycle continues
- [ ] Simulate selector rot (rename a selector) → verify graceful skip (no panic)
- [ ] Simulate action failure (e.g., make like button detached) → verify warning logged and cycle continues

**M18: Config Edge Cases**
- [ ] Test `TWITTER_ENABLED=false` → task exits early with info log
- [ ] Test `TWITTER_MAX_CYCLES=0` → zero cycles, task completes (edge but valid)
- [ ] Test probabilities sum > 1.0 → warning log, but proceeds

**M19: Documentation & Final Review**
- [ ] Update `README.md` with `twitteractivity` usage example
- [ ] Add sample `config/default.toml` Twitter section
- [ ] Document `TWITTER_*` environment variables
- [ ] Code review: error handling, no `unwrap()`, proper logging

---

### Phase 7 — Deferred (V2, Future)

**V2 Roadmap** (not in initial scope):
- [ ] LLM-powered replies & quote tweets (`twitteractivity_llm.rs`)
- [ ] Sentiment analysis with NLP wordlists (VADER-style)
- [ ] Bookmark action implementation
- [ ] Reply action implementation (keyboard typing simulation)
- [ ] Dynamic entry point weights (per-session randomization ±10%)
- [ ] Advanced persona behaviors: hesitation micro-movements, overscroll, tab-switch simulation
- [ ] `run-summary.json` embedded per-task metadata for Twitter breakdown
- [ ] Dashboard: real-time metrics UI (web sockets)

---

## 11. Success Criteria (Definition of Done)

V1 is complete when **all** of the following are verified:

**Functional Requirements**
- [ ] `cargo run twitteractivity` executes without panics or unwrap crashes
- [ ] Task completes 5–10 cycles (random within config range)
- [ ] Total session duration falls within 540–840 seconds (9–14min)
- [ ] At least one engagement (like/retweet/follow) occurs per session on average (configurable probability ≥ 0.1)
- [ ] Per-session limits respected: `likes ≤ max_likes`, `retweets ≤ max_retweets`, `follows ≤ max_follows` (hard stops)
- [ ] Sentiment guard (when enabled) blocks engagement on tweets containing blocklist keywords

**Reliability Requirements**
- [ ] Navigation failures (timeout, DNS error) do NOT crash the task — individual cycles are skipped, session continues
- [ ] Missing UI elements handled gracefully with info-level logs
- [ ] Modals dismissed automatically; persistent modal does not crash task
- [ ] No `anyhow!` or `panic!` in logs across 10 consecutive runs

**Observability Requirements**
- [ ] Every log line includes `[twitter]` tag
- [ ] Per-cycle logs: entry URL, tweet found, action taken (or skip reason), duration
- [ ] Final summary line: `[twitter] Session complete: cycles=X, likes=Y, retweets=Z, follows=W, skipped=N, duration=Ts`
- [ ] `run-summary.json` exports `task="twitteractivity"` with success status

**Configuration Requirements**
- [ ] All config fields in `[twitter]` section loaded from `config/default.toml`
- [ ] All `TWITTER_*` environment variables override TOML values
- [ ] Validation passes at startup: weights sum ~100, probabilities ≤ 1.0, non-negative limits
- [ ] Task handles `TWITTER_ENABLED=false` gracefully (early exit with info log)

**Testing Requirements**
- [ ] Unit tests pass: `cargo test`
- [ ] Clippy clean: `cargo clippy --all-targets --all-features`
- [ ] Build succeeds: `cargo build --all-features`
- [ ] Manual integration: run on live x.com, verify actions appear or logs show engagements attempted
- [ ] Counters reset on subsequent runs (no cross-session state bleed)

**Code Quality Requirements**
- [ ] No `unwrap()` or `expect()` on `Result`/`Option` in production code
- [ ] All public functions have `#[allow(dead_code)]` removed (or justified)
- [ ] Module docs (`//!`) explain purpose
- [ ] Functions have inline comments for non-obvious logic
- [ ] Error types use `anyhow::Result` with context-rich messages

**Deployment Readiness**
- [ ] `README.md` updated with `twitteractivity` usage examples
- [ ] Environment variable table included in docs
- [ ] `config/default.toml` sample includes `[twitter]` section
- [ ] Known limitations documented (no LLM, no bookmarks, no replies)

---

## 12. Rollout & Monitoring Plan

### Phase 1 — Shadow Mode (Week 1)
- Run task on 1–2 sessions with `max_likes=1`, `max_retweets=0`, `max_follows=0`
- Observe logs: entry point distribution, tweet find success rate, modal frequency
- No actual engagement (limits near zero) → pure dry-run
- Verify: no errors, selector health OK, session duration within budget

### Phase 2 — Conservative Engagement (Week 2)
- Increase limits to `likes=1`, `retweets=1`, `follows=1`
- Verify actions appear on test account
- Monitor for rate-limit responses (403, 429 HTTP codes in DevTools)
- If any signs of throttling, reduce limits further

### Phase 3 — Full Limits (Week 3)
- Enable full limits: `likes=5`, `retweets=3`, `follows=2`
- Run 10–20 sessions, collect logs, check `run-summary.json`
- Verify no bans, no CAPTCHAs, UI still navigable

### Phase 4 — Tuning
- Adjust probabilities if engagement rate too low/high
- Tweak entry point weights if certain pages yield poor dive rates
- Update selectors in `src/utils/twitter/twitteractivity_selectors.rs` as Twitter UI evolves

---

## 13. Known Gaps & Future Work

| Feature | Status | Notes |
|---------|--------|-------|
| LLM-powered replies/quotes | Deferred V2 | Requires local/cloud LLM integration, prompt engineering |
| Bookmark action | Deferred V2 | UI involves modal; low engagement value |
| Sentiment analysis (NLP) | Deferred V2 | Keyword-only in V1; consider VADER or transformer in V2 |
| Advanced persona behaviors (hesitation, distraction) | Deferred V2 | Profile already encodes some via `dive_probability`; micro-move not yet used |
| Quote Tweet | Deferred V2 | Complex flow (compose modal), low priority |
| Dynamic entry weight jitter | Deferred V2 | Static weights OK for V1 |
| Thread engagement (click "Show more replies") | Deferred V2 | Currently only surface-level tweets |
| Video/audio playback | Blocked by design | Media allowed but not explicitly blocked; no playback tracking |
| Cookie consent handling | Basic in `twitteractivity_popup.rs` | May need expansion if new consent banners appear |

---

## 14. References

- Node.js Reference: `auto-ai/tasks/api-twitterActivity.js`
- Browser automation patterns: `task/cookiebot.rs`, `task/pageview.rs`
- Utilities: `src/utils/{navigation,scroll,mouse,timing,profile,blockmedia,page_size}.rs`
- Config system: `src/config.rs`
- Task orchestration: `src/orchestrator.rs`, `task/mod.rs`
- Metrics: `src/metrics.rs`
- Validation: `src/validation/task.rs`
- Helper modules: `src/utils/twitter/twitteractivity_*.rs` (all in one directory)

---

## 15. Decisions Made (Confirmed)

Based on user feedback (2026-04-17), the following decisions are confirmed:

| # | Decision Topic | Chosen Option | Notes |
|---|----------------|---------------|-------|
| 1 | **LLM Integration** | V1: Exclude replies & quotes (no LLM) | Flags `reply_with_ai=false`, `quote_with_ai=false`. V2 can enable via config |
| 2 | **Sentiment** | Keyword-only, off-by-default | `block_negative_engagement = false`. Simple substring blocklist |
| 3 | **Bookmarks** | Disabled in V1 | `max_bookmarks=0`, `bookmark_probability=0` |
| 4 | **Media Blocking** | Do NOT use `block_heavy_resources` | Twitter needs images for realism; let media load naturally |
| 5 | **Engagement Roll Timing** | After click + simulate reading | Sequence: dive (click) → simulate_reading() → load context → roll → act |
| 6 | **Entry Point Weights** | Use provided weighted list | Home=59%, Explore=32%, Connect=4%, Supplementary=5% |
| 7 | **Entry Point Dynamism** | Static weights (no jitter) | Same distribution every cycle; V2 may add jitter |
| 8 | **Rate Limits** | Likes=5, Retweets=3, Follows=2 | Conservative caps; configurable via env |
| 9 | **Sentiment Blocklist** | ~10–15 conservative keywords | Avoid false positives; expand after manual review |
| 10 | **Validation Schema** | Simple object check (any JSON) | No required fields; extensible for V2 |
| 11 | **Fallback Selector Order** | First-match cascade | Try article→cellInnerDiv→fallback→XPath (implemented via JS querySelectorAll on each in turn) |
| 12 | **Action Failure Handling** | Skip silently, continue | No retry; log warning, proceed to next cycle |
| 13 | **AI Reply/Quote Flags** | `reply_with_ai = false`, `quote_with_ai = false` | Default off; require explicit config to enable (V2) |
| 14 | **Persona Support** | Extend `BrowserProfile` with `dive_probability` | Base engage rate per profile; persona provides multiplier |

**Implementation implications:**
- `Engagement` enum includes Reply/Quote but they are unreachable unless flags set true
- `simulate_reading()` phase added after tweet click, before context extraction
- No call to `block_heavy_resources()` in Twitter task
- `TwitterAgent::persona` derived from profile; `dive_probability` field added to all presets

---

*This document is part of the Twitter Activity task planning suite. See [README.md](README.md) for navigation.*
