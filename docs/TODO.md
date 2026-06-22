*last audited 23-06-26 by Buffy*
# Bug-Hunting Strategy for Rust Codebase

*Not "write more unit tests." The type system already kills most common bugs.
The best returns come from finding what the compiler can't see.*

---

## Layer 1-3: Type Safety, Property Testing & Fuzzing (Completed)
- **Newtypes & State Machines** — `TweetId`, `StatusUrl`, `ReplyFlowState`, `EngagementOutcome`, `FollowOutcome`, `PostOutcome` migrated.
- **Property-Based Testing** — Proptests active for timing ranges, persona weights, sentiment modulation, emoji removal, and engagement limits.
- **Fuzzing** — LLM parser and spec file loader fuzz targets verified (found + fixed real LLM parsing bug).

## Layer 4: Mutation Testing (`cargo mutants`)

Verifies tests actually catch bugs instead of just passing.

- [x] **Install & Setup** — `cargo install cargo-mutants` (v27.1.0) and `.\mutants.ps1` wrapper script created.
- [x] **Script encoding fix applied** — Replaced em dash characters (`—`, U+2014) with ASCII hyphens on 2 lines that broke PowerShell parsing.
- [x] **Baseline attempted** — Run `.\mutants.ps1 -Target limits`. **Still blocked by Windows `nul` copy bug** in `cargo-mutants` v27.1.0 (known issue: `Failed to copy ...\nul to ...\tmp\nul`). 166 mutants found but `outcomes.json` not written due to copy failure. Requires running on Linux/WSL or waiting for cargo-mutants fix.
- [ ] **Threshold** — Aim for < 10% surviving mutants on core logic modules.
- [ ] **Target** — Run `.\mutants.ps1` for decision strategies, sentiment analysis, limit enforcement.

## Layer 5: Coverage-Guided Gap Analysis (`cargo tarpaulin`)

Untested branches, not untested lines.

- [x] **Script** — `.\coverage.ps1` created (focuses on decision engine, JS verification, error paths, engagement state, etc.).
- [x] **coverage script fixed** — `--test-threads`, `$testFilter`, `$testThreads`, JSON array parsing, UTF-8 encoding, try/catch all working.
- [x] **Full project coverage** — **40.95%** (7,849/19,167 lines), 3,819 tests passed.
- [x] **P1: `extract_tweet_text` consolidated** — Enhanced with retweet recursion + truncated JSON fallback. Duplicate removed from `sentiment/helpers.rs` (replaced with `pub(crate) use` re-export).
- [x] **P2: Coordinate parsing centralized** — 6 inline sites migrated to `parse_button_coordinates` across `popup.rs`, `navigation.rs`, `llm_execute.rs`.
- [x] **P3: Pure functions moved to `state/types.rs`** — `compute_trending_bias`, `detect_conversation_indicators` moved + 27 new tests.
- [x] **Decision engine coverage verified** — 276/276 tests pass, all modules at 91-100% coverage:
  - `engine.rs`: 34/34 (100%)
  - `strategies/hybrid.rs`: 66/66 (100%)
  - `strategies/legacy.rs`: 105/106 (99%)
  - `strategies/persona.rs`: 54/54 (100%)
  - `strategies/unified.rs`: 129/141 (91%)
  - `types.rs`: 18/18 (100%)
  - `strategies/llm.rs`: 47/64 (73%) — LLM-dependent paths
- [x] **predictive_scorer.rs scaffold cleanup** — Removed 4 unused ML scaffold structs (-110 lines), precision-refined 10 dead_code annotations to field-level
- [x] **twitteractivity_actions.rs test expansion** — +7 extract_tweet_text edge case tests, +4 generate_quote_text tests (now matches generate_reply_text coverage), removed 8 duplicate tests across engagement/tests.rs and sentiment/core.rs
- [ ] **Remaining low-coverage areas** — All are browser-dependent async orchestration:
  - `engagement/dispatch.rs` (2.3%) and `engagement/mod.rs` (0%) — pure async orchestration
  - `twitteractivity_llm.rs` (0%) — LLM + browser dependent
  - `dom.rs` (0%, 378 lines) — feature-gated browser interaction

## Layer 6: Dynamic Analysis & Lint Hardening (Completed)
- **Miri Analysis** — `.\scripts\miri.ps1` verified 18/18 tests, no UB detected in unsafe duration blocks.
- **Lint Hardening** — Denied `unwrap_used`, `expect_used`, and unsafe code in pipeline.
- **`serde_yml` → `serde_yaml`** — Migrated 129 occurrences across 18 files.
- **Dead code audit** — 88 `#[allow(dead_code)]` annotations analyzed, 3 unused mocks removed.

---

## Layer 7: Twitter/X Activity Orchestrator Weaknesses (Resolved)

- [x] **Loop Scheduler Starvation** — Fixed scroll check ordering.
- [x] **Async Cancellation Safety** — `ThreadDiveGuard` drop guard.
- [x] **Dynamic Pause Scaling** — Profile-derived ranges replacing hardcoded values.
- [x] **LLM Context Fallback** — Valid context checks before LLM calls.

---

## Priority Order

1. **✅ P1: `extract_tweet_text` consolidated** — 1 canonical implementation, retweet recursion + JSON fallback.
2. **✅ P2: Coordinate parsing centralized** — 6 inline sites → `parse_button_coordinates`.
3. **✅ P3: Pure functions moved + tested** — `compute_trending_bias`, `detect_conversation_indicators` + 27 tests.
4. **✅ Decision engine coverage** — 276 tests, 91-100% across all modules.
5. **✅ Mutants.ps1 encoding fixed** — Em dash characters replaced, but **Windows `nul` copy bug persists** (needs WSL/Linux).
6. **✅ Bacon-pipeline investigated** — 22 source files, inline tests exist (25 `#[cfg(test)]` markers), 0% tarpaulin coverage (not measured from main crate).
7. **✅ Utils module coverage verified** — math.rs (43), geometry.rs (10), url.rs (9), retry.rs (7) — all well tested
8. **⏸️ Full check.ps1 passes** — 8/8 steps, 3,819 tests
9. **⏸️ `check.ps1` + `check-fast.ps1`** — Both clean, ready for commit.

## What NOT to Do

| Low-ROI activity | Why |
|---|---|
| More twitter pure-function extraction | Exhausted — remaining gaps are browser-dependent |
| `dom.rs` tests | Feature-gated (`accessibility-locator`), browser-only |
| `bacon-pipeline` coverage | Separate crate, 22 files, tests exist but not measured from main crate |
| `mutants.ps1` on Windows | `cargo-mutants` v27.1.0 `nul` copy bug — blocked |
