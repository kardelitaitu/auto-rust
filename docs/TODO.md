*last audited 16-06-26 by opencode*
# Bug-Hunting Strategy for Rust Codebase

*Not "write more unit tests." The type system already kills most common bugs.
The best returns come from finding what the compiler can't see.*

---

## Layer 1: Types over Tests (highest ROI)

Encode invariants so invalid states are unrepresentable.

- [x] **Newtype for tweet IDs** — `String` everywhere means mixups are silent. `struct TweetId(String)` with `FromStr` validation. *(spec 0027 — migrated ~140 call sites across ~15 files)*
- [x] **Newtype for status URLs** — same problem, `/status/` parsing scattered across `dive.rs`. *(spec 0027 — `status_id_from_url()` now delegates to `StatusUrl::tweet_id()`)*
- [x] **State machine for engagement flow** — `Idle -> ComposerOpen -> TextEntered -> Posted` encoded as `ReplyFlowState` enum + `ComposerFlow` struct with guarded transition methods. Used in 3 task files (twitterreply, twitterretweet, twitterquote).
- [x] **`NonZeroU32` for counters** — `EngagementCounters` fields are `u32` but should never be zero for initialized state.
- [x] **`bool` → `enum` for action outcomes** — `like_tweet(api) -> Result<EngagementOutcome>` + `FollowOutcome` + `PostOutcome`. *(spec 0029 — 14 functions across 8 files)*

## Layer 2: Property-Based Testing (`proptest`)

Hand-written tests find the cases you think of. Proptest finds the cases you don't.

- [x] **Timing ranges** — `random_delay` bounds are tested with proptest already.
- [x] **`select_persona_weights`** — property: given any valid JSON overrides, result weights stay in `[0, 1]`. *(proptest added)*
- [x] **`modulate_persona_by_sentiment`** — property: sentiment -1..=1 maps to `interest_multiplier` in `[0, 1]`. *(proptest added — via `with_sentiment_modulation`)*
- [x] **`remove_emojis`** — property: output length <= input length, no emoji codepoints remain, valid UTF-8 preserved. *(3 proptests added)*
- [x] **`status_id_from_url`** — property: roundtrip: `format!("/user/status/{id}")` → parse → same `id`. *(3 proptests added)*
- [x] **Engagement limit counters** — 5 proptests: total_actions==N after N increments, cache sync (total_actions==computed_total), no overflow on large counts, increment() dispatch for all 7 action types, to_summary() consistency.

## Layer 3: Fuzzing (`cargo fuzz`)

Best for parsing, deserialization, and any code that touches untrusted input.

- [x] **LLM response parser** — proptest fuzzing for `LlmDecision` deserializer (5 tests) + `LlmSentimentResult` deserializer (5 tests) + JSON extraction logic. **Found and fixed a real bug** in `analyze_sentiment_llm`: potential panic when `{` appears after `}` in LLM response + off-by-one that dropped closing `}`.
- [x] **Spec file loader** — 12 fuzz proptests: parse_task_yaml, parse_task_toml, TaskDefinition/Action/Condition/SpecMeta YAML deserialize, validate_task_definition, format_task_definition, unknown keys, extra fields, boundary tests. All with any::<String>() — no panics.
- [x] **JS evaluation results** — `api.page().evaluate()` returns `serde_json::Value`. Downstream code assumes shape. Fuzz with unexpected shapes.

## Layer 4: Mutation Testing (`cargo mutants`)

Verifies tests actually catch bugs instead of just passing.

- [x] **Install** — `cargo install cargo-mutants` (v27.1.0 installed)
- [x] **Script** — `.\mutants.ps1` created (7 targets: limits, decision-engine, decision-strategies, persona, llm, errors, duration)
- [ ] **Baseline** — run `.\mutants.ps1 -Target limits` on Windows native machine.
- [ ] **Threshold** — aim for < 10% surviving mutants on core logic modules.
- [ ] **Target** — run `.\mutants.ps1` for decision strategies, sentiment analysis, limit enforcement.

## Layer 5: Coverage-Guided Gap Analysis (`cargo tarpaulin`)

Untested branches, not untested lines.

- [x] **Script** — `.\coverage.ps1` created (6 focus modules: decision engine, JS verification, error paths, engagement state, persona weights, limits enforcement)
- [ ] **`handle_engagement_decision`** — which `match` arms are never exercised? Run `.\coverage.ps1 -Target decision`
- [ ] **`js_*` verification fallbacks** — every JS file has `.querySelector` chains with fallback to `null`/`document`. Run `.\coverage.ps1 -Target js`
- [ ] **Error propagation paths** — `anyhow::bail!`, `context()`, `unwrap_or_else` — which error paths are never triggered? Run `.\coverage.ps1 -Target errors`

## Layer 6: Dynamic Analysis (`cargo miri`)

Detects undefined behavior in unsafe code. Run weekly.

- [x] **Script** — `.\miri.ps1` created (installs nightly + miri, focuses on session/duration.rs unsafe blocks)
- [ ] **`miri` on test suite** — run `.\miri.ps1` to check for UB. Only 2 unsafe blocks found (both in `src/session/duration.rs` — `NonZeroU64::new_unchecked`).
- [ ] **Focus** — `unsafe` blocks in `session/duration.rs` lines 38 and 84.

## Layer 7: What NOT to Do

| Low-ROI activity | Why |
|---|---|
| More unit tests on covered lines | Find near-zero new bugs, cost stays flat |
| 100% line coverage | Rust compiler already guarantees no null/UB/data-race. Diminishing returns hit hard after ~70%. |
| Integration tests for browser automation | Slow, flaky, test the framework not your logic. Unit-test the decision layer, mock the browser. |
| Doc-test everything | Doc-tests are documentation with side effects. Prefer `#[cfg(test)] mod` for real test coverage. |

## Priority Order

1. **~~Newtypes~~** for tweet IDs, status URLs — ~~quick, mechanical, prevents entire bug class.~~ Done (spec 0027).
2. **~~Proptest~~** on `select_persona_weights`, `remove_emojis`, `status_id_from_url` — ~~find edge cases now.~~ Done (Layers 1-3 complete).
3. **~~Fuzz~~** LLM response parser — ~~untrusted input, high impact.~~ Done (found + fixed real bug).
4. **Mutants** baseline on limits module — quick confidence boost. **Ready: `.\mutants.ps1 -Target limits`**
5. **Coverage gap** on decision strategies — find untested branches. **Ready: `.\coverage.ps1 -Target decision`**
6. **Miri** UB detection — safety net for unsafe code. **Ready: `.\miri.ps1`**
