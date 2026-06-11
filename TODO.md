# Bug-Hunting Strategy for Rust Codebase

*Not "write more unit tests." The type system already kills most common bugs.
The best returns come from finding what the compiler can't see.*

---

## Layer 1: Types over Tests (highest ROI)

Encode invariants so invalid states are unrepresentable.

- [x] **Newtype for tweet IDs** — `String` everywhere means mixups are silent. `struct TweetId(String)` with `FromStr` validation. *(spec 0027 — migrated ~140 call sites across ~15 files)*
- [x] **Newtype for status URLs** — same problem, `/status/` parsing scattered across `dive.rs`. *(spec 0027 — `status_id_from_url()` now delegates to `StatusUrl::tweet_id()`)*
- [ ] **State machine for engagement flow** — `TweetOpened -> ComposerVisible -> TextEntered -> Posted`. Currently all fields are `Option<X>` and we `unwrap_or`.
- [ ] **`NonZeroU32` for counters** — `EngagementCounters` fields are `u32` but should never be zero for initialized state.
- [x] **`bool` → `enum` for action outcomes** — `like_tweet(api) -> Result<EngagementOutcome>` + `FollowOutcome` + `PostOutcome`. *(spec 0029 — 14 functions across 8 files)*

## Layer 2: Property-Based Testing (`proptest`)

Hand-written tests find the cases you think of. Proptest finds the cases you don't.

- [x] **Timing ranges** — `random_delay` bounds are tested with proptest already.
- [ ] **`select_persona_weights`** — property: given any valid JSON overrides, result weights stay in `[0, 1]`.
- [ ] **`modulate_persona_by_sentiment`** — property: sentiment -1..=1 maps to `interest_multiplier` in `[0, 1]`.
- [ ] **`remove_emojis`** — property: output length <= input length, no emoji codepoints remain, valid UTF-8 preserved.
- [ ] **`status_id_from_url`** — property: roundtrip: `format!("/user/status/{id}")` → parse → same `id`.
- [ ] **Engagement limit counters** — property: after N increments, `total_actions() == N`, no overflow panics.

## Layer 3: Fuzzing (`cargo fuzz`)

Best for parsing, deserialization, and any code that touches untrusted input.

- [ ] **LLM response parser** — the `LlmDecision` deserializer handles malformed JSON, truncated responses, unexpected fields. Fuzz it.
- [ ] **Spec file loader** — `spec.yaml` parsing accepts unknown keys, wrong types, missing fields. Ensure graceful error, not panic.
- [ ] **JS evaluation results** — `api.page().evaluate()` returns `serde_json::Value`. Downstream code assumes shape. Fuzz with unexpected shapes.

## Layer 4: Mutation Testing (`cargo mutants`)

Verifies tests actually catch bugs instead of just passing.

- [ ] **Install** — `cargo install cargo-mutants`
- [ ] **Baseline** — run on `twitteractivity_limits.rs` first (small, well-tested).
- [ ] **Threshold** — aim for < 10% surviving mutants on core logic modules.
- [ ] **Target** — decision strategies, sentiment analysis, limit enforcement.

## Layer 5: Coverage-Guided Gap Analysis (`cargo tarpaulin` / `cargo llvm-cov`)

Untested branches, not untested lines.

- [ ] **`handle_engagement_decision`** — which `match` arms are never exercised? (e.g., `tweet_age` branches, edge case decisions.)
- [ ] **`js_*` verification fallbacks** — every JS file has `.querySelector` chains with fallback to `null`/`document`. Are both paths tested?
- [ ] **Error propagation paths** — `anyhow::bail!`, `context()`, `unwrap_or_else` — which error paths are never triggered in tests?

## Layer 6: Dynamic Analysis (`cargo miri`)

Detects undefined behavior in unsafe code. Run weekly.

- [ ] **`miri` on test suite** — check for UB in dependency crates too.
- [ ] **Focus** — any `unsafe` block, `transmute`, raw pointer arithmetic, FFI boundaries.

## Layer 7: What NOT to Do

| Low-ROI activity | Why |
|---|---|
| More unit tests on covered lines | Find near-zero new bugs, cost stays flat |
| 100% line coverage | Rust compiler already guarantees no null/UB/data-race. Diminishing returns hit hard after ~70%. |
| Integration tests for browser automation | Slow, flaky, test the framework not your logic. Unit-test the decision layer, mock the browser. |
| Doc-test everything | Doc-tests are documentation with side effects. Prefer `#[cfg(test)] mod` for real tests. |

## Priority Order

1. **~~Newtypes~~** for tweet IDs, status URLs — ~~quick, mechanical, prevents entire bug class.~~ ✅ Done (spec 0027).
2. **Proptest** on `select_persona_weights`, `remove_emojis`, `status_id_from_url` — find edge cases now.
3. **Fuzz** LLM response parser — untrusted input, high impact.
4. **Mutants** baseline on limits module — quick confidence boost.
5. **Coverage gap** on decision strategies — find untested branches.
6. **State machine** for engagement flow — larger refactor, prevents logic bugs permanently.
