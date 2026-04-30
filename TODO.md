## P1: Critical (High Impact, Low Effort)

- [ ] **Reduce unwrap() Count (330 → <100)**
  - **Status: 137 fixed, ~116 remaining** ✅ Major progress made
  - Risk: 330 unwraps could panic in production
  - Action: Audit and convert to `?` or `ok_or()` patterns
  - Files to prioritize: `task_context.rs`, `mouse.rs`, `twitteractivity.rs`
  - Effort: ~2-3 days
  - Subtasks:
    - [x] **Audit Phase**
      - [x] Run `grep -rn "\.unwrap()" src/ --include="*.rs" > /tmp/unwraps.txt`
      - [x] Categorize: test code vs production code vs examples
      - [x] Count baseline: `unwraps_test=~90, unwraps_prod=~20, unwraps_examples=~11`
    - [x] **session/mod.rs (Critical - 30 unwraps)** ✅ DONE
      - [x] Fixed all 3 production code unwraps + 27 test unwraps
    - [x] **task_context.rs (Critical - 4 unwraps)** ✅ DONE
      - [x] Fixed all 4 test unwraps (lines 1197, 1733, 1790, 1791)
    - [x] **navigation.rs (High - 17 unwraps)** ✅ DONE
      - [x] Fixed all 17 test unwraps (serde_json + parse_selector)
    - [x] **twitteractivity_interact.rs (High - 9 unwraps)** ✅ DONE
      - [x] Fixed all 9 test unwraps (root_tweet_button_center_js)
    - [x] **result.rs (Low - 11 unwraps)** ✅ DONE
      - [x] Fixed all 11 test unwraps (serde_json serialize/deserialize)
    - [x] **state/overlay.rs (Low - 11 unwraps)** ✅ DONE
      - [x] Fixed all 11 test unwraps (Option + thread joins)
    - [x] **llm/unified_processor.rs (MED - 13 unwraps)** ✅ DONE
      - [x] Fixed all 13 test unwraps (parse_batch_response_static)
    - [x] **llm/unified_action_processor.rs (MED - 10 unwraps)** ✅ DONE
      - [x] Fixed all 10 test unwraps (process_candidate)
    - [x] **llm/sentiment_aware_processor.rs (MED - 9 unwraps)** ✅ DONE
      - [x] Fixed all 9 test unwraps (process_reply/process_quote)
    - [x] **llm/client.rs (MED - 9 unwraps)** ✅ DONE
      - [x] Fixed 7 test unwraps + 2 production unwraps
    - [x] **orchestrator.rs (MED - 4 unwraps)** ✅ DONE
      - [x] Fixed 4 test unwraps (semaphore acquire)
    - [x] **api/client.rs (MED - 5 unwraps)** ✅ DONE
      - [x] Fixed 3 test unwraps + 2 production unwraps (execute function)
    - [x] **native_input.rs (Medium - 2 unwraps)** ✅ DONE
      - [x] Fixed line 433: `delays.iter().min().unwrap()` → `expect("delays should not be empty")`
      - [x] Fixed line 434: `delays.iter().max().unwrap()` → `expect("delays should not be empty")`
    - [x] **mouse.rs (Low priority - test code only)** ✅ DONE
      - [x] Fixed 2 test unwraps in `#[cfg(test)]` blocks
      - [x] Converted to `expect("...")` with context
    - [x] **cli.rs (Critical - 1 unwrap)** ✅ DONE
      - [x] Fixed production code unwrap at line 191: `current_task.as_ref().unwrap()` → `expect("current_task should be Some")`
    - [ ] **Verification**
      - [x] Run `cargo test --all-features` - all tests pass ✅
      - [x] Run `cargo clippy --all-targets --all-features -- -D warnings` - clean ✅
      - [ ] Re-count unwraps: target <100 total, <20 in production code

- [ ] **Split Large Files** (~1 week)
  - `task_context.rs` (5,880 LOC) → 4 focused modules
    - [x] Phase 1: Create module structure (Day 1) ✅ DONE
      - [x] Create `src/runtime/task_context/` directory
      - [x] Extract shared types to `types.rs` (Rect, HttpResponse, FileMetadata, outcome structs)
      - [x] Run tests: `cargo test task_context::` - 109 passed
    - [x] Phase 2: Extract click_learning.rs (Day 1-2) ✅ DONE
      - [x] Move `ClickLearningState`, `ClickTimingContext`, `ClickAdaptation`
      - [x] Move persistence: `save_click_learning()`, `load_click_learning()`
      - [x] Re-export public types, run tests - 109 passed
    - [x] Phase 3: Extract query.rs (Day 2) ✅ DONE
      - [x] Move query methods: `exists()`, `visible()`, `text()`, `html()`, `attr()`, `value()`
      - [x] Move `wait_for()`, `wait_for_visible()`, `url()`, `title()`, `viewport()`
      - [x] Run tests - 109 passed
    - [x] Phase 4: Extract interaction.rs (Day 3-4) ✅ DONE
      - [x] Extract keyboard: `press()`, `press_with_modifiers()` to interaction module
      - [x] Extract clipboard: `copy()`, `cut()`, `paste()` to interaction module  
      - [x] Extract scroll: `scroll_to()`, `scroll_back()` use interaction module
      - [x] Complex methods (click, hover, drag, r#type) remain in TaskContext due to state dependencies
      - [x] Run full test suite - 109 passed, clippy clean
    - [x] Phase 5: Cleanup (Day 4-5) ✅ DONE
      - [x] `task_context.rs` is now organized layer with re-exports
      - [x] Added convenient aliases: `dom_query`, `actions`
      - [x] Verified `runtime/mod.rs` exports task_context correctly
      - [x] Public API unchanged, fully backward compatible
      - [x] 109 tests pass, clippy clean
  - `mouse.rs` (3,773 LOC) → 3 focused modules
    - [x] Phase 1: Create module structure (Day 1) ✅ DONE
      - [x] Create `src/utils/mouse/` directory
      - [x] Extract `types.rs`: `ClickOutcome`, `HoverOutcome`, `NativeCursorOutcome`, `MouseButton`
      - [x] All types properly exported and re-exported
      - [x] Tests pass, clippy clean
    - [x] Phase 2: Extract trajectory.rs (Day 1-2) ✅ DONE
      - [x] Move path generation: `generate_bezier_curve_with_config()`, `generate_arc_curve()`
      - [x] Move curves: `generate_zigzag_curve()`, `generate_overshoot_curve()`, `generate_stopped_curve()`, `generate_muscle_path()`
      - [x] Create standalone `Point` struct in trajectory module
      - [x] Keep wrapper functions in mouse.rs delegating to trajectory
      - [x] All 63 tests pass, clippy clean
    - [ ] Phase 3: Extract native.rs (Day 3-4)
      - [ ] Move native click infrastructure: `NATIVE_CLICK_LOCK`, calibration cache
      - [ ] Move calibration: `NativeClickCalibration`, `NativeClickFingerprint`, `solve_calibration_from_probe_samples()`
      - [ ] Move native functions: `native_click_selector_human()`, `native_move_cursor_human()`
      - [ ] Move test helpers: `set_nativeclick_forced_calibration_for_tests()`
    - [ ] Phase 4: Core cleanup (Day 4-5)
      - [ ] `mouse.rs` becomes thin re-export layer
      - [ ] `core.rs` keeps: `left_click_at()`, `right_click_at()`, `dispatch_click()`
      - [ ] Keep in core: `click_selector_human()`, `hover_selector_human()`, overlay functions
      - [ ] Run full test suite, update docs
  - **Dependencies to watch:**
    - `profile.rs` imports `CursorMovementConfig`, `PathStyle`, `Precision`, `Speed` from mouse
    - `task_context.rs` imports `ClickOutcome`, `HoverOutcome`, `NativeCursorOutcome` from mouse
    - Both need stable public API during refactor

- [ ] **Fix Remaining Warnings**
  - 2 warnings in `native_input.rs` (useless comparisons)
  - Action: Clean up u64 >= 0 assertions
  - Effort: 30 minutes

## P2: Important (Medium Term)

- [ ] **Dependency Audit**
  - 39 dependencies - some may be redundant
  - Action: Audit with `cargo tree` and remove unused
  - Priority: `log` crate (appears unused with tracing), `do-over` (unknown utility)
  - Effort: 1 day

- [ ] **Add Benchmark Suite**
  - No performance benchmarks currently
  - Action: Add `criterion` for hot paths (selector resolution, mouse paths)
  - Benefit: Prevent performance regressions
  - Effort: 2 days

- [ ] **Increase Bus Factor**
  - Single dominant contributor pattern
  - Action: Document architecture decisions (ADRs), add onboarding guide
  - Effort: Ongoing

---

## Accessibility Locator Test Coverage Program (High Coverage Target)



### Current Progress (2026-04-29, Code-Proven)
- [ ] Full feature-on test sweep has one unrelated existing failure:
  - `runtime::task_context::tests::test_pageview_policy_has_screenshot_only`

### Coverage Targets (Gate to Expand Rollout)
- [ ] `src/utils/accessibility_locator.rs` line coverage >= 95%
- [ ] `src/utils/navigation.rs` line coverage >= 90%
- [ ] locator paths in `src/runtime/task_context.rs` line coverage >= 85%
- [ ] `src/task/twitterfollow.rs` line coverage >= 90%
- [ ] zero flaky failures across 5 consecutive feature-on CI runs

### Phase 1: Parser Exhaustiveness (`src/utils/accessibility_locator.rs`)
- [ ] Positive matrix:
  - exact match default behavior
  - contains match behavior
  - with and without scope
  - whitespace tolerance around segments
- [ ] Negative matrix:
  - missing `role`
  - missing `name`
  - double-quoted `name`
  - invalid `match` enum
  - malformed bracket ordering
  - invalid scope quoting/format
- [ ] Property-style safety tests:
  - parser never panics on arbitrary malformed input
  - malformed locator always maps to `locator_parse_error` downstream

### Phase 2: Resolver Classification (`src/utils/navigation.rs`)
- [ ] `selector_exists` coverage:
  - 0 match => `locator_not_found`
  - 1 match => success
  - >1 match => `locator_ambiguous`
- [ ] `selector_is_visible` coverage:
  - hidden/ignored nodes filtered correctly
  - visible multi-match => `locator_ambiguous`
- [ ] `selector_text`/`selector_value` coverage:
  - no semantic value => `locator_not_found`
  - multi-match semantic value => `locator_ambiguous`
- [ ] `selector_action_point` coverage:
  - valid point extraction
  - missing backend node / box model failure => deterministic not-found path
- [ ] Unsupported operations coverage:
  - `selector_html` + locator => `locator_unsupported`
  - `selector_attr` + locator => `locator_unsupported`

### Phase 3: TaskContext Action Path Coverage (`src/runtime/task_context.rs`)
- [ ] Locator-path success tests for:
  - `focus`, `hover`, `click`, `double_click`, `right_click`, `middle_click`, `drag`
- [ ] Locator-path failure mapping tests for:
  - ambiguous target
  - invalid scope
  - not found target
- [ ] Explicit unsupported test:
  - `nativeclick(locator)` => `locator_unsupported`
- [ ] Input helpers coverage:
  - locator-aware `select_all`
  - locator-aware typing verification path

### Phase 4: Browser Runtime Compatibility (`tests/task_api_behavior.rs`)
- [ ] Expand CSS compatibility matrix:
  - mixed CSS + locator flow in one scenario
  - CSS-only behavior unchanged with feature on
- [ ] Expand locator integration matrix:
  - exact + contains + scope combinations
  - deterministic errors for all failure classes
- [ ] Action-path matrix additions:
  - `middle_click` end-to-end assertion
  - `drag` locator->locator end-to-end assertion
  - `focus` end-to-end assertion

### Phase 5: Telemetry Assertions (Unit + Runtime)
- [ ] Unit telemetry assertions (`src/utils/navigation.rs`):
  - `selector_mode=a11y` + `locator_result=ok`
  - `selector_mode=css` + `locator_result=ok`
- [ ] Runtime telemetry assertions (`tests/task_api_behavior.rs`):
  - `locator_result=not_found`
  - `locator_result=ambiguous`
  - `locator_result=unsupported`
- [ ] Verify telemetry field stability:
  - `locator_role`
  - `locator_match_mode`
  - `locator_scope_used`

### Phase 6: Pilot Task Regression (`src/task/twitterfollow.rs`)
- [ ] Candidate ordering tests:
  - scoped locator preferred before global
  - global locator preferred before CSS fallback
- [ ] State detection tests:
  - not-followed path (`Follow @...`, `...-follow`)
  - already-following path (`Following @...`, `...-unfollow`)
- [ ] Safety tests:
  - no pending/private-state dependency
  - preserves fallback behavior when semantic locator is unavailable

### Phase 7: CI Quality Gates (Required)
- [ ] Add feature-on CI lane with hard fail rules:
  - `cargo check --features accessibility-locator`
  - `cargo test --features accessibility-locator`
- [ ] Add runtime lane (when `TASK_API_TEST_WS` available):
  - run locator runtime matrix tests
  - enforce no skip on critical locator suites in that environment
- [ ] Add coverage report artifact (line + branch) for feature-on runs

### Exit Criteria (Definition of High Coverage)
- [ ] All phases above complete
- [ ] Coverage targets met
- [ ] No flaky locator tests in 5 consecutive CI runs
- [ ] Rollout monitoring period passes without regression spike
- [ ] Rollback trigger/action documented before default-on decision

#### TODO LIST -- *Scratch the list if its completed*
- [x] ~~Split largest modules (task_context.rs, mouse.rs) to reduce concentration risk.~~ (Moved to P1)
- [x] ~~Add reproducible benchmark and reliability reports~~ (Moved to P2)
---
- [ ] TaskContext click / interaction pipeline (`src/runtime/task_context.rs`, `src/capabilities/mouse.rs`, `src/utils/mouse.rs`, `src/internal/profile.rs`, `src/state/overlay.rs`)
  - This is the full path from task API call → mouse movement / timing / verification / fallback behavior.
  - It’s one user-facing concept, but the implementation spans several layers and helpers.
  - This is a strong candidate for deepening because callers really just want “click this thing,” not the full internal machinery.
- [ ] Runtime shutdown + group execution coordination (`src/main.rs`, `src/runtime/execution.rs`, `src/orchestrator.rs`)
  - This is the logic that decides when task groups start, stop, and how shutdown interrupts them.
  - Right now, the shutdown wiring is split between `main`, runtime execution, and orchestrator code, so the flow is harder to follow than it should be.
  - Deepening this would make “run groups until shutdown” one clearer boundary.
- [ ] CLI task parsing + validation + registry (`src/cli.rs`, `src/task/mod.rs`, `src/validation/*`)
  - This is the logic that turns command-line arguments into normalized, validated tasks.
  - It currently mixes parsing rules, task name normalization, registry lookups, and validation across multiple modules.
  - Deepening it would make the CLI behavior more self-contained and easier to extend safely.
- [ ] Browser discovery / session assembly (`src/browser.rs`, `src/session/*`, `src/config.rs`)
  - This is the part that finds browsers, connects to them, filters them, and turns them into sessions.
  - The same concept is spread across config, browser discovery, and session construction, which makes the startup path feel fragmented.
  - A deeper module here would make browser setup easier to test and reason about.
- [ ] Config loading / normalization / validation (`src/config.rs`, `config/*.toml`)
  - This is the system that loads config files, applies defaults, merges env overrides, and validates the result.
  - It’s a classic “wide surface, many rules” module where small changes can ripple unexpectedly.
  - A deeper config boundary would make startup behavior more predictable and testable.
- [ ] Click-learning / behavior adaptation persistence (`src/runtime/task_context.rs`, `src/utils/mouse.rs`, `click-learning/`, `src/internal/profile.rs`, `src/utils/profile.rs`)
  - This is the adaptive behavior system that learns from prior interactions and persists those patterns.
  - It couples learning, timing adaptation, profile behavior, and persistence, so it’s easy for the logic to become scattered.
  - Deepening it would help isolate “how the system adapts” from the mechanics of mouse interaction.



- [ ] Fix remaining unwrap() in test code ~116 unwraps remaining
