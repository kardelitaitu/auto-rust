# Build Journal

## 23-06-26

### P1: LLM Module — Extract pure functions from unified_processor.rs ✅
- Created `src/llm/processor.rs` (12 pure functions + 3 types + 30 tests + 6 fuzz tests)
- Stripped `src/llm/unified_processor.rs` to 2 async methods only
- Deleted stale duplicate `src/utils/twitter/unified_processor.rs` (169 lines behind)
- Updated mod.rs, twitterreply.rs, twitterquote.rs import paths
- Verification: 306 LLM tests, clippy clean, compilation clean

### P2: Session Module — Extract duration math from duration.rs ✅
- Moved `duration_with_variance()` and `duration_ms()` to `session/duration.rs`
- Added `DurationMs::with_variance()`, `.checked_add()`, `.checked_sub()`
- Added 12 new tests (32 total for duration module)
- Full backwards compat via re-export chain through session/mod.rs → utils/timing.rs
- Verification: compiles clean, clippy clean, 32 duration tests passing

### P3: Orchestrator — status
- health.rs: format_duration, broadcast_execution_count, should_mark_session_unhealthy — already tested ✅
- guards.rs: GlobalExecutionSlot, SessionExecutionGuard, acquire_global_execution_slot — already tested ✅
- retry.rs: TaskAttemptFailure tested, Backoff extracted as RetryPolicy::delay_for_attempt in api/client.rs ✅
- execution.rs: execute_group_with_cancel — tested in orchestrator/mod.rs tests

### Phase 9 #1: Dead code cleanup in predictive_scorer.rs ✅
- Replaced 19 `#[allow(dead_code)]` with `#[cfg_attr(not(test), allow(dead_code))]`
- Added 11 test assertions to read all previously-unread fields
- All fields now documented in tests with their expected default values
- Verification: 128 adaptive tests passing, clippy clean, compilation clean

### Phase 9 #3: Expect hotspots — false positive ✅
- All `.expect("Should succeed")` calls in `api/client.rs` were in `#[cfg(test)]`, which is idiomatic
- The single production `#[allow(clippy::expect_used)]` in `execute()` was restructured to use `match` + `unreachable!()`, removing the need for the allow annotation

### Phase 9 #6: std::sync::Mutex audit — safe ✅
- `src/logger.rs`: `Mutex<File>` only used in synchronous `Log::log()`/`Log::flush()` — safe
- `src/task/dsl/api.rs`: `Arc<Mutex<...>>` in `#[cfg(test)]` mock — never held across `.await` — safe
- `src/utils/mouse/native.rs`: `Mutex<HashMap<...>>` for calibration cache/trace hooks — synchronous callbacks only — safe

### Phase 9 #9: Clippy allow cleanup ✅
- Removed `#[allow(clippy::expect_used)]` from `api/client.rs` by restructuring `execute()` retry loop
- Remaining allows (`cast_precision_loss` in health_logger + delay_for_attempt, `unused_imports` in capabilities) are legitimate and minimal

### Phase 9 #4: Split session/mod.rs into sub-modules ✅
- Created `src/session/lifecycle.rs` (new, +162 lines) — 19 methods: register_page, state, health, circuit breaker accessors via `impl super::Session`
- Created `src/session/worker.rs` (new, +198 lines) — 10 methods: acquire_worker, acquire_page, release_page, circuit breaker internals, graceful_shutdown via `impl super::Session`
- Updated `src/session/mod.rs` (1,799 → ~800 lines) — removed moved impl blocks, added `mod lifecycle;` and `mod worker;`, cleaned up unused imports
- All tests preserved (241 session tests, all passing)
- Verification: cargo check --all-targets ✅, clippy ✅, cargo test --lib session ✅ (241 passed)

### Phase 9 #5: Split utils/profile.rs into sub-modules ✅
- **Deleted**: `src/utils/profile.rs` (1,746 lines)
- **Created**: `src/utils/profile/mod.rs` (~450 lines) — Core types, derived behaviors, all 63 tests
- **Created**: `src/utils/profile/presets.rs` (~1,050 lines) — 21 preset constructors + from_preset() + p() helper
- Verification: cargo check --all-targets ✅, clippy ✅, 63 profile tests passing

### Phase 9 #7: Unnecessary .clone() optimization ✅
- Fixed `last_error.clone().unwrap_or_else(|| ...)` → `as_deref().unwrap_or(...).to_string()` in execution.rs and retry.rs
- Fixed clippy `to_string_in_format_args` lint in execution.rs (removed unnecessary .to_string() inside warn!())
- Verification: cargo check ✅, clippy ✅

### Phase 9 #8: Stream simplification audit ✅
- All FuturesUnordered/StreamExt patterns appropriate for their use cases — no change needed
- execution.rs: per-task cancellation ✓ guards.rs: test concurrency ✓ connector.rs: I/O-bound port scan ✓ factory.rs: parallel session creation ✓

### P3 Orchestrator: Tests verification ✅
- health.rs, guards.rs, retry.rs all have comprehensive tests
- Backoff already extracted as RetryPolicy::delay_for_attempt()

### P1 LLM Module: Tests verification ✅
- reply_strategies.rs: 20+ tests, reply_engine.rs: 20+ tests
- Pure functions already extracted; 288+ total LLM tests

### Phase 9 #10: Audit API surface — pub → pub(crate) visibility reductions ✅
- `src/lib.rs`: `pub mod internal;` → `pub(crate) mod internal;` — hides 12 implementation-detail sub-modules from external consumers
- `src/state/overlay.rs`: All 5 free functions + SessionOverlayState + 10 methods → `pub(crate)` — internal session state management
- `src/state/mod.rs`: `pub use overlay::*` → `pub(crate) use overlay::*` — matches source visibility
- `src/internal/mod.rs`: Removed unused `pub mod geometry { pub use crate::utils::geometry::*; }` — only unused internal re-export
- `src/utils/mouse/mod.rs`: Split `pub(crate) use overlay::run_cursor_overlay_background` from pub re-exports
- `src/utils/mouse/overlay.rs`: `pub fn run_cursor_overlay_background` → `pub(crate)` — matches parameter type visibility
- Verification: cargo check --all-targets ✅, clippy ✅

## 24-06-26

### Phase 10 #2: Dead code audit — 3 removals ✅
- `src/utils/keyboard.rs`: Removed `type_character()` and `typo_correction()` (private async functions only called by each other) + their `#[allow(dead_code)]` annotations
- `src/task/registry.rs`: Removed `allow_external: bool` field from `TaskRegistry` struct — stored but never read
- `src/task/dsl/api.rs`: Removed `set_count_result()` test helper from `#[cfg(test)]` mock module — never called
- Verified: cargo check ✅, clippy ✅

### Phase 10 #3: Production .expect() audit — 14 calls verified safe ✅
- `llm/models.rs`: 4 calls with compile-time NonZeroU32 constants (2048, 4096, 16384) — cannot fail
- `utils/math.rs`: 1 call guarded by std_dev <= 0.0 check above
- `utils/native_input.rs`: 2 calls in #[cfg(test)] test code
- `twitteractivity_types.rs`: 4 calls in From impls — intentional panic on empty string
- `twitteractivity_llm_validation.rs`: 3 calls compiling hardcoded regex patterns — cannot fail
- No changes needed — all safe

### Phase 10 #4: unused_self audit — 35 annotations assessed, 3 functions converted ✅
- `src/utils/twitter/decision/strategies/persona.rs`: Converted `contains_any()` and `calculate_base_score()` to associated functions. Updated all test/proptest call sites from method syntax to `PersonaStrategy::fn()` syntax.
- `src/utils/twitter/decision/strategies/hybrid.rs`: Converted `combine_llm_primary()` to associated function. Updated all test/proptest call sites to `HybridStrategy::fn()` syntax.
- `validator.rs`: Assessed — `count_actions()` keeps `&self` as it's pub(crate) API method. `validate_action()` also needs `&self` for accessing `self.max_nesting_depth`.
- Trait-impl methods in `sentiment/core.rs`, `legacy.rs`, `unified.rs` cannot be converted without changing trait signatures.
- Verified: cargo check --all-targets ✅, clippy ✅, 260 decision tests passing ✅

### Phase 10 #8: too_many_arguments audit — export_summary_to refactored ✅
- `src/metrics.rs`: Replaced 4 individual fan-out params (planned_groups, completed_groups, planned_executions, actual_executions) with `&FanOutMetrics` struct (already existed)
- Removed `#[allow(clippy::too_many_arguments)]` annotation (param count 9→6)
- Updated `export_summary()` to pass `&FanOutMetrics::default()`
- Updated `src/main.rs` caller to construct `FanOutMetrics` struct
- Updated both test call sites
- Remaining 7 functions with `too_many_arguments` assessed as legitimate (constructors, dispatch)
- Verified: cargo check --all-targets ✅, clippy ✅, 36 metrics tests passing ✅

### Phase 10 #1: llm/client.rs split — already done in codebase ✅
- Confirmed `src/llm/client.rs` monolithic file no longer exists
- `src/llm/client/` directory has `mod.rs`, `fallback.rs`, `tests.rs` — clean modular structure
- Verified: compiles clean ✅

### Phase 10 #5–#7, #9: Large file split assessments ✅
- **#5 twitteractivity_limits.rs** (1,609 ln): ~500ln production (2 structs), ~1,100ln inline tests (idiomatic Rust). Low-ROI for split.
- **#6 metrics.rs** (1,372 ln): counter types are ~30ln, production ~550ln. Low-ROI for split.
- **#7 mouse/mod.rs** (1,414 ln): already split into 7 sub-modules (adaptive, cdp, curves, native, overlay, trajectory, types). Low-ROI for further split.
- **#9 dsl/parser.rs** (1,494 ln): ~200ln production code, ~1,300ln proptests (idiomatic Rust). Low-ROI for split.

### Phase 10 #10: .clone() hot-spot reduction assessment ✅
- Hot-path `last_error.clone()` pattern already fixed in Phase 9 #7
- Scanned high-density files: session/mod.rs (4 clones, async task spawning), twitteractivity_types.rs (5 clones, test assertions), twitteractivity_persona.rs (8 clones, builder pattern), twitteractivity_retry.rs (17 clones, Arc for test helpers)
- Remaining clones are necessary (Arc, async closures, builder taking self by value)
- Low-ROI for further optimization

### Bug fix: Missing `ReplyAnalysis` struct pre-existing compilation error ✅
- `src/utils/twitter/decision/strategies/legacy.rs`: Added missing `ReplyAnalysis` struct (3 f64 fields: positive_ratio, negative_ratio, spam_ratio)
- This struct was used by `analyze_replies()` but never defined — clearly an accidental deletion during prior refactoring
- Added `#[allow(dead_code)]` because `positive_ratio` field is written but not read externally
- Verified: cargo check --all-targets compiles clean ✅

### Final verification: Full lib test suite ✅
- `cargo test --lib`: **4,005 passed, 0 failed, 6 ignored**
- `cargo check --all-targets` ✅
- `cargo clippy --all-targets` ✅ (0 warnings)
- `cargo test --lib --no-run` ✅

### TODO.md housekeeping ✅
- Updated stale P1 table (referenced old monolithic `client.rs` → now points to `client/` dir)
- Added "What's Next" section with suggestions for future work

**All phases complete.**

### Git hooks: cargo fmt pre-commit + setup script ✅
- Created `.git/hooks/pre-commit` — runs `cargo fmt --check` on commit
- Exits cleanly when no `.rs` files staged; otherwise fails with a clear message
- Provides actionable error output: "Run 'cargo fmt' to auto-fix"
- Hook is executable and tested: `rustfmt` correctly catches bad formatting
- Project formatting is currently clean (`cargo fmt --check` ✅)

### Git hooks: setup script + bug fixes ✅
- Created `scripts/pre-commit` — version-controlled hook source with header pointing to `setup-hooks.sh`
- Created `scripts/setup-hooks.sh` — installer script that copies/symlinks hooks from `scripts/` to `.git/hooks/`
  - Symlinks on Unix, copies on Windows (MINGW/MSYS detection)
  - Supports `--list` flag, specific hook install, and full install mode
  - Makes hooks executable after copy
- **Bug fix #1**: Removed `set -e` from hook — was silently aborting before error message block when `cargo fmt --check` failed, swallowing the "❌ Commit rejected" guidance
- **Bug fix #2**: Changed grep pattern from `\\.rs$` to `[.]rs$` — `write_file` was double-escaping backslashes, causing grep to never match staged files (hook silently passed every commit)
- End-to-end verified: hook rejects bad formatting (exit 1 ✅), passes clean formatting (exit 0 ✅)

### Git hooks: commit-msg conventional-commit linter ✅
- Created `scripts/commit-msg` — version-controlled commit-msg hook source
- Enforces format: `type(scope): subject` or `type: subject`
- Recognizes: feat, fix, docs, chore, test, refactor, ci, perf, style, revert, build
- Skips merge commits, fixup!/squash!, WIP, and empty (draft) messages
- Allows breaking change `!` syntax: `feat!: ...`, `feat(scope)!: ...`
- Warns (non-blocking) on subject lines > 100 chars
- Updated `scripts/setup-hooks.sh` with wider glob patterns (`pre-commit* commit-msg* prepare-commit-msg* post-commit* pre-push*`)
- Tested 7 cases: bad message ❌, good type ✅, good scope ✅, empty ✅, bad type ❌, missing colon ❌, merge ✅
- Made source file executable for Unix symlink path

### P1: LLM Module — Final status verification ✅
- Confirmed pure functions extracted to `processor.rs` (12 functions + 3 types + 30 tests + 6 fuzz tests)
- Confirmed 306 LLM tests passing (reply_strategies 20+, reply_engine 20+, models 30+, processor 30+6 fuzz)
- Updated TODO.md with checked boxes, updated test counts, removed stale "288 tests" reference
- Clippy clean ✅

### P2–P3: All items complete ✅
- P2: All high-ROI items done. Pool concurrency tests blocked by browser dependency (noted in TODO).
- P3: health.rs, guards.rs, retry.rs all well-tested. Backoff extracted. No gaps.

### P4: Doc Audit — final stamping ✅
- Confirmed 33 audit stamps across docs/ tree (ARCHITECTURE.md, API docs, TASK docs, spec templates)
- `find-oldest-files.ps1` script fixed and verified
- TODO.md refreshed with current audit date

### Phase 9: All 10 items complete ✅
- #1 dead code cleanup, #3 expect hotspots, #4 large file split (session), #5 profile split,
  #6 mutex audit, #7 clone optimization, #8 stream audit, #9 clippy allow cleanup, #10 API surface
- All verified: cargo check --all-targets ✅, clippy ✅, lib tests passing ✅

## 24-06-26 (late)

## 26-06-26

### Spec 0030: Typing Realism — word pauses, full keyboard adjacency, profile-driven speed ✅
- **`src/utils/keyboard.rs`**:
  - Activated `word_pause_ms` in `natural_typing_profiled` — inserts `human_pause(word_pause, 30)` after every space character, clamped to [0, 1500]ms, placed after the entire character-dispatch block (not inside one branch) so it fires regardless of typo path
  - Expanded `get_similar_char` with 11 new QWERTY-adjacent mappings: `z↔x, c↔v, b→v, g↔h, j↔k, l→k, u→y` (all 26 letters now mapped)
  - Added `#[deprecated(since = "0.2.32")]` to `natural_typing` wrapper (hardcodes keystroke_mean_ms=120) directing callers to `natural_typing_profiled` — wrapper has zero actual callers, safe deprecation
- **Tests**: +14 new unit tests (3 word-pause logic, 11 per-character mapping). Updated proptest char range `'a'..='z'` → `' '..='~'` since all letters are now mapped. Updated `is_self_inverse` to include new reciprocal pairs.
- **Verification**: cargo check ✅ (zero errors, zero warnings), cargo test utils::keyboard ✅ (53/53 passed), cargo fmt ✅
- **Spec archived** to `docs/specs/_done/0030-typing-realism/`

### Cursor overlay enhancement — circle+ring, ghost trail, ripple click, configurable color/trail ✅
- **`src/utils/mouse/overlay.rs`**: Replaced basic dot with circle+ring design, 3-dot fading ghost trail managed via JS `dataset.trail`, expanding-ring ripple click effect, native cursor hiding via `body * { cursor: none !important; }`, GPU-composited transform positioning
- **`src/config/types.rs`**: Added `cursor_overlay_color: String` (default `#ff6600`) and `cursor_overlay_show_trail: bool` (default `true`) to `BrowserConfig`
- **`src/config/defaults.rs`**, **`src/config/env.rs`**: Default values and `CURSOR_OVERLAY_COLOR` / `CURSOR_OVERLAY_SHOW_TRAIL` env var overrides
- **`config/default.toml`**: Added inline config fields + env var documentation
- **`src/config/validation.rs`**: Added `is_valid_hex_color()` (validates `#RGB`, `#RRGGBB`, `#RGBA`, `#RRGGBBAA`), `BrowserConfig::validate()` checks color when `ms>0`, `validate_cursor_overlay_color()` standalone helper
- **Tests**: 13 hex color validation tests, 9 config unit tests (defaults/env/TOML integration), 2 `.env` pipeline tests, 13 direct `load_dotenv_defaults()` edge case tests
- All construction sites updated; `cargo check` clean

### Typing pipeline overhaul — InputEvent + Selection API fixes ✅ — InputEvent + Selection API fixes ✅
- **Root cause**: `execCommand('insertText')` and `execCommand('delete')` are deprecated and unreliable on React-managed contentEditables (Twitter's composer). They would fire incorrect events, causing garbled output where typo-corrected text showed corrupted characters.
- **`src/utils/keyboard.rs`**:
  - `dispatch_input_event`: Replaced `execCommand('insertText')` with InputEvent + Selection API (`beforeinput({insertText})` → `range.insertNode()` → `input({insertText})`)
  - `dispatch_key_event` Backspace: Replaced `execCommand('delete')` with InputEvent + Selection API (`beforeinput({deleteContentBackward})` → `range.deleteContents()` → `input({deleteContentBackward})`)
  - `natural_typing_profiled`: Wired up `typo_recovery_chance_pct` — now decides to correct (wrong→Backspace→right) or leave (wrong char only)
  - Added `typo_without_correction_profiled`: types wrong char and moves on without fixing
- **`src/utils/profile/presets.rs`**: Lowered `typo_rate` base values from 2-9% to 1.5-2.5% for ~1-2 typos per 100 chars
- **`src/utils/twitter/twitteractivity_interact.rs`**: Added 3-attempt retry loop for reply button detection + `scrollIntoView` before clicking

### LLM response parsing fix — markdown code block + indented content: lines ✅
- **Root cause**: LLM returned JSON wrapped in ````json...````, but `validate_reply` only checked for `[` or `{` as first char — the backticks blocked all parsing. Additionally, indented `    content:` lines weren't matched by `strip_prefix("content:")`.
- **`src/utils/twitter/twitteractivity_llm_validation.rs`**:
  - Added `strip_code_block()` helper: strips ```...``` markdown fence (with optional language tag, handles whitespace, empty content, missing closing fence)
  - Called at start of `validate_reply()` before all other parsing
  - Fixed whitespace-indented prefix matching: `l.strip_prefix("content:")` → `l.trim().strip_prefix("content:")`
  - Added bounds check: `if closing <= content_begin { return None; }` — prevents panic on malformed fences like ```` `` ````

### Test expansion — +22 tests across 3 modules ✅
- `keyboard.rs`: 3 new proptests (self-inverse property, unmapped identity property)
- `llm_validation.rs`: 12 new edge case tests (strip_code_block: trailing content, no newline, lone fence, empty fences, multiple blocks, only backticks; validate_reply: empty content value, banned words after strip, code block+JSON combo, whitespace only, empty content lines, normal text pass-through)
- `interact.rs`: 7 new parse_button_coordinates edge cases (negative, zero, NaN/null, large values, extra fields)
- **Total**: 39 keyboard + 46 validation + 51 interact = 136 tests, all passing ✅

## 27-06-26

### Spec 0031: Scroll tweet into viewport and retweet confirmation fallback ✅
- **`src/utils/twitter/engagement/dispatch.rs`**: Feed-level actions (like, retweet, follow, bookmark) now scroll the target tweet element to the center of the viewport before acting.
- **`src/utils/twitter/twitteractivity_actions.rs`**: Added selector-based fallback click using `RETWEET_CONFIRM_SELECTOR` in `retweet_at_position` if coordinate-based click fails to open the confirm dropdown.
- **`src/utils/twitter/js/js_scroll_and_get_tweet_button.js`**: Extracted JS logic to handle the scrolling and coordinate retrieval of the tweet button.
- **Verification**: `.\check.ps1` runs clean, all tests passing. Spec archived to `docs/specs/_done/0031-scroll-tweet-into-viewport`.

### Spec 0032: Expose task staggering delay as TASK_STAGGER_DELAY_MS environment override ✅
- **`src/config/env.rs`**: Added logic to load `TASK_STAGGER_DELAY_MS` environment variable to override `config.orchestrator.task_stagger_delay_ms`. Includes graceful fallback if env var is empty or invalid.
- **`src/config/tests.rs`**: Added unit tests validating env load behavior and graceful fallbacks.
- **Verification**: `.\check.ps1` runs clean. Spec archived to `docs/specs/_done/0032-configurable-stagger-delay`.

### Checklist Updates ✅
- Marked **Configurable Staggering** as complete in `CODEBASE_IMPROVEMENT_CHECKLIST.md`.

### Spec 0033: Segment Twitter persistence state files by browser profile name ✅
- **`src/utils/twitter/twitteractivity_persistence.rs`**: Updated `load()`, `save()`, and `update_async()` to require and utilize the browser profile name, constructing separate persistence files like `twitter-state-<profile_name>.json` and lock files like `twitter-state-<profile_name>.json.lock`. Includes safe input sanitization.
- **`src/task/twitteractivity.rs`**: Extracted the browser profile name from `api.behavior_profile().name` and passed it to the persistence layer.
- **Tests**: Added 2 new integration tests verifying state file roundtripping and concurrent updates.
- **Verification**: `.\check.ps1` runs clean (4,175/4,175 tests pass). Spec archived to `docs/specs/_done/0033-segmented-twitter-persistence`.
- **Checklist**: Marked **Database Segmentation / Migration** as complete in `CODEBASE_IMPROVEMENT_CHECKLIST.md`.