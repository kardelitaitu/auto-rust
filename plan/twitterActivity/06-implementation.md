# Twitter Activity — Implementation Plan

This document covers implementation milestones, rollout strategy, known gaps, references, and decisions log.

---

## 9. Dependencies & Risks

### 9.1 Dependencies (No new crates expected)

**Reuse existing utilities:**
- `crate::utils::navigation::{goto, wait_for_load}`
- `crate::utils::scroll::{random_scroll, scroll_to_bottom}`
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

Centralize all Twitter CSS selectors in `src/utils/twitter/selectors.rs`. Implement a **selector health check** at startup (optional):

```rust
pub async fn validate_selectors(page: &Page) -> Result<()> {
    let critical = [
        (BTN_LIKE, "like button"),
        (BTN_RETWEET, "retweet button"),
        (BTN_FOLLOW, "follow button"),
    ];

    for (selector, name) in &critical {
        if page.querySelector(selector).await?.is_none() {
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
- [ ] Create `src/task/twitteractivity.rs` (stub: `run()` returning `TaskResult::success(0)`)
- [ ] Register in `src/task/mod.rs` (`pub mod twitteractivity`)
- [ ] Add match arm in `perform_task()` (in `task/mod.rs`)
- [ ] Add validation schema in `src/validation/task.rs` (any object OK for V1)
- [ ] Verify task discovered: `cargo run twitterActivity` reaches stub

**M0.3: Profile Extension (Twitter-specific)**
- [ ] Add `dive_probability: ProfileParam` field to `BrowserProfile` in `src/utils/profile.rs`
- [ ] Add `#[serde(default)]` to maintain backward compatibility with existing configs
- [ ] Provide default of `p(0.35, 20.0)` in `BrowserProfile::default()` (or in each preset)
- [ ] Initialize `dive_probability` in all 21 BrowserProfile preset constructors (average(), teen(), senior(), etc.)
- [ ] Unit test: `BrowserProfile::from_preset()` returns profile with non-zero `dive_probability`

---

### Phase 1 — Core Infrastructure (Day 1–2)

**M1: Navigation & Entry Points**
- [ ] Create `src/task/twitter_navigation.rs`
- [ ] Implement `weighted_pick()` utility
- [ ] Define `ENTRY_POINTS` constant with weights per spec
- [ ] Implement `select_entry_point()` (category → concrete URL)
- [ ] Implement `navigate_to()` with 20s timeout budget
- [ ] Add unit test: `select_entry_point()` returns valid URL for all categories
- [ ] Integration: `TwitterAgent::run_cycle()` Phase 1–2 logs entry URL

**M2: Feed Scanning & Tweet Extraction**
- [ ] Create `src/task/twitter_feed.rs`
- [ ] Implement `find_random_tweet()` using selector family cascade
- [ ] Write `extract_metadata_from_element()` with JS evaluate
- [ ] Add `TweetMetadata` struct (id, author, text_preview, has_media)
- [ ] Create `src/utils/twitter/selectors.rs` with all selectors
- [ ] Integration: verify agent finds at least 1 tweet per cycle (log count)
- [ ] Unit test: mock page DOM → extraction returns expected metadata

**M3: Popup Handler**
- [ ] Create `src/task/twitter_popup.rs`
- [ ] Implement `close_all_modals()` iterating dismiss selectors
- [ ] Add ESC key fallback
- [ ] Inject in `run_cycle()` after navigation arrive
- [ ] Integration: run on x.com — observe any modals dismissed (log each)

---

### Phase 2 — Engagement Actions (Day 2)

**M4: Like Action**
- [ ] Implement `twitter_interact::like_tweet()`
- [ ] Locate `[data-testid="like"]` button
- [ ] Hover → click via `mouse::cursor_move_to` + `click_at`
- [ ] Verify state change (wait up to 2s for `[data-testid="unlike"]`)
- [ ] Add skip-if-already-liked logic
- [ ] Integration test: run 1 cycle → verify like appears on test account (manual check)

**M5: Retweet Action**
- [ ] Implement `twitter_interact::retweet_tweet()`
- [ ] Click retweet → modal → click confirm (native RT, no quote)
- [ ] Verify modal closes (best-effort)
- [ ] Integration: RT a known tweet, verify on account

**M6: Follow Action**
- [ ] Implement `twitter_interact::follow_user()`
- [ ] Find follow button inline or via author profile link
- [ ] Click, verify state → "Following" or "Pending"
- [ ] Skip if already following
- [ ] Integration: follow a test user, verify state change

**M7: Bookmark Stub (disabled V1)**
- [ ] Implement `bookmark_tweet()` returning `Err("disabled in V1")`
- [ ] Ensure config `max_bookmarks=0` blocks this action at limit guard

---

### Phase 3 — Agent Logic & Orchestration (Day 2–3)

**M8: TwitterAgent State Machine**
- [ ] Create `src/task/twitter_agent.rs`
- [ ] Implement struct: `TwitterAgent { page, profile, config, state, persona }`
- [ ] `run_session()`: loop cycles until max_cycles or duration budget
- [ ] `run_cycle()`: full flow (nav → modal close → dive decision → tweet find → click → simulate_reading → context load → sentiment → limit check → action)
- [ ] `select_entry_point()` delegates to navigation module
- [ ] `should_dive()`: `rand::random::<f64>() < profile.dive_probability * persona.multiplier()`
- [ ] `roll_engagement()`: pick action weighted by config probabilities (from available set)
- [ ] `can_engage()`: consult counters + limits
- [ ] `increment_counter()`: update `EngagementCounters`
- [ ] Phase tracking (warmup/active/cooldown) — used for V2 timing overrides

**M9: Limits & Counters**
- [ ] `EngagementCounters` struct with per-action u32 (likes, retweets, follows, replies, quotes, bookmarks)
- [ ] `can_engage()` checks each action's limit
- [ ] Log when limit hit (info level)

**M10: Sentiment Guard**
- [ ] Create `src/task/twitter_sentiment.rs`
- [ ] `contains_negative_sentiment()`: lowercase substring search against blocklist
- [ ] Config flag: `block_negative_engagement` (default false)
- [ ] If enabled: block action, log matched keyword, record skip

**M11: Persona Mapping**
- [ ] Create `src/task/twitter_persona.rs`
- [ ] `TwitterPersona::from_profile(profile)` heuristic mapping
- [ ] `input_method_weights()` — currently informational (V1 all mouse), V2 for keyboard/wheel mix
- [ ] `dive_probability_multiplier()` adjusts base profile.dive_probability

---

### Phase 4 — Config & Validation (Day 3)

**M12: Config Integration**
- [ ] Extend `Config` struct with `twitter: TwitterConfig`
- [ ] Add `default_twitter_config()` in `config.rs`
- [ ] Merge TOML + env overrides in `apply_env_overrides()`
- [ ] Add validation in `validate_config()`:
  - `max_cycles >= min_cycles`
  - `max_duration_sec >= min_duration_sec`
  - engagement probabilities sum ≤ 1.0 (warning only, not error)
  - entry point weights sum to ~100 (±5 tolerance)

**M13: Validation Schema**
- [ ] Extend `src/validation/task.rs` with `validate_twitteractivity()`
- [ ] V1: accept any JSON object (no required fields)
- [ ] Future: validate optional fields like `cycles=3` or `max_likes=2`
- [ ] Unit test: valid payload accepted, invalid rejected

---

### Phase 5 — Metrics & Logging (Day 3)

**M14: Task Metrics**
- [ ] `TwitterAgent` accumulates `TwitterMetrics`
- [ ] At task end, log final summary:
  ```
  [twitter] Session complete: cycles=8, likes=4, retweets=2, follows=1,
           skipped_dive=2, duration=723s, profile=Casual
  ```
- [ ] Ensure all log lines include `[twitter]` tag for log parsing

**M15: Run Summary Compatibility**
- [ ] Verify `perform_task()` returns `TaskResult::success()` if at least one engagement succeeded
- [ ] If all cycles skipped (no engagements), still return Success (not failure)
- [ ] `run-summary.json` from `MetricsCollector` captures top-level stats

---

### Phase 6 — Integration & Polish (Day 3)

**M16: End-to-End Dry Run**
- [ ] `cargo run twitterActivity` on machine with browser connected
- [ ] Observe log: entry point selection, tweet found, actions executed
- [ ] Verify no panics, no unwrap() crashes
- [ ] Check counters never exceed limits
- [ ] Monitor for modal storms — increase dismiss selector coverage if needed

**M17: Error Path Verification**
- [ ] Simulate network timeout (block x.com) → verify navigation timeout handling
- [ ] Simulate selector rot (rename a selector) → verify graceful skip (no panic)
- [ ] Simulate action failure (element detached) → verify warning logged

**M18: Config Edge Cases**
- [ ] Test `TWITTER_ENABLED=false` → task should exit early with info log
- [ ] Test `TWITTER_MAX_CYCLES=0` → zero cycles, task completes (edge, but valid)
- [ ] Test probabilities sum > 1.0 → warning log, but proceeds (over-limit is OK)

**M19: Documentation & Final Review**
- [ ] Update `README.md` with `twitterActivity` usage example
- [ ] Add sample `config/default.toml` Twitter section to README
- [ ] Document environment variable table
- [ ] Code review checklist: error handling, no unwraps, proper logging

---

### Phase 7 — Deferred (V2, Future)

**V2 Roadmap** (not in initial scope):
- [ ] LLM-powered replies & quote tweets (`twitter_llm.rs`)
- [ ] Sentiment analysis with NLP wordlists (VADER-style)
- [ ] Bookmark action implementation
- [ ] Reply action implementation (keyboard typing simulation)
- [ ] Dynamic entry point weights (per-session randomization ±10%)
- [ ] Advanced persona behaviors: hesitation micro-movements, overscroll, tab-switch simulation
- [ ] `run-summary.json` embedded per-task metadata
- [ ] Dashboard: real-time metrics UI (web sockets)

---

## 11. Success Criteria (Definition of Done)

V1 is complete when **all** of the following are verified:

**Functional Requirements**
- [ ] `cargo run twitterActivity` executes without panics or unwrap crashes
- [ ] Task completes 5–10 cycles (random within config range)
- [ ] Total session duration falls within 540–840 seconds (9–14min)
- [ ] At least one engagement (like/retweet/follow) occurs per session on average (configurable probability ≥ 0.1)
- [ ] Per-session limits respected: `likes ≤ max_likes`, `retweets ≤ max_retweets`, `follows ≤ max_follows` (hard stops)
- [ ] Sentiment guard (when enabled) blocks engagement on tweets containing blocklist keywords

**Reliability Requirements**
- [ ] Navigation failures (timeout, DNS error) do NOT crash the task — individual cycles are skipped, session continues
- [ ] Missing UI elements (e.g., already-liked tweet) are handled gracefully with info-level logs
- [ ] Modals are dismissed automatically; if a modal persists, task continues without crashing
- [ ] No evidence of `anyhow!` or `panic!` in logs across 10 consecutive runs

**Observability Requirements**
- [ ] Every log line includes `[twitter]` tag (searchable in log files)
- [ ] Per-cycle logs: entry URL, tweet found, action taken (or skip reason), duration
- [ ] Final summary line: `[twitter] Session complete: cycles=X, likes=Y, retweets=Z, follows=W, skipped=N, duration=Ts`
- [ ] `run-summary.json` exports with `task="twitterActivity"` and success status

**Configuration Requirements**
- [ ] All config fields in `[twitter]` section are loaded from `config/default.toml`
- [ ] All `TWITTER_*` environment variables override TOML values
- [ ] Validation passes at startup: weights sum ~100, probabilities ≤ 1.0, non-negative limits
- [ ] Task gracefully handles `TWITTER_ENABLED=false` (early exit with info log)

**Testing Requirements**
- [ ] Unit tests pass: `cargo test`
- [ ] Clippy warnings addressed: `cargo clippy --all-targets --all-features`
- [ ] Build succeeds: `cargo build --all-features`
- [ ] Manual integration: run task on live x.com (public), verify actions appear on test account (or at least logs show engagements attempted)
- [ ] Verify counters reset on subsequent runs (no cross-session state bleed)

**Code Quality Requirements**
- [ ] No `unwrap()` or `expect()` on `Result`/`Option` in production code (only in tests)
- [ ] All public functions have `#[allow(dead_code)]` removed (or justified)
- [ ] Module docs (`//!`) explain purpose
- [ ] Functions have inline comments for non-obvious logic
- [ ] Error types use `anyhow::Result` with context-rich messages

**Deployment Readiness**
- [ ] `README.md` updated with `twitterActivity` usage examples
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
- Update selectors in `selectors.rs` as Twitter UI evolves

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
| Cookie consent handling | Basic in `twitter_popup.rs` | May need expansion if new consent banners appear |

---

## 14. References

- Node.js Reference: `auto-ai/tasks/api-twitterActivity.js`
- Browser automation patterns: `task/cookiebot.rs`, `task/pageview.rs`
- Utilities: `src/utils/{navigation,scroll,mouse,timing,profile,blockmedia,page_size}.rs`
- Config system: `src/config.rs`
- Task orchestration: `src/orchestrator.rs`, `src/task/mod.rs`
- Metrics: `src/metrics.rs`
- Validation: `src/validation/task.rs`

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
| 11 | **Fallback Selector Order** | First-match cascade | Try article→cellInnerDiv→fallback→XPath |
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
