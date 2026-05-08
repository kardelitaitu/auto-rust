# TODO
> **Priority Order:** P1 Critical → P2 Important → P3 Lower → Spec-Ready Backlog → Coverage/Performance → Future Ideas
> **Quality Gate:** No task may be marked complete until `./check.ps1` passes (test suite verification)
> **Confidence Levels:** (95%) = Verified in codebase, clear scope | (80%) = Mostly verified, minor ambiguity | (60%) = Partially verified, need exploration

---

## AI Agent Task Execution Protocol
> **Rule:** Follow this protocol for every task marked `[ ]` in this file. Deviations require explicit user approval.

### Step 1: READ TASK
**What:** Read the full task description without assuming the current approach is correct.
- Identify: **what** needs to change, **where**, and **why**
- Extract: specific files, functions, and acceptance criteria
- Do NOT assume the current approach is correct
**Checkpoint:** "I understand this task requires [what] in [where] to achieve [why]."

### Step 2: CHECK CURRENT CODEBASE
**What:** Verify baseline state and gather context before planning.
- Run `cargo check` to verify clean compilation
- Run `git status` to confirm no uncommitted changes
- Search for relevant code patterns using grep/search
- Read related files to understand context
**Checkpoint:** "Baseline is clean. Relevant code exists in [files]. No interference detected."

### Step 3: ANALYZE
**What:** Deep-trace the affected code paths to understand behavior.
- Trace data flow through affected code
- Identify: function signatures, return types, error handling patterns
- Check for existing tests covering the affected paths
- Review documentation if available
**Checkpoint:** "The code currently works by [data flow]. Tests cover [paths]. Edge cases are [known/unknown]."

### Step 4: BREAK DOWN TO SMALLER STEPS
**What:** Split into atomic, reversible changes ordered by risk.
- Split into 3-7 atomic, reversible changes
- Order: safe changes first, risky changes last
- Identify test coverage gaps
- Plan rollback/verify strategy
**Checkpoint:** "Changes ordered: [1. safe], [2. medium], [3. risky]. Rollback via [method]."

### Step 5: CONFIRM STRATEGY WITH USER
**What:** Present problem, proposed fix, and rationale before irreversible changes.
- Present: problem, proposed fix, rationale
- Show: relevant code snippets, test results, analysis
- Ask: explicit approval before irreversible changes
- Adjust based on user feedback
**Anti-Patterns to Avoid:**
- Never say "I'll just fix it quickly" without analysis
- Never skip analysis by jumping to conclusions
- Never assume user wants the obvious solution
- Never make breaking changes without rollback plan
**Trigger Questions:**
- "Should I proceed with this approach?"
- "Is this change acceptable, or should I adjust?"
- "Should I create a rollback checkpoint first?"
**Checkpoint:** User approved "[approach]". Adjustments: [none/changes].

### Step 6: EXECUTE
**What:** Implement changes following existing code style with verification.
- Make minimal changes following existing style
- Add/update tests for the fixed code
- Run `./check.ps1` to verify (NOT just `cargo test`)
- Report: what changed, why, and verification results
**Checkpoint:** "Implementation complete. `./check.ps1` passed. Changes: [summary]."

### Quality Gates
- **Never** commit without running `./check.ps1`
- **Never** skip user confirmation for breaking changes
- **Always** provide rollback strategy for large changes
- **Always** run verification commands listed in task subtasks

### Common Anti-Patterns (AI Agents)
| Anti-Pattern | Why Bad | Correct Approach |
|-------------|--------|------------------|
| Skip analysis and fix immediately | May miss root cause, introduce bugs | Analyze first, confirm strategy, then fix |
| Assume obvious solution is correct | User may have different constraints | Ask "Should I approach this as [X]?" |
| Make changes without running checks | May break CI, introduce regressions | Run `./check.ps1` before marking done |
| Skip user confirmation | User may reject the approach | Present strategy, wait for approval |
| Add comments without understanding | May be wrong or misleading | Only add comments if you fully understand |
| Rename without checking usage | May break external APIs | Search all usages before renaming |

---

## P1: Critical (High Impact, Low Effort) ✅ COMPLETE
- [x] **unwrap() Reduction** - DONE (2026-05) - Production: ~15 unwraps (<20 target) ✅
- [x] **Split Large Files** - DONE (2026-05) - task_context.rs → 4 modules, mouse.rs → 3 modules ✅
- [x] **Fix Remaining Warnings** - DONE (0 warnings), cargo-nextest CI ✅

## P2: Important (Medium Term) ✅ COMPLETE
- [x] **Dependency Audit** - DONE (2026-05) - Removed 3 deps, 37 direct deps ✅
- [x] **Add Benchmark Suite** - DONE (2026-05) - `criterion`, 3 benches ✅
- [x] **Increase Bus Factor** - DONE (2026-05) - ARCHITECTURE.md, 8 ADRs ✅
- [x] **Config Loading Normalization** - DONE (2026-05) - Removed 2 dead fields ✅
- [x] **Click-Learning Persistence** - DONE (2026-05) - `LearningEngine`, 12 tests ✅

## P3: Lower Priority ✅ COMPLETE (Verified 2026-05-08)
- [x] **Session execution guard + deterministic shutdown tests** - DONE ✅
- [x] **TaskContext click / interaction pipeline** - DONE ✅
- [x] **Runtime shutdown + group execution coordination** - DONE ✅
- [x] **CLI task parsing + validation + registry** - DONE ✅
- [x] **Browser discovery / session assembly** - DONE (2026-05-08) **(Confidence: 100%)**
  - [x] `src/session/connector.rs` - `LocalBrowserConnector` with Brave (9001-9050) + Chrome (9222-9230) ports
  - [x] `src/session/factory.rs` - `SessionFactory` + `SessionFactoryBuilder` with timeout/clamping
  - [x] `src/session/pool.rs` - `SessionPoolManager` with connector counting + capability matching
  - [x] `src/browser.rs` - Refactored to use `SessionPoolManager`
  - **Verification:** Read all 3 files, confirmed implementations match TODO description

---

## Test Coverage Improvement Program (NEW PRIORITY)

> **Goal:** Target modules with <50% coverage, focus on units that can be tested without browser dependencies.
> **Prerequisite:** Run `cargo tarpaulin --all-features` to get accurate baseline numbers.

### High Priority (0-20% Coverage) ✅ COMPLETE
- [x] **src/main.rs** - 8 tests added (session health, warning format, working dir) ✅
- [x] **src/session/mod.rs** - 20+ tests added (circuit breaker, state machine) ✅

### Medium Priority (20-40% Coverage) ✅ COMPLETE
- [x] **src/utils/mouse/trajectory.rs** - 25 tests added ✅
- [x] **src/task/*.rs** - Coverage improved ✅

### Spec-Ready Backlog

> Only keep items here if the code shape is already verified and the next step is implementation, not discovery.

| Item | Confidence | Why it is spec-ready |
|---|---|---|
| `src/task/twitteractivity.rs` | 95% | Thin orchestrator; helper logic already moved to `src/utils/twitter/*` and current tests exist. |
| `src/adaptive/predictive_scorer.rs` | 95% | File structure and test surface are stable; remaining work is edge-case coverage and bounds checks. |
| `src/browser.rs` | 95% | Helper behavior and port config coverage already exist; remaining work is regression coverage, not refactor. |
| `src/orchestrator.rs` | 95% | Cancellation, guard, timeout, and aggregation paths are already implemented and testable. |

- [ ] **src/task/twitteractivity.rs**
  - Add regression tests for `run()` / `run_inner()` payload normalization and error propagation.
  - Add a wiring test for `select_entry_point()` and task config defaults.
  - Add a summary/logging regression for `log_summary()`.
  - Recheck coverage before keeping this in any <50% bucket.

- [ ] **src/adaptive/predictive_scorer.rs**
  - Rebaseline with `cargo tarpaulin` or `cargo llvm-cov`.
  - Add edge-case tests for zero, max, and clamped scoring inputs.
  - Add property tests for score bounds and monotonicity.
  - If persistence is still required, move that work to `learning_engine.rs`.

- [ ] **src/browser.rs**
  - Add integration coverage for filtered discovery and profile/session fallback ordering.
  - Add regression coverage for empty-config + filter inputs.
  - Keep the pure helper tests as the baseline.

- [ ] **src/orchestrator.rs**
  - Add a backoff-cancellation regression for `execute_task_with_retry()`.
  - Add one timeout/unhealthy-session regression only if the baseline still shows a gap.
  - Rebaseline before adding more cases.

### Low Priority (Already Well Covered) ✅ COMPLETE
- [x] **src/utils/scroll.rs** - 40+ tests ✅
- [x] **src/utils/zoom.rs** - 18+ tests ✅
- [x] **src/utils/keyboard.rs** - 80+ tests ✅
- [x] **src/utils/navigation.rs** - 43+ tests ✅
- [x] **src/runtime/task_context.rs** - Well covered via browser tests ✅
- [x] **src/utils/accessibility_locator.rs** - 95%+ coverage ✅

### Coverage Measurement Improvements
- [x] Add coverage gate to CI (fail if < 40% on new code) **(Confidence: 90%)** - DONE (2026-05-08) in `.github/workflows/ci.yml` via `cargo llvm-cov --workspace --all-features --no-report --fail-under-lines 40`
- [x] Consider `cargo-llvm-cov` for integration test coverage **(Confidence: 70%)** - DONE (2026-05-08) in `.github/workflows/ci.yml`
- [x] Track coverage trends over time **(Confidence: 85%)** - DONE (2026-05-08) by publishing `coverage.json` from CI

### Target Outcomes
| Metric | Current | Target |
|--------|---------|--------|
| True unit test coverage | ~40% | **65%** |
| twitteractivity.rs | ~45% | **60%** |
| Entry point tests | 0% | 80% |
| Utility module tests | 20% | 80% |
| Session management tests | 0% | 50% |

---

## Performance Work (NEW PRIORITY)

> **Goal:** Profile slow tests, optimize cargo-nextest runs, reduce CI execution time.
> **Current state:** `.config/nextest.toml` only has `[profile.ci] fail-fast = true` (verified 2026-05-08)

### Profile Slow Tests **(Confidence: 85%)**
- [x] **Profile slow tests with a temporary low `slow-timeout`** - (95% confidence) - DONE (2026-05-08); 500ms and 100ms thresholds produced no `SLOW` output in the current lib suite
  - Command: set a temporary `slow-timeout` override, then run `cargo nextest run --all-features --profile ci --status-level slow --final-status-level slow`
  - Identify the top 20 slowest tests from `SLOW` output
  - Analyze why they're slow (sleep, retry, timeout)
  - Create optimization plan
  - **Effort:** 0.5 day

- [ ] **Profile test categories:** - (80% confidence)
  - [ ] API client integration tests (currently ~0.1-0.2s each after optimization)
  - [ ] Health logger tests (currently ~1.1s each after reduction)
  - [ ] Gaussian math tests (currently ~0.03s each after optimization)
  - [ ] Navigation integration tests (10 tests, browser-dependent)
  - [ ] Accessibility locator tests (25+ tests, mixed unit/integration)
  - **Effort:** 1 day

### Optimize cargo-nextest Runs **(Confidence: 90%)**
- [ ] **Reduce test parallelism where it causes conflicts:** - (95% confidence)
  - Browser-dependent tests: `--test-threads=1` for integration suites
  - Unit tests: `--test-threads=8` (or auto-detect)
  - Mixed suites: Split into separate test targets
  - **Effort:** 0.5 day

- [ ] **Add .config/nextest.toml optimizations:** - (90% confidence)
  - [x] Add output format: `[profile.ci] failure-output = "immediate-final"`
  - [x] Retry failed tests once: `[profile.ci] retries = 1`
  - [x] Profile-based timeouts: `[profile.ci] slow-timeout = { period = "60s", terminate-after = 5, grace-period = "30s" }`
  - **Current config:** `[profile.ci] fail-fast = true`, `retries = 1`, `failure-output = "immediate-final"`, `slow-timeout = { period = "60s", terminate-after = 5, grace-period = "30s" }`
  - **Effort:** 0.5 day

- [ ] **CI cache optimization:** - (75% confidence)
  - [ ] Cache cargo registry + git deps
  - [ ] Cache target/ directory between runs
  - [ ] Use sccache for compilation caching
  - **Effort:** 1 day (requires CI workflow changes)

### Benchmark and Track **(Confidence: 80%)**
- [ ] **Add performance benchmarks to CI:** - (85% confidence)
  - [ ] `cargo bench` as separate CI job (not blocking)
  - [ ] Track test execution time trends
  - [ ] Alert if CI run exceeds 10 minutes
  - **Effort:** 1 day

- [ ] **Local performance tooling:** - (90% confidence)
  - [x] `cargo nextest list --verbose` for test inventory
  - [x] `cargo nextest run --profile ci --status-level slow` for slow-test review
  - [x] Custom script to generate slow-test report (`performance.ps1`)
  - **Effort:** 0.5 day

### Target Outcomes
| Metric | Current | Target |
|--------|---------|--------|
| CI full run | ~5-7 min | **<10 min** (with alerts) |
| Slowest test | 6.2s | **<2s** (or parallelized) |
| cargo-nextest speedup | 2-10x | **Maintain + tune** |
| Test flakiness | Occasional | **0 flaky in 10 runs** |

---

## Future Ideas (Parking Lot)

> **Goal:** Track ideas that are not spec-ready yet.
> **Rule:** Do not convert these into specs until they move out of discovery mode.
> **Current state (verified 2026-05-08):** Partially implemented - `twitteractivity_engagement.rs` has `llm_enabled`, `smart_decision_enabled`, `enhanced_sentiment_enabled`, `dry_run_actions` fields AND implementation code. `src/utils/twitter/twitteractivity_llm.rs` and `twitteractivity_sentiment_llm.rs` EXIST.

### Parking Lot

> Keep these out of the spec seed until there is a concrete implementation plan or new evidence.

- [x] **Bookmark action implementation** - already implemented in `src/utils/twitter/twitteractivity_interact.rs`; rollout remains config-gated by default limits/probabilities.
- [x] **Reply action implementation** - already implemented in `src/utils/twitter/twitteractivity_interact.rs`; natural typing and LLM prompt polish can stay as follow-up work if needed.

- [ ] **LLM-powered replies & quote tweets** (`twitteractivity_llm.rs`) - (80% confidence)
  - Verify prompt contract, output sanitization, and failure behavior.
  - Add mock-provider tests for fallback and timeout paths.
  - Add config validation for `reply_with_ai` and `quote_with_ai`.
  - Document prompt rules in `docs/TASKS/twitteractivity.md`.

- [ ] **Sentiment analysis with NLP** (`twitteractivity_sentiment_enhanced.rs` + `twitteractivity_sentiment_llm.rs`) - (70% confidence)
  - Compare keyword-only, enhanced, and hybrid sentiment on a shared corpus.
  - Add tests for thread context, reputation, and confidence gating.
  - Decide whether a lightweight external model is worth the added complexity.

- [ ] **Dynamic entry point weights** (per-session randomization ±10%) - (90% confidence)
  - Add jitter around the existing category weights.
  - Keep totals normalized after jitter.
  - Add a config flag for enabling jitter.
  - Add distribution tests for the randomized weights.

- [ ] **Advanced persona behaviors** - (65% confidence)
  - Implement hesitation micro-movements in `twitteractivity_humanized.rs`.
  - Add overscroll simulation after engagement.
  - Add tab-switch simulation only if the browser/session model can support it cleanly.
  - Wire micro-movements into `TwitterPersona` multipliers.

- [ ] **`run-summary.json` embedded per-task metadata** - (85% confidence)
  - Extend the task result/metrics shape with an optional metadata payload.
  - Populate the Twitter breakdown in `MetricsCollector::task_completed_from_result`.
  - Update summary serialization and add JSON shape tests.
  - Keep the metadata small and optional so non-Twitter tasks stay cheap.

- [ ] **Thread engagement** (click "Show more replies") - (70% confidence)
  - Implement "Show more replies" click logic.
  - Add DOM traversal for reply-thread expansion.
  - Add tests for the thread-engagement path.

### Prerequisites Before Spec Drafts **(Confidence: 95%)**
- [ ] Rebaseline the spec-ready backlog before using file-specific targets.
- [ ] Performance: CI runs <10 min (currently ~5-7 min, acceptable).
- [ ] Documentation: V2 spec packages exist in `docs/specs/_active/` for any approved feature.
- [ ] Config: Validate any new schema extensions before drafting spec text.
- [ ] Integration: Test with real LLM only if reply/quote work is moved out of the parking lot.

---

## Accessibility Locator Test Coverage Program ✅ COMPLETE

### Coverage Targets (Gate to Expand Rollout)
- [x] `src/utils/accessibility_locator.rs` line coverage >= 95% - DONE ✅
- [x] `src/utils/navigation.rs` line coverage >= 90% - DONE ✅
- [x] `src/runtime/task_context.rs` line coverage >= 85% - DONE ✅
- [x] `src/task/twitterfollow.rs` line coverage >= 90% - DONE ✅
- [x] zero flaky failures across 5 consecutive feature-on CI runs - DONE ✅
- [x] Brave and Chrome browser ports configurable via environment variables - DONE ✅

### Exit Criteria (Definition of High Coverage)
- [x] Phases 1-6 complete (comprehensive test coverage achieved) ✅
- [x] Coverage targets met ✅
  - `src/utils/accessibility_locator.rs` >= 95% ✓
  - `src/utils/navigation.rs` >= 90% ✓
  - `src/runtime/task_context.rs` >= 85% ✓
  - `src/task/twitterfollow.rs` >= 90% ✓
- [x] Rollout monitoring period passes without regression spike ✅
- [ ] No flaky locator tests in 5 consecutive CI runs (monitoring)
- [x] Rollback trigger/action documented before default-on decision ✅

**Phase 7 deferred** - Current `--all-features` CI coverage sufficient.

---

## Recently Completed (2026-05-08)

### Documentation Audit ✅ COMPLETE
- [x] **Audit 48 documentation files** - DONE (2026-05-08)
  - Added `last audited 08-05-26 by Kilo` stamp to all docs
  - Fixed drift: 8→12 permissions, 13→15 task count, v0.0.3→v0.1.0 refs
  - Fixed config path `data/config/`→`config/`, env vars `AUTO_*`→`ROXYBROWSER_API_*`
  - Fixed `twitteractivity_engagement.rs`→`twitteractivity.rs`, API signatures
  - 5 twitterActivity archive files stamped (02-config, 03-agent, 04-modules, 05-metrics, 06-implementation)
  - Commit: `170b2b1` pushed to `origin/main`

### TwitterActivity Review + Specs ✅ COMPLETE
- [x] **Specs 0031-0033** - DONE (2026-05-08)
  - 0031: Scroll timing fixes (content load delay, initial scroll timing)
  - 0032: Scroll error handling (consecutive failure tracking)
  - 0033: Empty scan early exit (consecutive empty scan tracking)
  - All moved to `_done/` with status=`done`

### Cargo-Nextest Migration ✅ COMPLETE
- [x] **Migrated to cargo-nextest** - DONE (2026-04-30)
  - Created `.config/nextest.toml` with `[profile.ci] fail-fast = true`
  - Updated CI workflow to use cargo nextest
  - Verified test parity (2110+ passed)
  - Optimized 14 slow tests (timing, math, health logger, API client)

---

## Archive: Completed Tasks (2026-04 to 2026-05)

<details>
<summary>Click to expand: P1, P2, P3, and other completed tasks</summary>

### P1: Critical ✅ COMPLETE
- [x] **unwrap() Reduction** - Production: ~15 unwraps, 1838+ tests pass ✅
- [x] **Split Large Files** - task_context.rs → 4 modules, mouse.rs → 3 modules ✅
- [x] **Fix Remaining Warnings** - 0 warnings, cargo-nextest CI ✅

### P2: Important ✅ COMPLETE
- [x] **Dependency Audit** - Removed 3 deps, 37 direct deps ✅
- [x] **Add Benchmark Suite** - `criterion`, 3 benches ✅
- [x] **Increase Bus Factor** - ARCHITECTURE.md, 8 ADRs ✅
- [x] **Config Loading Normalization** - Removed 2 dead fields ✅
- [x] **Click-Learning Persistence** - `LearningEngine`, 12 tests ✅

### P3: Lower Priority ✅ COMPLETE
- [x] **Session execution guard** - `SessionExecutionGuard`, shutdown tests ✅
- [x] **TaskContext interaction pipeline** - `interaction_pipeline.rs` ✅
- [x] **Runtime shutdown coordination** - `ShutdownManager` ✅
- [x] **CLI task parsing + registry** - `TaskRegistry`, `parser.rs` ✅
- [x] **Browser discovery / session assembly** - Verified complete ✅

### Accessibility Locator ✅ COMPLETE
- [x] Phases 1-6: Parser, Resolver, Action Paths, Compatibility, Telemetry, Pilot Task ✅
- [x] Coverage targets: 95%, 90%, 85%, 90% achieved ✅

### Test Coverage (Earlier Work) ✅ COMPLETE
- [x] **src/main.rs** - 8 tests ✅
- [x] **src/session/mod.rs** - 20+ tests ✅
- [x] **src/utils/mouse/trajectory.rs** - 25 tests ✅
- [x] **src/task/twitteractivity.rs** - 19 tests (~45% coverage) ✅
- [x] **src/task/twitterintent.rs** - 14 tests (~65% coverage) ✅

</details>

---

## Notes
- **cargo-nextest** migration complete: 2-10x faster test runs ✅
- **Documentation audit** complete: 48 files stamped, drift fixed ✅
- **Spec system** operational: 7 specs in `_done/`, lint passing ✅
- **Current focus:** Coverage watchlist re-baseline, Performance optimization
- **Future:** V2 Twitter roadmap after coverage + performance targets met
- **Confidence key:** (95%+) Verified | (80-94%) Mostly verified | (60-79%) Partially verified | (<60%) Need investigation
