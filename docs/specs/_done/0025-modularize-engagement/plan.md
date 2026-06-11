# Plan — Modularize twitteractivity_engagement.rs

## Baseline

`src/utils/twitter/twitteractivity_engagement.rs` is 1,474 lines — the 2nd largest remaining monolith in the twitteractivity subsystem. It contains four logical concerns:

| Concern | Lines | Functions |
|---------|-------|-----------|
| **Scoring** | 59-191 | `handle_engagement_decision()`, `modulate_persona_by_sentiment()` |
| **Orchestration** | 193-571 | `engage_replies()`, `process_candidate()` (298 lines) |
| **Action dispatch** | 385-540 | Inline match block inside `process_candidate()` for 6 actions |
| **Tests** | 577-1093 | 5 test modules: integration, decision, statistical, property, gap |

The action dispatch match block (~155 lines) is the largest self-contained extraction target within `process_candidate()`. It handles like (with position/selector fallback), retweet, quote (with LLM/template fallback), follow, reply (with LLM/template fallback), and bookmark — each with retry wrapping, counter updates, success/failure metrics, and humanized pauses.

## Implementation Steps

1. Create `src/utils/twitter/engagement/` directory with 4 files:
   - `mod.rs` — `process_candidate()`, `engage_replies()`, `SENTIMENT_ANALYZER` static, public re-exports
   - `scoring.rs` — `handle_engagement_decision()`, `modulate_persona_by_sentiment()`
   - `dispatch.rs` — `dispatch_action()` extracted from the `process_candidate()` match block (lines 385-540)
   - `tests.rs` — all 5 `#[cfg(test)]` modules from lines 577-1093

2. In `dispatch.rs`, define `dispatch_action()`:
   ```rust
   pub async fn dispatch_action(
       api: &TaskContext,
       action: &str,
       tweet: &Value,
       tweet_id: &str,
       did_dive: bool,
       sentiment: Sentiment,
       task_config: &TaskConfig,
       counters: &mut EngagementCounters,
       action_tracker: &mut TweetActionTracker,
       actions_this_scan: &mut u32,
   ) -> Result<bool>
   ```
   Returns `true` on success, `false` on failure. All counter updates, metrics, and pauses happen inside.

3. In `mod.rs`, replace the inline match block with:
   ```rust
   let success = dispatch::dispatch_action(
       api, action, tweet, tweet_id, did_dive,
       sentiment, task_config, counters, action_tracker, actions_this_scan,
   ).await?;
   ```

4. Update imports: `dispatch.rs` needs its own independent import block (the metrics constants, twitteractivity_interact, twitteractivity_llm, etc.).

5. Delete `src/utils/twitter/twitteractivity_engagement.rs`.

6. Update callers — search for `twitteractivity_engagement` imports and update to `engagement`:
   - `src/task/twitteractivity.rs` — imports `process_candidate`
   - `src/utils/twitter/twitteractivity_state.rs` — may reference types
   - Any other files importing from this module

## API Changes

- Module path changes from `crate::utils::twitter::twitteractivity_engagement` to `crate::utils::twitter::engagement`
- Public API surface preserved: `process_candidate`, `handle_engagement_decision`, `engage_replies`, test re-exports (`should_follow`, `should_like`, `should_reply`, `should_retweet`, `SentimentTemplates`, `TweetActionTracker`)
- `dispatch_action()` is a new internal API but not re-exported at the `utils::twitter` level

## Validation

- `cargo check --lib` — no compilation errors
- `cargo test --lib twitteractivity_engagement` — all existing tests pass
- `powershell -File check.ps1` — full quality gate passes

## Design Decisions and Risks

**Why extract dispatch and scoring separately?** Scoring (`handle_engagement_decision`, `modulate_persona_by_sentiment`) is self-contained decision logic with no side effects beyond the SentimentAnalyzer static. Dispatch is pure I/O with heavy retry/counter instrumentation. Separating them makes each file testable and reviewable independently.

**Why keep `process_candidate()` in mod.rs?** The orchestration function is the integration point between scoring, dispatch, dive, and replies. Extracting it further would require restructuring the control flow — which risks behavioral changes. The spec explicitly avoids refactoring the flow.

**Risk: dispatch_action() parameter count.** The function needs ~12 parameters (api, action type, tweet data, state). This is intentionally verbose to avoid hidden coupling. Alternative: bundle into a `DispatchContext` struct — deferred as a possible follow-up.

**Confidence: High.** Seven prior modularization specs (0016-0024) have followed this exact pattern on files of similar or larger size. All passed check.ps1 and spec-lint on first verification.
