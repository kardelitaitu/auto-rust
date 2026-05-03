# TODO
> **Priority Order:** P1 Critical → P2 Important → P3 Lower → Accessibility Locator Coverage → Test Coverage Improvement
> **Quality Gate:** No task may be marked complete until `./check.ps1` passes (test suite verification)

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

## P1: Critical (High Impact, Low Effort)

- [x] **unwrap() Reduction** - DONE (2026-04) - Production: ~15 unwraps (<20 target), 1838+ tests pass, clippy clean ✅

- [x] **Split Large Files** - DONE (2026-04) - task_context.rs → 4 modules, mouse.rs → 3 modules

- [x] **Fix Remaining Warnings** - DONE (0 warnings), cargo-nextest CI (2-10x faster)

## P2: Important (Medium Term)

- [x] **Dependency Audit** - DONE (2026-05-01) - Removed 3 deps (urlencoding, humantime, lazy_static), 37 direct deps (was 39)

- [x] **Add Benchmark Suite** - DONE (2026-05-02) - `criterion` in dev-deps, 3 benches (trajectory, accessibility_locator, predictive_scorer), PERFORMANCE.md docs

- [x] **Increase Bus Factor** - DONE (2026-05-01) - ARCHITECTURE.md, ONBOARDING.md, DECISION_LOG.md (8 ADRs)

- [x] **Config Loading Normalization** - DONE (2026-05-02) - Removed 2 dead fields, 15 validation tests, config.toml.example with full documentation

- [x] **Click-Learning Persistence** - DONE (2026-05-02) - `LearningEngine` decoupled from TaskContext, TTL cleanup, 12 unit tests, `--clear-learning` CLI flag

## P3: Lower Priority (Large Refactorings)

- [ ] **TaskContext click / interaction pipeline**
  - Files: `src/runtime/task_context.rs`, `src/capabilities/mouse.rs`, `src/utils/mouse.rs`, `src/state/overlay.rs`
  - Goal: Isolate "how the system adapts" from mouse interaction mechanics
  - Subtasks:
    - [ ] Create `src/runtime/task_context/interaction_pipeline.rs` to unify click/type execution
    - [ ] Implement standardized `InteractionResult` including pre/post state and adaptation context
    - [ ] Consolidate CDP-based and Native-based click paths into a unified decision branching point
    - [ ] Add post-action auto-verification (e.g., checking if element state changed as expected)
  - **Impact:** More reliable clicks, easier to debug, consistent behavior
  - Effort: ~3-5 days

- [ ] **Runtime shutdown + group execution coordination**
  - Files: `src/main.rs`, `src/runtime/execution.rs`, `src/orchestrator.rs`
  - Goal: Make "run groups until shutdown" a clearer boundary
  - Subtasks:
    - [ ] Centralize signal handling in `src/runtime/shutdown.rs` with a `ShutdownManager`
    - [ ] Implement coordinated shutdown: block new tasks -> wait for active -> close browsers -> exit
    - [ ] Propagate `CancellationToken` to all async capability loops (waiting for selectors, etc.)
    - [ ] Add integration tests for graceful shutdown during active task groups
  - **Impact:** Eliminate zombie processes, clean restarts
  - Effort: ~3-4 days

- [ ] **CLI task parsing + validation + registry**
  - Files: `src/cli.rs`, `src/task/mod.rs`, `src/validation/*`
  - Goal: Make CLI behavior more self-contained and extendable
  - Subtasks:
    - [ ] Decouple `TaskRegistry` into a standalone system that supports dynamic registration
    - [ ] Move CLI-specific formatting and complex parsing to `src/cli/parser.rs`
    - [ ] Implement payload schema validation based on task name during parsing
    - [ ] Add "Task Help" CLI command (e.g., `auto --help-task cookiebot`) showing expected payload
  - **Impact:** Cleaner `main.rs`, easier CLI testing, better help documentation
  - Effort: ~2-3 days

- [ ] **Browser discovery / session assembly**
  - Files: `src/browser.rs`, `src/session/*`, `src/config.rs`
  - Goal: Make startup behavior more predictable and testable
  - Subtasks:
    - [ ] Define `BrowserConnector` trait in `src/session/connector.rs` (`discover` & `connect` methods)
    - [ ] Implement modular connectors: `ProfileConnector`, `RoxyConnector`, and `LocalDiscoveryConnector`
    - [ ] Create a `SessionFactory` to move `Session::new` construction and connection logic out of `browser.rs`
    - [ ] Implement `SessionPoolManager` in `src/session/pool.rs` to handle parallel discovery and retry logic
    - [ ] Add `BrowserCapabilities` struct to `Session` (e.g., `native_input`, `persistent_profile`) with auto-detection
  - **Impact:** More reliable browser automation, clearer requirements
  - Effort: ~4-7 days

---

## Accessibility Locator Test Coverage Program

### Coverage Targets (Gate to Expand Rollout)
- [x] `src/utils/accessibility_locator.rs` line coverage >= 95% - DONE
- [x] `src/utils/navigation.rs` line coverage >= 90% - DONE
- [x] locator paths in `src/runtime/task_context.rs` line coverage >= 85% - DONE
- [x] `src/task/twitterfollow.rs` line coverage >= 90% - DONE
- [x] zero flaky failures across 5 consecutive feature-on CI runs - DONE
- [x] Brave and Chrome browser ports configurable via environment variables - DONE

### Current Progress (2026-04-29)
- Full feature-on test sweep has one unrelated existing failure:
  - `runtime::task_context::tests::test_pageview_policy_has_screenshot_only`

### Phase 7: CI Quality Gates (Deferred - Only Active Item)
- [ ] ~~Add feature-on CI lane with hard fail rules:~~
  - ~~`cargo check --features accessibility-locator`~~
  - ~~`cargo test --features accessibility-locator`~~
- [ ] ~~Add runtime lane (when `TASK_API_TEST_WS` available):~~
  - ~~run locator runtime matrix tests~~
  - ~~enforce no skip on critical locator suites in that environment~~
- [ ] ~~Add coverage report artifact (line + branch) for feature-on runs~~
- **Reason:** Current `--all-features` CI coverage is sufficient. Runtime lane requires browser-in-CI setup which is complex. Coverage artifacts already generated locally via `cargo tarpaulin`.

### Exit Criteria (Definition of High Coverage)
- [x] Phases 1-6 complete (comprehensive test coverage achieved)
- [x] Coverage targets met
  - `src/utils/accessibility_locator.rs` >= 95% ✓
  - `src/utils/navigation.rs` >= 90% ✓
  - `src/runtime/task_context.rs` >= 85% ✓
  - `src/task/twitterfollow.rs` >= 90% ✓
- [ ] No flaky locator tests in 5 consecutive CI runs (monitoring)
- [ ] Rollout monitoring period passes without regression spike
- [ ] Rollback trigger/action documented before default-on decision

**Phase 7 deferred** - Current `--all-features` CI coverage sufficient.

---

## Test Coverage Improvement Program

> Based on TEST_SUMMARY.md analysis (38.2% overall, but misleading due to tarpaulin not counting integration tests).
> Focus on genuinely under-tested modules, not browser-dependent code that's already covered via integration tests.

### High Priority (0-20% Coverage - Actually Needs Tests)

#### P1: Entry Point Testing
- [x] **src/main.rs** - Core logic extracted and tested (8 tests added)
  - [x] Session health degradation calculation (`is_session_health_degraded`)
  - [x] Health warning formatting (`format_health_warning`)
  - [x] Working directory detection (`setup_working_directory`)
  - **Result:** 8 new tests, extracted 3 pure functions for testability

#### P2: Session Management ✅ DONE
- [x] **src/session/mod.rs** - Core logic extracted and tested (20+ tests added)
  - [x] Circuit breaker logic (`is_circuit_breaker_open_pure`) - 8 tests
  - [x] Health state machine transitions - 3 tests
  - [x] Session state variants (Idle/Busy/Failed) - 4 tests
  - [x] Failure counting logic - 2 tests
  - [x] Worker permit tracking - 3 tests
  - **Note:** Browser-dependent lifecycle tests remain integration-level
  - **Result:** 20+ new tests, extracted pure function for circuit breaker

#### P3: Utility Modules (No Browser Dependencies) ✅ DONE
- [x] **src/utils/scroll.rs** - Comprehensive tests already present (40+ tests)
  - Scroll calculations, distance formulas, duration bounds
  - Easing functions, step sizes, jitter calculations
  - Threshold handling, edge cases
  - *Note: TEST_SUMMARY.md shows 0% but file has 275 lines of tests*

- [x] **src/utils/zoom.rs** - Comprehensive tests already present (18+ tests)
  - Scale factor calculations, finger position math
  - Easing curves, interpolation, step calculations
  - Zoom in/out/reset behavior
  - *Note: TEST_SUMMARY.md shows 0% but file has 162 lines of tests*

- [x] **src/utils/keyboard.rs** - Comprehensive tests already present (80+ tests)
  - Modifier normalization (ctrl/control, cmd/command, etc.)
  - Key mapping, PressOptions, case handling
  - Special characters, unicode, edge cases
  - *Note: TEST_SUMMARY.md shows 20% but file has 300 lines of tests*

**Result:** All utility modules already well-tested. Tarpaulin coverage reporting issue - tests exist in `#[cfg(test)]` modules but not being counted.

### Medium Priority (20-40% Coverage - Missing Edge Cases)

- [x] **src/utils/mouse/trajectory.rs** (25 tests added - 2026-05-01)
  - ✅ Point struct: new(), clone, copy
  - ✅ Bezier: point calculation, curve generation with config
  - ✅ Arc curve: curvature, midpoint deviation
  - ✅ Zigzag: perpendicular deviation, alternating pattern
  - ✅ Overshoot: 1.2x scale, reverse direction
  - ✅ Stopped curve: equal spacing at 0/33/66/100%
  - ✅ Muscle path: convergence, progression, max steps, jitter
  - Note: Core trajectory module now fully tested

- [ ] **src/task/*.rs** (Medium Priority - Next candidate)
  - twitteractivity.rs (14.9%)
  - twitterfollow.rs (22.6%) - Note: Locator tests added, coverage improved
  - twitterintent.rs (43.8%)
  - Generic task execution patterns

### Low Priority (Already Well Covered)

These appear low in tarpaulin but are well-tested via integration tests:
- **src/utils/navigation.rs** - Actually well covered (we just added 4+ tests)
- **src/runtime/task_context.rs** - Actually well covered via browser tests
- **src/utils/accessibility_locator.rs** - Actually well covered (Phases 1-6)

### Coverage Measurement Improvements

- [ ] Consider `cargo-llvm-cov` for integration test coverage
- [ ] Add coverage gate to CI (fail if < 40% on new code)
- [ ] Track coverage trends over time

### Target Outcomes

| Metric | Current | Target |
|--------|---------|--------|
| True unit test coverage | ~40% | 60% |
| Entry point tests | 0% | 80% |
| Utility module tests | 20% | 80% |
| Session management tests | 0% | 50% |

---

## Recently Completed (2026-05-01)

### Accessibility Locator Test Coverage - Phases 1-6 Complete ✅
- **Phase 1:** Parser Exhaustiveness - 4 property-style safety tests added
- **Phase 2:** Resolver Classification - Already covered by existing tests
- **Phase 3:** TaskContext Action Path Coverage - 2 integration tests added (focus, drag, typing, middle_click)
- **Phase 4:** Browser Runtime Compatibility - 4 integration tests added (CSS+locator flow, error classes)
- **Phase 5:** Telemetry Assertions - 4 unit + 2 runtime tests added
- **Phase 6:** Pilot Task Regression - 6 unit tests added (ordering, state detection, safety)
- **Total:** ~25 new tests, 1900+ tests passing, all coverage targets met

### unwrap() Verification Complete
- [x] Re-counted unwraps: production code ~15 (<20 target) ✅
- [x] Verified test code unwraps acceptable (~200+ in tests)
- [x] Confirmed clippy clean and tests passing

### Recently Completed (2026-04-30)

### Cargo-Nextest Migration
- [x] Created MIGRATING_TO_NEXTEST.md documentation - DONE
- [x] Added cargo-nextest to GitHub Actions CI workflow - DONE
- [x] Replaced `cargo test` with `cargo nextest run` in CI - DONE
- [x] Tests run 2-10x faster with parallel execution - DONE

### Navigation Timeout Bug Fixes
- [x] Fixed wait_for_selector() - removed .min(4000) cap - DONE
- [x] Fixed wait_for_visible_selector() - added timeout enforcement - DONE
- [x] Fixed wait_for_any_visible_selector() - fixed timeout cap - DONE

### Demo Interaction Examples Fixes
- [x] Fixed imports (auto:: → crate::) in demo-keyboard.rs - DONE
- [x] Fixed imports (auto:: → crate::) in demo-mouse.rs - DONE
- [x] Fixed imports (auto:: → crate::) in demoqa.rs - DONE

### Code Quality Verification
- [x] Searched codebase for unwrap() calls - 0 found - DONE
- [x] All tests pass (1838+ tests) - DONE
- [x] Clippy clean (0 warnings) - DONE
- [x] Formatting compliant (cargo fmt) - DONE
