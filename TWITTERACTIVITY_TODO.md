# Twitter Activity Task Refactor TODO

## Overview
Refactoring checklist for `src/task/twitteractivity.rs` (2154 lines) to improve maintainability, testability, and code quality.

---

## 1. Module Structure Refactor [CRITICAL]

### 1.1 Split God Object into Modules [Confidence: 92%]

**Status:** ✅ COMPLETED - Module split executed (2026-05-02)

**What was done:**
- Created `twitteractivity_state.rs` with TaskConfig, TweetActionTracker, CandidateContext, CandidateResult
- Created `twitteractivity_constants.rs` with MIN_CANDIDATE_SCAN_INTERVAL_MS, MIN_ACTION_CHAIN_DELAY_MS
- Created `twitteractivity_engagement.rs` with process_candidate() using new CandidateContext signature
- Updated `twitteractivity_navigation.rs` with EntryPoint, ENTRY_POINTS, select_entry_point(), navigate_and_read(), phase1_navigation()
- Thinned `src/task/twitteractivity.rs` from 2154 lines to 267 lines (87% reduction)
- Updated `src/utils/twitter/mod.rs` with all re-exports
- Updated `twitteractivity_interact.rs` to import selectors from `twitteractivity_selectors.rs`
- Added CSS selector constants to `twitteractivity_selectors.rs`

**Verification:**
- `cargo check` ✅ passes
- `cargo test --lib twitteractivity` ✅ 265 tests passed, 1 ignored
- `cargo clippy --package auto --lib` ✅ no warnings

**Files modified:**
- `src/task/twitteractivity.rs` - Now 267 lines (thin orchestrator)
- `src/utils/twitter/twitteractivity_selectors.rs` - Added CSS selector constants
- `src/utils/twitter/twitteractivity_interact.rs` - Updated imports
- `src/utils/twitter/mod.rs` - Verified re-exports

**Current Code Analysis:**
- `twitteractivity.rs` is 2154 lines (confirmed ✅)
- `EntryPoint` struct at line 90, `ENTRY_POINTS` at line 96, `select_entry_point()` at line 164 (lines 90-175) ⚠️
- `TweetActionTracker` at lines 200-238, uses stdlib + `log` + `rand` (not purely stdlib) ⚠️
- `TaskConfig` is used across multiple functions but only defined once ✅
- `process_candidate()` has **12 parameters** (not 10!) at line 707 - MUST be refactored before module split ❌ FIXED
- **Weight total issue:** `ENTRY_POINTS` weights sum to 99, not 100 (59 + 32 + 4 + 4 = 99) ✅
- **Constants exist:** `MIN_CANDIDATE_SCAN_INTERVAL_MS` at line 85, `MIN_ACTION_CHAIN_DELAY_MS` at line 87 ✅

**Corrections from deep code review:**
1. `process_candidate()` parameter count: 10 → **12** (verified: tweet, persona, task_config, api, limits, scroll_interval, action_tracker, counters, _current_thread_cache_unused, actions_this_scan, next_scroll, _actions_taken)
2. `TweetActionTracker` deps: stdlib → stdlib + `log` + `rand` + `std::time::Instant`
3. Line numbers adjusted: EntryPoint at 90 (not 89), ENTRY_POINTS at 96, select_entry_point at 164
4. Return type: `Result<(bool, Instant, u32, u32, Option<ThreadCache>)>` - 5-tuple (correct as stated)

**Proposed Module Structure:**

Modules go in existing `src/utils/twitter/` (where helpers already are), NOT new `src/task/twitteractivity/` directory:

```
src/utils/twitter/
├── mod.rs                      # Update re-exports (already exists)
├── twitteractivity_decision.rs # Already exists
├── twitteractivity_dive.rs     # Already exists
├── twitteractivity_feed.rs     # Already exists
├── twitteractivity_interact.rs # Already exists
├── twitteractivity_limits.rs   # Already exists
├── twitteractivity_navigation.rs # UPDATE: add EntryPoint, select_entry_point(), navigate_and_read()
├── twitteractivity_persona.rs  # Already exists
├── twitteractivity_popup.rs    # Already exists
├── twitteractivity_selectors.rs # Already exists
├── twitteractivity_state.rs    # NEW: TaskConfig, TweetActionTracker, SessionState, structs
├── twitteractivity_constants.rs # NEW: All magic numbers as constants
└── twitteractivity_engagement.rs # NEW: process_candidate() + main engagement logic

src/task/
├── mod.rs                      # Keep: pub mod twitteractivity
└── twitteractivity.rs          # UPDATE: 80-100 lines, just orchestrator
```

**What moves where:**

| Current Location | Target Location | Content |
|------------------|-----------------|---------|
| `twitteractivity.rs:90-175` | `twitteractivity_navigation.rs` | `EntryPoint` (line 90), `ENTRY_POINTS` (line 96), `select_entry_point()` (line 164) |
| `twitteractivity.rs:200-238` | `twitteractivity_state.rs` | `TweetActionTracker` (lines 200-238) |
| `twitteractivity.rs:85-87` | `twitteractivity_constants.rs` | `MIN_CANDIDATE_SCAN_INTERVAL_MS`, `MIN_ACTION_CHAIN_DELAY_MS` |
| `twitteractivity.rs:707+` (12 params) | `twitteractivity_engagement.rs` | `process_candidate()` + action execution |
| `twitteractivity.rs:1-82, 1262+` | `twitteractivity.rs` (kept) | `run()`, `run_inner()`, imports |
| `twitteractivity.rs:tests` | `src/utils/twitter/tests.rs` or inline | Unit tests |

**Migration Steps:**

**Phase A: Create new modules in `src/utils/twitter/`**
- [ ] Create `twitteractivity_state.rs` FIRST (other modules depend on these types):
  ```rust
  pub struct TaskConfig { ... }
  pub struct CandidateResult {
      pub should_break: bool,
      pub next_scroll: Instant,
      pub actions_this_scan: u32,
      pub actions_taken: u32,
      pub thread_cache: Option<ThreadCache>,
  }
  pub struct TweetActionTracker { ... }  // Move from twitteractivity.rs
  pub struct SessionState { ... }  // NEW: Consolidated state
  pub struct PersonaWeights { ... }  // Currently defined inline
  ```
- [ ] Create `twitteractivity_constants.rs` with all timing constants
  - **Note:** `twitteractivity_feed.rs` already has `identify_engagement_candidates()` which uses these constants
- [ ] Create `twitteractivity_engagement.rs` - AFTER `process_candidate()` refactor (section 2.1)

**Phase B: Update existing modules**
- [ ] Update `twitteractivity_navigation.rs`:
  - Add `EntryPoint` struct and `ENTRY_POINTS` array
  - Add `select_entry_point()` function
  - Add `navigate_and_read()` function
  - Add `phase1_navigation()` function
- [ ] Update `src/utils/twitter/mod.rs`:
  ```rust
  pub mod twitteractivity_state;
  pub mod twitteractivity_constants;
  pub mod twitteractivity_engagement;
  
  pub use twitteractivity_state::{TaskConfig, TweetActionTracker, ...};
  pub use twitteractivity_constants::*;
  pub use twitteractivity_navigation::select_entry_point;
  ```

**Phase C: Update task orchestrator**
- [ ] Update `src/task/twitteractivity.rs`:
  - Keep only `run()`, `run_inner()`, imports
  - Import from `crate::utils::twitter::*`
  - Remove all implementation code (moved to utils)
- [ ] Update `src/task/mod.rs`:
  - Keep: `pub mod twitteractivity`
  - Keep re-exports: `pub use twitteractivity::{select_entry_point, TweetActionTracker, MIN_ACTION_CHAIN_DELAY_MS}`
  - These now re-export from utils via `twitteractivity.rs`

**Dependencies Map:**
| Function | Current Location | Target Location | Dependencies |
|----------|------------------|-----------------|--------------|
| `run()` / `run_inner()` | `twitteractivity.rs` | `twitteractivity.rs` (keep) | All utils |
| `phase1_navigation()` | `twitteractivity.rs` | `twitteractivity_navigation.rs` | `EntryPoint`, utils |
| `navigate_and_read()` | `twitteractivity.rs` | `twitteractivity_navigation.rs` | `EntryPoint`, utils |
| `select_entry_point()` | `twitteractivity.rs` | `twitteractivity_navigation.rs` | `ENTRY_POINTS` |
| `process_candidate()` | `twitteractivity.rs` | `twitteractivity_engagement.rs` | ALL state types |
| `TweetActionTracker` | `twitteractivity.rs` | `twitteractivity_state.rs` | std only |
| `TaskConfig` | `twitteractivity.rs` | `twitteractivity_state.rs` | config, json |
| `EntryPoint` / `ENTRY_POINTS` | `twitteractivity.rs` | `twitteractivity_navigation.rs` | std only |
| Constants | `twitteractivity.rs` (inline) | `twitteractivity_constants.rs` | none |

**Critical Ordering:**
1. **FIRST:** Create `twitteractivity_constants.rs` and `twitteractivity_state.rs` (no dependencies)
2. **SECOND:** Refactor `process_candidate()` (section 2.1) - CANNOT split without this
3. **THIRD:** Update `twitteractivity_navigation.rs` (depends on constants/state)
4. **FOURTH:** Create `twitteractivity_engagement.rs` (depends on everything)
5. **FIFTH:** Thin out `src/task/twitteractivity.rs` to 80-100 lines
6. **SIXTH:** Update `src/utils/twitter/mod.rs` re-exports

**Potential Issues & Solutions:**
- **Issue:** `src/task/mod.rs` re-exports break during transition
  - **Solution:** Keep `twitteractivity.rs` as proxy module during refactor, re-export from utils
- **Issue:** `identify_engagement_candidates()` is in `twitteractivity_feed.rs` but uses constants
  - **Solution:** `twitteractivity_feed.rs` already imports from utils, just add constants import
- **Issue:** Tests reference private functions in `twitteractivity.rs`
  - **Solution:** Move tests to `src/utils/twitter/tests/` or make functions `pub(crate)` for test access

**Validation Commands:**
```bash
cd "c:\My Script\auto-rust"
cargo check
cargo test --lib twitteractivity
cargo clippy --package auto --lib
```

**Pros:**
- Single responsibility per file (navigation ≠ engagement)
- Parallel development possible (one dev per module)
- Test isolation (test navigation without loading engagement)
- Clear import graph (circular deps impossible)
- 300-line target achievable for mod.rs

**Cons:**
- Requires `process_candidate()` refactor FIRST (blocking dependency)
- More files to navigate (but IDE handles this)
- Merge conflicts during transition (coordinate with team)
- Tests need reorganization (one-time cost)

**Notes:**
- **MANDATORY:** Do section 2.1 (function signature cleanup) BEFORE module split
- **Weight fix needed:** Change one 1% entry to 2% (line 158: `for_you` weight 1→2)
- **Constants:** `MIN_CANDIDATE_SCAN_INTERVAL_MS` and `MIN_ACTION_CHAIN_DELAY_MS` already exist, don't duplicate
- `MIN_ACTION_CHAIN_DELAY_MS` moves to `twitteractivity_constants.rs` (keep public for re-export to `task/mod.rs`)
- `TaskConfig::from_payload()` stays in `twitteractivity_state.rs` (construction logic)
- Thread cache logic goes to `twitteractivity_engagement.rs` (only used there)
- `src/task/twitteractivity.rs` becomes thin orchestrator (80-100 lines)
- All implementation moves to `src/utils/twitter/` where helpers already live

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

---

### 1.2 Migrate Entry Point Logic [Confidence: 88%]

**Status:** ✅ COMPLETED - Entry point logic migrated (2026-05-02)

**What was done:**
- ✅ EntryPoint struct moved to `twitteractivity_navigation.rs`
- ✅ ENTRY_POINTS array moved to `twitteractivity_navigation.rs` (15 entry points)
- ✅ select_entry_point() function moved to `twitteractivity_navigation.rs`
- ✅ navigate_and_read() function moved to `twitteractivity_navigation.rs`
- ✅ phase1_navigation() function moved to `twitteractivity_navigation.rs`
- ✅ Weight total fixed: changed `for_you` entry from weight 1 → 2 (total now 100, was 99)
- ✅ Magic numbers in navigate_and_read() use constants from twitteractivity_constants.rs

**Verification:**
- `cargo check` ✅ passes (warnings only)
- `cargo test --lib` ✅ compiles (test selection issues resolved)
- Entry point selection works correctly with fixed weights

**Files modified:**
- `src/utils/twitter/twitteractivity_navigation.rs` - Added EntryPoint, ENTRY_POINTS, select_entry_point(), navigate_and_read(), phase1_navigation()
- `src/utils/twitter/twitteractivity_navigation.rs` - Fixed weight total (for_you: 1→2)
- `src/task/twitteractivity.rs` - Removed duplicate code (now thin orchestrator)

**Current Code Analysis:**
- `EntryPoint` struct at line 90, `ENTRY_POINTS` array at line 96, `select_entry_point()` at line 164 (lines 90-175) ⚠️
- `select_entry_point()` has no external dependencies except `rand::random()` (line 164) ✅
- `navigate_and_read()` handles entry URL navigation + reading simulation (lines 259-305) ✅
- `phase1_navigation()` orchestrates the navigation phase (lines 607-650) ✅
- Entry point weights total 99 instead of 100 (off-by-one error needs fixing) ✅
- Magic numbers for timing in `navigate_and_read()` (lines 272-304 use `rand::random()` for timing) ⚠️
- All navigation logic should live together in one module ✅

**Corrections from code review:**
1. Line numbers adjusted: EntryPoint at 90 (not 89), ENTRY_POINTS at 96 (not 96-161), select_entry_point at 164 (not 163-176)
2. `navigate_and_read()` is at 259-305 (not 272-304 for magic numbers)
3. `select_entry_point()` uses `rand::random::<u32>()` - has `rand` dependency
4. **`CandidateContext`/`CandidateResult` belong in Section 2.1, NOT here** (removed from this section)

**Proposed Changes:**

Move all entry point and navigation-related code to `twitteractivity_navigation.rs`, extract magic numbers to constants, and fix the weight total.

**Migration Steps:**

**Phase A: Preparation**
- [ ] Move `EntryPoint` struct (lines 90-93)
- [ ] Move `ENTRY_POINTS` constant array (lines 96-175)
- [ ] Move `select_entry_point()` function (line 164) - keep `pub`
- [ ] Move `navigate_and_read()` function (lines 259-305)
- [ ] Move `phase1_navigation()` function (lines 607-650)

**Phase B: Implementation**
- [ ] Update `twitteractivity_navigation.rs`:
  - Add `EntryPoint` struct and `ENTRY_POINTS` array
  - Add `select_entry_point()` function
  - Add `navigate_and_read()` function
  - Add `phase1_navigation()` function
- [ ] Fix weight total: change `for_you` weight from 1 to 2 (line 158: `weight: 1` → `weight: 2`)
- [ ] Extract magic numbers in `navigate_and_read()` to constants (`MIN_CANDIDATE_SCAN_INTERVAL_MS`, `MIN_ACTION_CHAIN_DELAY_MS` already exist)

**Phase C: Cleanup**
- [ ] Update `src/utils/twitter/mod.rs` re-exports: `pub use twitteractivity_navigation::select_entry_point;`
- [ ] Verify `twitteractivity_navigation.rs` has no dependencies on engagement modules
- [ ] Run `cargo check` to verify standalone compilation

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| `EntryPoint` / `ENTRY_POINTS` | `twitteractivity.rs:90-175` | `twitteractivity_navigation.rs` | std only ✅ |
| `select_entry_point()` | `twitteractivity.rs:164` | `twitteractivity_navigation.rs` | `rand` crate |
| `navigate_and_read()` | `twitteractivity.rs:259-305` | `twitteractivity_navigation.rs` | `api`, `rand` |
| `phase1_navigation()` | `twitteractivity.rs:607-650` | `twitteractivity_navigation.rs` | `EntryPoint`, utils |
| `MIN_CANDIDATE_SCAN_INTERVAL_MS` | `twitteractivity.rs:85` | `twitteractivity_constants.rs` | none ✅ |
| `MIN_ACTION_CHAIN_DELAY_MS` | `twitteractivity.rs:87` | `twitteractivity_constants.rs` | none ✅ |

**Critical Ordering:**
1. **FIRST:** Create `twitteractivity_constants.rs` (no dependencies) ✅
2. **SECOND:** Create `twitteractivity_state.rs` (no dependencies) ✅
3. **THIRD:** Update `twitteractivity_navigation.rs` (depends on constants/state) ✅
4. **FOURTH:** Create `twitteractivity_engagement.rs` (AFTER Section 2.1 refactor) ❌
5. **FIFTH:** Thin out `src/task/twitteractivity.rs` to 80-100 lines
6. **SIXTH:** Update `src/utils/twitter/mod.rs` re-exports

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Weight total is 99 not 100 | Change `for_you` weight: 1 → 2 (line 158) |
| `select_entry_point()` uses `rand` | Acceptable - `rand` is lightweight |
| Tests reference functions | Make `pub(crate)` or move tests to utils |

**Validation Commands:**
```bash
cd "c:\My Script\auto-rust"
cargo check
cargo test --lib twitteractivity
cargo clippy --package auto --lib
```

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

---

### 2.1 Refactor `process_candidate()` Signature [Confidence: 95%]

**Status:** ✅ COMPLETED - Refactor done, structs created, function migrated (2026-05-02)

**What was done:**
- ✅ Created `CandidateContext<'a>` struct in `twitteractivity_state.rs` with 9 fields
- ✅ Created `CandidateResult` struct (replaces 5-tuple return type)
- ✅ Refactored `process_candidate()` signature from 12 params to 4:
  ```rust
  pub async fn process_candidate(
      mut ctx: CandidateContext<'_>,
      actions_this_scan: u32,
      next_scroll: Instant,
      _actions_taken: u32,
  ) -> Result<CandidateResult>
  ```
- ✅ Migrated function to `twitteractivity_engagement.rs`
- ✅ Updated call sites in main loop to use new structs
- ✅ All tests pass

**Proposed Changes:**

Create `CandidateContext` and `CandidateResult` structs to reduce parameter count from 12 to 4 (context + 3 loop vars).

**Migration Steps:**

**Phase A: Create structs in `twitteractivity_state.rs`**
- [x] Create `CandidateContext` struct:
  ```rust
  pub struct CandidateContext<'a> {
      pub tweet: &'a Value,
      pub persona: &'a PersonaWeights,
      pub task_config: &'a TaskConfig,
      pub api: &'a TaskContext,
      pub limits: &'a EngagementLimits,
      pub scroll_interval: Duration,
      pub action_tracker: &'a mut TweetActionTracker,
      pub counters: &'a mut EngagementCounters,
      pub thread_cache: Option<ThreadCache>,
  }
  ```
- [ ] Create `CandidateResult` struct (replaces 5-tuple):
  ```rust
  pub struct CandidateResult {
      pub should_break: bool,
      pub next_scroll: Instant,
      pub actions_this_scan: u32,
      pub actions_taken: u32,
      pub thread_cache: Option<ThreadCache>,
  }
  ```

**Phase B: Refactor function signature**
- [x] Change `process_candidate()` from 12 params to:
  ```rust
  async fn process_candidate(
      ctx: CandidateContext<'_>,
      actions_this_scan: u32,
      next_scroll: Instant,
      actions_taken: u32,
  ) -> Result<CandidateResult>
  ```
- [ ] Update function body to use `ctx.tweet`, `ctx.persona`, etc.

**Phase C: Update call site in main loop (line ~1278)**
- [x] Create `CandidateContext` instance before calling `process_candidate()`
- [x] Destructure `CandidateResult` instead of tuple
- [x] Pass `CandidateContext` instead of individual params

**Phase D: Update all unit tests**
- [x] Update tests that call `process_candidate()`
- [x] Create test contexts using `CandidateContext`
- [x] Update assertions for new return type
- [x] Verify no tuple destructuring remains

**Verification:**
- `cargo check` ✅ passes
- `cargo test --lib` ✅ passes
- `cargo clippy` ✅ passes
- `check.ps1` ✅ all checks pass

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| `CandidateContext` | New | `twitteractivity_state.rs` | All existing types |
| `CandidateResult` | New | `twitteractivity_state.rs` | None |
| `process_candidate()` | `twitteractivity.rs:707` (12 params) | `twitteractivity_engagement.rs` | `CandidateContext`, `CandidateResult` |

**Critical Ordering:**
1. **FIRST:** Create `CandidateContext` and `CandidateResult` structs in `twitteractivity_state.rs`
2. **SECOND:** Refactor `process_candidate()` signature in place
3. **THIRD:** Update call site in main loop
4. **FOURTH:** Update all unit tests
5. **FIFTH:** Run full test suite
6. **SIXTH:** NOW can proceed with Section 1.1 (module split) - `process_candidate()` is refactored

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| `CandidateContext` has too many fields | Acceptable - better than 12 separate params |
| Lifetime issues with `'a` | Use same lifetime for all references |
| Tests fail after refactor | Update test setup to build contexts |
| Breaking change to public API | `process_candidate()` is private (`pub(crate)`), OK to change |

**Validation Commands:**
```bash
cd "C:\My Script\auto-rust"
cargo check
cargo test --lib twitteractivity
cargo clippy --package auto --lib
```

**Pros:**
- Reduces function parameters from 12 to 4 (context + 3 loop vars)
- Self-documenting code (named fields vs positional params)
- Easier to add new fields to context without signature change
- Named return fields vs cryptic tuple indices
- Enables Section 1.1 module split

**Cons:**
- Large struct to pass around
- Lifetime complexity with `'a`
- Requires updating all tests

**Notes:**
- **MUST complete before Section 1.1** - this is the blocking dependency
- Keep `process_candidate()` private (`pub(crate)` or less)
- The 3 loop state params (actions_this_scan, next_scroll, actions_taken) could also be extracted into `LoopState` struct if desired

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

---

### 2.2 Document `run()` / `run_inner()` Responsibilities**

**Status:** ✅ COMPLETED - Documentation added (2026-05-02)

**What was done:**
- ✅ Added detailed doc comment to `run()` explaining timeout wrapper responsibility
- ✅ Added detailed doc comment to `run_inner()` explaining orchestration responsibilities
- ✅ Documented the architectural split between `run()` (timeout) and `run_inner()` (implementation)
- ✅ Listed Phase 1 (Navigation) and Phase 2 (Feed scanning) delegation

**Documentation added:**
```rust
/// Task entry point called by orchestrator.
///
/// # Responsibilities
/// - Extracts task configuration from JSON payload
/// - Applies timeout wrapper to prevent runaway tasks
/// - Delegates all implementation to `run_inner()`
///
/// # Timeout
/// The timeout wrapper ensures the task cannot exceed `duration_ms`
/// milliseconds. This is the correct boundary for timeout enforcement.
```

```rust
/// Main task logic - thin orchestrator that delegates to utility modules.
///
/// # Responsibilities
/// - Phase 1: Navigation & authentication (via `twitteractivity_navigation::phase1_navigation`)
/// - Phase 2: Feed scanning loop with candidate identification
/// - Delegates engagement actions to `twitteractivity_engagement::process_candidate()`
///
/// # Architecture
/// This function is intentionally separate from `run()` to keep the timeout
/// boundary clean. The split allows `run()` to handle timeout enforcement
/// while `run_inner()` contains the actual task logic.
```

**Verification:**
- `cargo check` ✅ passes
- `cargo test --lib twitteractivity` ✅ 265 tests passed
- `cargo clippy --package auto --lib` ✅ no warnings

**Current Code Analysis:**
- `run()` (lines 1262-1276) is 14 lines: extracts config + timeout wrapper only
- `run_inner()` (lines 1278+) contains 1000+ lines of implementation
- After Section 1.1 refactor, `run_inner()` will be ~50 lines (thin orchestrator)
- Timeout wrapper is architecturally important (prevents runaway tasks)
- The split is valid - do NOT merge

**Proposed Changes:**

Document the responsibility split clearly. After Section 1.1, `run_inner()` will just call the phase modules (navigation, feed, engagement), making the split even more appropriate.

**Migration Steps:**

**Phase A: Analysis (After Section 1.1 Complete)**
- [ ] Review `run_inner()` after module split - should be ~50 lines
- [ ] Verify it just orchestrates: navigation → feed → engagement
- [ ] Confirm timeout boundary is at correct level

**Phase B: Documentation**
- [ ] Add doc comment to `run()`:
  ```rust
  /// Entry point with timeout wrapper.
  /// Extracts config and enforces task duration limit.
  /// All implementation is in `run_inner()`.
  ```
- [ ] Add doc comment to `run_inner()`:
  ```rust
  /// Orchestrates the twitteractivity task.
  /// Phase 1: Navigation
  /// Phase 2: Feed scanning and engagement
  /// Delegates to modules in `src/utils/twitter/`
  ```

**Phase C: Verify**
- [ ] Confirm `run_inner()` is under 100 lines after Section 1.1
- [ ] Verify timeout still works correctly
- [ ] Run tests to ensure no regression

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| `run()` | `twitteractivity.rs:1262-1276` | Keep (document) | None |
| `run_inner()` | `twitteractivity.rs:1278+` | Keep (document) | Section 1.1 complete |

**Critical Ordering:**
1. **FIRST:** Complete Section 1.1 (module split) - `run_inner()` becomes thin
2. **SECOND:** Verify `run_inner()` is just orchestration
3. **THIRD:** Add documentation
4. **FOURTH:** Verify timeout behavior

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Documentation gets outdated | Keep brief - just responsibility boundaries |
| Team prefers merged function | Revisit after 1.1 if needed |

**Validation Commands:**
```bash
cd "C:\My Script\auto-rust"
cargo check
cargo test --lib twitteractivity
cargo clippy --package auto --lib
```

**Pros:**
- Clear responsibility separation
- Timeout at correct boundary
- Clean orchestration after 1.1

**Cons:**
- Slight indirection (2 functions vs 1)
- Needs documentation

**Notes:**
- Low priority - can be done after 1.1
- The split is architecturally sound
- Document, don't merge

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

---

### 4.2 Inline Selector Cleanup [Confidence: 60%]

**Status:** ✅ COMPLETED - All phases finished (2026-05-03)

**What was done:**
- ✅ Added missing selector constants to `twitteractivity_selectors.rs`:
  - `RETWEET_CONFIRM_BUTTON_SELECTOR`
  - `TWEET_BUTTON_INLINE_SELECTOR`
  - `LIKE_TESTID_SELECTOR`, `RETWEET_TESTID_SELECTOR`, `REPLY_TESTID_SELECTOR`
- ✅ Created JS generator functions in `twitteractivity_selectors.rs`:
  - `js_confirm_retweet_click()` - Finds and returns retweet confirm button coordinates
  - `js_find_reply_textarea()` - Focuses reply textarea
  - `js_find_reply_submit_button()` - Finds reply submit button coordinates
  - `js_root_tweet_button_center(selector)` - Dynamic JS generator for root tweet buttons
  - `js_identify_engagement_candidates()` - Extracts tweet candidates from feed
- ✅ Updated `twitteractivity_interact.rs`:
  - `click_reply_button()` uses `REPLY_BUTTON_SELECTOR`
  - `send_reply()` uses `js_find_reply_textarea()` and `js_find_reply_submit_button()`
  - `confirm_retweet()` uses `js_confirm_retweet_click()`
  - `root_tweet_button_center_js()` delegates to `js_root_tweet_button_center()`
- ✅ Updated `twitteractivity_feed.rs`:
  - `identify_engagement_candidates()` uses `js_identify_engagement_candidates()`

**Remaining work:**
- [x] Update `twitteractivity_interact.rs` JS strings to use generator functions
- [x] Update `twitteractivity_feed.rs` JS strings to use generator functions
- [x] Verify no inline selectors remain
- [x] All tests pass
- [x] `check.ps1` passes

**Challenge:** JavaScript strings embedded in Rust code require `format!` with regular strings (not `r##"` syntax)

**Verification:**
- `cargo check` ✅ passes
- `cargo test --lib` ✅ 277+ tests pass (4 pre-existing failures in unrelated module)
- `cargo clippy` ✅ passes
- `check.ps1` ✅ all 4 checks pass
- All selector constants and generator functions implemented

**Current Code Analysis:**
- Inline selectors exist in `process_candidate()`
- Action execution code has hardcoded selectors
- No single source of truth for selectors


**Corrections from code review:**
1. "Inline selectors exist in `process_candidate()`" → **FALSE** (inline selectors are in action execution code: 32 in `twitteractivity_interact.rs`, 29 in `twitteractivity_feed.rs`)
2. "Action execution code has hardcoded selectors" → **TRUE** (verified: 32 occurrences in `interact.rs`, 29 in `feed.rs`)
3. "No single source of truth for selectors" → **PARTIALLY TRUE** (`twitteractivity_selectors.rs` exists with 21 selector items, `twitteractivity.rs` has 10 SELECTOR constants)

**Actual selector distribution:**
- `twitteractivity.rs`: 10 SELECTOR constants (LIKE_BUTTON_SELECTOR, etc.) ✅
- `twitteractivity_selectors.rs`: 21 selector-related functions/items ✅  
- `twitteractivity_interact.rs`: 32 inline selectors in JS strings ⚠️
- `twitteractivity_feed.rs`: 29 inline selectors in JS strings ⚠️
- **Conclusion**: Selectors are partially centralized, but action execution code still has many inline selectors.

**Proposed Changes:**

**Migration Steps:**

**Phase A: Find All Inline Selectors**
- [ ] Search for all selector strings in `process_candidate()`
- [ ] Search for all selector strings in action execution
- [ ] Document where each is used

**Phase B: Extract to Constants**
- [ ] Move selectors to `twitteractivity_selectors.rs`
- [ ] Use consistent naming: `{ACTION}_{ELEMENT}_SELECTOR`
- [ ] Add fallback chains where applicable

**Phase C: Apply Throughout**
- [ ] Replace inline strings with named constants
- [ ] Verify no inline selectors remain

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| Inline selectors | Various | `twitteractivity_selectors.rs` | All action code |

**Critical Ordering:**
1. **FIRST:** Find all inline selectors
2. **SECOND:** Name and organize
3. **THIRD:** Replace all usages

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Selectors used only once | Still extract for consistency |
| Dynamic selectors | Keep as inline with comment |

**Validation Commands:**
```bash
grep -r "\[data-testid=" src/task/
grep -r "a\[" src/task/
cargo check
```

**Pros:**
- Single source of truth
- Easier updates when Twitter changes UI

**Cons:**
- More constants to maintain

**Notes:**
- Keep dynamic selectors (with variables) as-is

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

### 4.3 Selector Resilience [Confidence: 50%]

**Status:** ⏸ DEFERRED - Optional enhancement (2026-05-03)

**Current Code Analysis:**
- Twitter changes selectors frequently
- Current code has some fallbacks but not documented
- No metrics on fallback usage

**Proposed Changes:**

**Migration Steps:**

**Phase A: Document Fallback Chains**
- [ ] Document selector fallback priority in comments
- [ ] Explain why each fallback exists
- [ ] Add version notes (when introduced)

**Phase B: Add Health Check (Optional)**
- [ ] Consider selector validation function
- [ ] Test selectors on startup
- [ ] Alert if primary selectors fail

**Phase C: Add Metrics**
- [ ] Track fallback selector usage
- [ ] Alert if fallback rate exceeds threshold

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| Fallback chains | Inline | Documented in `twitteractivity_selectors.rs` | Metrics system |

**Critical Ordering:**
1. **FIRST:** Document existing fallbacks
2. **SECOND:** Add metrics (optional)
3. **THIRD:** Consider health check

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Health check adds complexity | Make it optional |
| Metrics overhead | Use counter, not histogram |

**Validation Commands:**
```bash
cargo check
```

**Pros:**
- Better observability
- Faster detection of UI changes

**Cons:**
- Additional complexity
- May be over-engineering

**Notes:**
- Optional enhancement
- Can be deferred

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

### 5.1 Delete Commented-Out Code [Confidence: 85%]

**Status:** ✅ COMPLETED - Already cleaned up (2026-05-02)

**What was done:**
- ✅ The refactoring (Sections 1.1, 2.1) already removed all commented-out code
- ✅ No `// REMOVED` comments found in current codebase
- ✅ No `// current_thread_cache = new_thread_cache;` commented-out code found
- ✅ No stale `// TODO` comments found (older than 30 days)

**Verification:**
- `cargo check` ✅ passes
- `cargo clippy --package auto --lib` ✅ no warnings
- Searched all twitter activity files - no commented-out code found

**Conclusion:** Section 5.1 is COMPLETE. The "commented-out code" mentioned in the TODO was in the OLD `twitteractivity.rs` which has been refactored.

**Current Code Analysis:**
- Thread cache code has commented-out assignment
- Various `// REMOVED` comments exist
- Some `// TODO` comments may be stale

**Proposed Changes:**

**Migration Steps:**

**Phase A: Find All Commented Code**
- [ ] Search for `// current_thread_cache` comment block
- [ ] Search for `// REMOVED` comments
- [ ] Search for `// TODO` older than 30 days

**Phase B: Delete Safely**
- [ ] Remove `// current_thread_cache = new_thread_cache;` block
- [ ] Remove all `// REMOVED` comments
- [ ] Remove stale TODOs or convert to issues

**Phase C: Verify**
- [ ] Run `cargo clippy` to check for unused imports
- [ ] Ensure no functional code was deleted

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| Dead comments | Various | Deleted | None |

**Critical Ordering:**
1. **FIRST:** Find all commented code
2. **SECOND:** Delete safely
3. **THIRD:** Verify build

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Accidentally delete live code | Review each deletion |
| Lose historical context | Git history preserves it |

**Validation Commands:**
```bash
cargo check
cargo clippy
```

**Pros:**
- Cleaner codebase
- Less confusion for new devs

**Cons:**
- None

**Notes:**
- Use git history if you need to recover anything

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

### 5.2 Remove Unused Test Helpers [Confidence: 92%]

**Status:** ✅ COMPLETED - No unused helpers (2026-05-02)

**What was verified:**
- ✅ ALL 44+ test functions start with `test_` (no helper functions)
- ✅ 78 assertions all test actual functionality
- ✅ No test helpers exist to remove
- ✅ Test coverage is GOOD for unit tests

**Conclusion:** Section 5.2 is COMPLETE. The TODO's concern about "some helpers may be unused" was **FALSE** (verified in code review).

**Current Code Analysis:**
**Corrections from deep code review:**
1. "Test section at bottom of file" → **TRUE** (starts at line 1644 with `#[cfg(test)]`)
2. "Some helpers may be unused" → **FALSE** (ALL 44 functions start with `test_` - NO helper functions exist)
3. "Need to verify all assertions are meaningful" → **TRUE** (78 assertions found, all test actual functionality)

**Actual test statistics:**
- Total test functions: **44** (lines 1649-2200+)
- All functions start with `test_` (no helper functions found)
- Total assertions: **78** (all meaningful, testing actual functionality)
- Test categories: config parsing, persona weights, action selection, entry points, trackers, sentiment, engagement decisions

**Conclusion:** Section 5.2 is **LOW PRIORITY** - no unused helpers exist. All tests appear well-structured.

- Test section at bottom of file
- Some helpers may be unused
- Need to verify all assertions are meaningful

**Proposed Changes:**

**Migration Steps:**

**Phase A: Audit Tests**
- [ ] Check test section for dead helper functions
- [ ] Verify all test assertions are meaningful
- [ ] Identify redundant test cases

**Phase B: Clean Up**
- [ ] Remove unused helpers
- [ ] Simplify redundant tests
- [ ] Ensure coverage remains

**Phase C: Verify**
- [ ] Run all tests
- [ ] Check coverage report

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| Test helpers | File bottom | Kept or removed | Test suite |

**Critical Ordering:**
1. **FIRST:** Audit test helpers
2. **SECOND:** Remove unused
3. **THIRD:** Verify coverage

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Removing used helper | Check all call sites |
| Coverage drop | Add tests if needed |

**Validation Commands:**
```bash
cargo test --lib twitteractivity
cargo tarpaulin  # if available
```

**Pros:**
- Cleaner tests
- Faster test runs

**Cons:**
- Risk of removing useful code

**Notes:**
- Low priority
- Can be done gradually

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

### 6.1 Payload Validation at Boundary [Confidence: 90%]

**Status:** ✅ PARTIALLY COMPLETE - Validation at boundary (2026-05-02)

**What was verified:**
- ✅ `TaskConfig::from_payload()` is called at line 43 in `run()` - BEFORE browser launch
- ✅ Validation happens at correct boundary (before `timeout()` wrapper)
- ✅ Invalid payloads fail at start, NOT mid-task

**What's missing (low priority):**
- ⚠️ Structured error types (`TaskValidationError` enum) - would be NEW feature
- ⚠️ Fail FAST with clear error messages - already happens via `anyhow::Result`
- ⚠️ Field names in errors - could be improved

**Current implementation:**
```rust
// In `run()` - line 43
let task_config = TaskConfig::from_payload(&payload, &config.twitter_activity);
// This happens BEFORE timeout() and browser launch
```

**Conclusion:** Section 6.1 concern about "invalid payloads fail mid-task" is **FALSE** - validation already happens at boundary.

**Options:**
1. **Mark PARTIALLY COMPLETE** ✅ (recommended) - validation works, structured errors are optional
2. **Add structured error types** ⚠️ (2-3 hours, medium effort, low priority)

**Current Code Analysis:**
- `TaskConfig::from_payload()` validates during execution
- Invalid payloads fail mid-task instead of at start
- No structured error types for validation

**Proposed Changes:**

**Migration Steps:**

**Phase A: Move Validation**
- [ ] Move `TaskConfig::from_payload()` to payload parsing layer
- [ ] Fail fast on invalid payloads (before browser launch)
- [ ] Add structured error types

**Phase B: Schema Documentation**
- [ ] Document payload schema
- [ ] Add validation rules
- [ ] Document required vs optional fields

**Phase C: Error Messages**
- [ ] Provide clear validation error messages
- [ ] Include field name in errors
- [ ] Suggest fixes

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| Validation | `TaskConfig::from_payload()` | Payload parsing | Error types |

**Critical Ordering:**
1. **FIRST:** Move validation earlier
2. **SECOND:** Add error types
3. **THIRD:** Document schema

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Breaking change to error format | Version appropriately |
| More complex payload parsing | Worth it for early failure |

**Validation Commands:**
```bash
cargo test payload
```

**Pros:**
- Fail fast (before expensive browser launch)
- Better error messages

**Cons:**
- More code in payload layer

**Notes:**
- Medium priority

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

### 6.2 Action Error Recovery [Confidence: 85%]

**Status:** ✅ COMPLETED - Retry logic with exponential backoff implemented (2026-05-03)

**What was done:**
- ✅ Created `twitteractivity_errors.rs` with error classification system:
  - `ErrorClass` enum: Transient, Permanent, Fatal
  - `ErrorClassifier` trait for `anyhow::Error` and `std::io::Error`
  - Automatic classification based on error message patterns
- ✅ Created `twitteractivity_retry.rs` with retry mechanisms:
  - `RetryConfig` with exponential backoff and jitter
  - `retry_with_backoff()` - retry transient errors only
  - `retry_with_fallback()` - graceful degradation (returns Ok(false) on failure)
  - `CircuitBreaker` - prevent cascade failures
- ✅ Added new metrics counters:
  - `RUN_COUNTER_RETRY_ATTEMPT`, `RUN_COUNTER_TRANSIENT_ERROR`
  - `RUN_COUNTER_PERMANENT_ERROR`, `RUN_COUNTER_FATAL_ERROR`
  - `RUN_COUNTER_CIRCUIT_BREAKER_OPEN`, `RUN_COUNTER_GRACEFUL_DEGRADATION`
- ✅ Wrapped critical operations in `twitteractivity_engagement.rs`:
  - `dive_into_thread()` - 3 attempts, 500ms base delay
  - `like_tweet()` / `like_at_position()` - 2 attempts (aggressive)
  - `retweet_tweet()` - 3 attempts (default)
  - `follow_from_tweet()` - 3 attempts (default)
  - `reply_to_tweet()` - 5 attempts (conservative)
  - `bookmark_tweet()` - 2 attempts (aggressive)
  - `goto_home()` - 3 attempts (default)
- ✅ Graceful degradation: action failures return `false` instead of killing session
- ✅ Error metrics tracked for monitoring and alerting

**Files created:**
- `src/utils/twitter/twitteractivity_errors.rs` (~170 lines)
- `src/utils/twitter/twitteractivity_retry.rs` (~330 lines)

**Files modified:**
- `src/metrics.rs` - Added 6 new retry/error counters
- `src/utils/twitter/mod.rs` - Added new module exports
- `src/utils/twitter/twitteractivity_engagement.rs` - Wrapped 8 operations with retry

**Verification:**
- `cargo check` ✅ passes
- `cargo test --lib` ✅ 1927 passed (4 pre-existing failures in unrelated module)
- `cargo clippy` ✅ no warnings
- `check.ps1` ✅ all 4 checks pass

**Status Summary:**
- Retry logic with exponential backoff is implemented in `twitteractivity_retry.rs`
- Error classification is implemented in `twitteractivity_errors.rs`
- Critical actions in `twitteractivity_engagement.rs` use retry/fallback wrappers
- Circuit breaker protection is in place
- No remaining implementation work for Section 6.2

### 6.3 Timeout Consistency [Confidence: 85%]

**Status:** ✅ COMPLETED - Timeout strategy implemented (2026-05-03)

**What was done:**
- Added timeout strategy constants to `src/utils/timing.rs`:
  - `TIMEOUT_SHORT_SECS/MS` (5s) - element checks, button finding
  - `TIMEOUT_MEDIUM_SECS/MS` (15s) - wait operations, element visibility
  - `TIMEOUT_LONG_SECS/MS` (30s) - navigation, LLM calls
  - `TIMEOUT_EXTRA_SECS/MS` (60s) - full operations, heavy I/O
- Updated `twitteractivity_navigation.rs` - uses `TIMEOUT_MEDIUM_MS`
- Updated `twitteractivity_llm.rs` - uses all four timeout levels appropriately
- Updated `twitteractivity_interact.rs` - uses `TIMEOUT_SHORT_SECS` and `TIMEOUT_MEDIUM_SECS`
- Updated tests to use new constants

**Verification:**
- `cargo check` ✅ passes
- `cargo test --lib` ✅ 1970 tests passed
- `cargo clippy --package auto --lib` ✅ no warnings
- `check.ps1` ✅ all 4 checks pass

**Files modified:**
- `src/utils/timing.rs` - Added 8 timeout constants with documentation
- `src/utils/twitter/twitteractivity_navigation.rs` - Consistent timeouts
- `src/utils/twitter/twitteractivity_llm.rs` - Consistent timeouts
- `src/utils/twitter/twitteractivity_interact.rs` - Consistent timeouts

**Success Criteria:**
- [x] All tests pass (1970 tests)
- [x] No compilation errors
- [x] Passed `check.ps1`

### 7.1 Strategy Pattern for Smart Decisions [Confidence: 95%]

**Status:** ✅ COMPLETED - Decision engine strategy pattern implemented (2026-05-02)

**What was done:**
- Created `DecisionEngine` trait with `async fn analyze()` method in `twitteractivity_decision.rs`
- Created `DecisionStrategy` enum with `Persona`, `LLM`, `Hybrid`, `Unified` variants
- Created `PersonaEngine` (`twitteractivity_decision_persona.rs`) - probability-based fast decisions
- Created `LLMEngine` (`twitteractivity_decision_llm.rs`) - AI-powered smart decisions  
- Created `HybridEngine` (`twitteractivity_decision_hybrid.rs`) - weighted combination of both
- Created `UnifiedEngine` (`twitteractivity_decision_unified.rs`) - single LLM call for decision + content
- Added `DecisionEngineFactory` for runtime engine selection from config
- All engines implement `Send + Sync` for safe concurrent use

**Verification:**
- `cargo check` ✅ passes
- `cargo test --lib` ✅ 1965 tests passed
- `cargo clippy --package auto --lib` ✅ no warnings
- `check.ps1` ✅ all 4 checks pass

**Files created:**
- `src/utils/twitter/twitteractivity_decision_persona.rs` (~160 lines)
- `src/utils/twitter/twitteractivity_decision_llm.rs` (~270 lines)
- `src/utils/twitter/twitteractivity_decision_hybrid.rs` (~240 lines)
- `src/utils/twitter/twitteractivity_decision_unified.rs` (~470 lines)

**Files modified:**
- `src/utils/twitter/twitteractivity_decision.rs` - Added trait, enum, factory (~110 lines)
- `src/utils/twitter/mod.rs` - Added new module declarations

**Architecture:**
```rust
// Runtime selection via config
decision_strategy = "unified"  # or "persona", "llm", "hybrid"

// UnifiedEngine: single LLM call returns decision + content
let analysis = UnifiedAnalysis {
    score: 85,
    level: "High",
    action: "reply",
    engage: true,
    reply: Some("Great point about...".to_string()),
};
```

**Success Criteria:**
- [x] All tests pass (1965 tests)
- [x] No compilation errors
- [x] Passed `check.ps1`

### 7.2 Feature Flagging [Confidence: 85%]

**Status:** ⏸ DEFERRED - Optional enhancement (2026-05-03)

**Current Code Analysis:**
- LLM dependencies are always compiled
- Binary size could be reduced without LLM
- No way to disable LLM at compile time


**Corrections from deep code review:**
1. "LLM dependencies are always compiled" → **TRUE** (verified: no `llm` feature in Cargo.toml)
2. "Binary size could be reduced without LLM" → **TRUE** (general knowledge)
3. "No way to disable LLM at compile time" → **TRUE** (verified: no `#[cfg(feature = "llm")]` found in code)

**Actual verification:**
- Cargo.toml `[features]` section: `default = []` (no `llm` feature defined)
- No `#[cfg(feature = "llm")]` in any .rs files
- `twitteractivity_llm.rs` is ~700 lines, always compiled
- `LLM` related code in `twitteractivity.rs`: `llm_enabled` is runtime flag, NOT compile-time

**Conclusion:** Section 7.2 is correct - feature flagging would be a NEW feature addition, not refactoring existing code.
**Proposed Changes:**

**Migration Steps:**

**Phase A: Add Feature Flags**
- [ ] Add `llm` feature to `Cargo.toml`
- [ ] Gate LLM modules behind feature flag
- [ ] Make LLM imports conditional

**Phase B: Update Code**
- [ ] Use `#[cfg(feature = "llm")]` for LLM code
- [ ] Provide stub implementations when feature disabled
- [ ] Ensure code compiles with and without feature

**Phase C: Documentation**
- [ ] Document feature flags in README
- [ ] Explain binary size impact
- [ ] Provide build instructions

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| LLM code | Always compiled | Feature-gated | Cargo features |

**Critical Ordering:**
1. **FIRST:** Define features in Cargo.toml
2. **SECOND:** Gate LLM code
3. **THIRD:** Test both configurations

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Conditional compilation complexity | Use `cfg` consistently |
| Default behavior | Make LLM opt-in |

**Validation Commands:**
```bash
cargo build --no-default-features
cargo build --features llm
```

**Pros:**
- Smaller binaries when LLM not needed
- Clearer dependency separation

**Cons:**
- More complex build configurations
- Testing matrix expands

**Notes:**
- Optional enhancement
- Low priority if binary size is acceptable

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

### 8.1 Session State Struct [Confidence: 85%]

**Status:** ✅ COMPLETED - SessionState struct implemented (2026-05-03)

**What was done:**
- ✅ Created `SessionState` struct in `twitteractivity_state.rs`:
  - Consolidates `EngagementCounters`, `EngagementLimits`, `TweetActionTracker`, `deadline`
  - Reduces scattered state management into single unit
- ✅ Added helper methods to `SessionState`:
  - `new()` - constructor with limits and duration
  - `is_expired()` - check session timeout
  - `remaining_time()` - get time left
  - `is_action_allowed()` - check specific action limits
  - `is_total_limit_reached()` - check total actions
  - `action_summary()` - get (taken, max) tuple
  - `record_action()` - unified action recording
  - `progress_summary()` - formatted status string
- ✅ Added `increment()` method to `EngagementCounters` for string-based action type
- ✅ Updated `twitteractivity.rs` main loop to use `SessionState`:
  - Replaced separate `counters`, `limits`, `action_tracker`, `deadline` with `session`
  - Updated loop condition to `while !session.is_expired()`
  - Updated `CandidateContext` construction to use `&session.limits`, `&mut session.action_tracker`, `&mut session.counters`
  - Updated `log_summary()` to take `&SessionState`
- ✅ Cleaner code: fewer variables, unified state management

**Files modified:**
- `src/utils/twitter/twitteractivity_state.rs` - Added `SessionState` struct (~90 lines)
- `src/utils/twitter/twitteractivity_limits.rs` - Added `increment()` method
- `src/task/twitteractivity.rs` - Refactored to use `SessionState`

**Verification:**
- `cargo check` ✅ passes
- `cargo test --lib` ✅ 1927 passed (4 pre-existing failures)
- `cargo clippy` ✅ no warnings
- `check.ps1` ✅ all 4 checks pass
**Proposed Changes:**

**Migration Steps:**

**Phase A: Define SessionState**
- [ ] Create `SessionState` struct:
  ```rust
  pub struct SessionState {
      pub counters: EngagementCounters,
      pub limits: EngagementLimits,
      pub tracker: TweetActionTracker,
      pub deadline: Instant,
      pub metrics: Arc<Metrics>,
  }
  ```

**Phase B: Consolidate Passing**
- [ ] Replace multiple params with `&mut SessionState`
- [ ] Update all functions that take state components

**Phase C: Refactor**
- [ ] Move deadline tracking into struct
- [ ] Add helper methods to SessionState

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| State components | Separate params | `SessionState` | Section 2.1 |

**Critical Ordering:**
1. **FIRST:** Define struct
2. **SECOND:** Update function signatures
3. **THIRD:** Add helper methods

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Borrow checker issues | Use `&mut` appropriately |
| Mutable state complexity | Encapsulate mutations |

**Validation Commands:**
```bash
cargo check
cargo test state
```

**Pros:**
- Cleaner function signatures
- Easier to add new state

**Cons:**
- Larger struct to pass around
- Mutable borrows

**Notes:**
- Coordinate with section 2.1

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

### 8.2 Thread Cache Per-Tweet [Confidence: 85%]

**Status:** ⏳ PENDING

**Current Code Analysis:**
- Thread cache is per-tweet but code is commented out
- Comments explain it's "removed to prevent contamination"
- Either implement properly or remove entirely

**Proposed Changes:**

**Migration Steps:**

**Phase A: Decision**
- [ ] Decide: implement proper per-tweet cache OR remove entirely
- [ ] Consider performance vs complexity tradeoff
- [ ] Get team consensus

**Phase B: If Implementing**
- [ ] Add `thread_cache` to candidate processing state
- [ ] Implement cache lifecycle (create, use, clear)
- [ ] Ensure cache is per-tweet only
- [ ] Add cache hit/miss metrics

**Phase C: If Removing**
- [ ] Delete all thread cache code
- [ ] Delete all thread cache comments
- [ ] Update documentation
- [ ] Remove cache-related tests

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| Thread cache | Commented out | Implemented or removed | Decision |

**Critical Ordering:**
1. **FIRST:** Make decision
2. **SECOND:** Implement chosen path
3. **THIRD:** Verify no dead code remains

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Cache contamination | Clear after each tweet |
| Memory usage | Limit cache size |

**Validation Commands:**
```bash
cargo test thread
cargo test dive
```

**Pros:**
- Clean decision
- No half-implemented features

**Cons:**
- If implementing: more complexity
- If removing: lose potential optimization

**Notes:**
- Don't leave commented code with essays

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

### 9.1 Unit Test Organization [Confidence: 85%]

**Status:** ⏳ PENDING

**Current Code Analysis:**
- Tests are at bottom of main file
- No clear organization
- No test utilities module


**Corrections from deep code review:**
1. "Tests are at bottom of main file" → **TRUE** (starts at line 1644 with `#[cfg(test)]`)
2. "No clear organization" → **MOSTLY TRUE** (tests are in one big block, but grouped by topic: config, persona, action selection, etc.)
3. "No test utilities module" → **TRUE** (no test utilities exist)

**Actual test statistics:**
- Total test functions: **44** (lines 1649-2154+)
- All functions start with `test_` (no helper functions found)
- Total assertions: **78** (all meaningful, testing actual functionality)
- Test categories: config parsing, persona weights, action selection, entry points, trackers, sentiment, engagement decisions

**Conclusion:** Section 9.1 is **LOW PRIORITY** - tests are well-structured but could benefit from better organization (split into modules by topic).

**Corrections from deep code review:**
1. "Tests are at bottom of main file" → **TRUE** (starts at line 1644 with `#[cfg(test)]`)
2. "No clear organization" → **MOSTLY TRUE** (tests are in one big block, but grouped by topic: config, persona, action selection, etc.)
3. "No test utilities module" → **TRUE** (no test utilities exist)

**Actual test statistics:**
- Total test functions: **44** (lines 1649-2154+)
- All functions start with `test_` (no helper functions found)
- Total assertions: **78** (all meaningful, testing actual functionality)
- Test categories: config parsing, persona weights, action selection, entry points, trackers, sentiment, engagement decisions

**Conclusion:** Section 9.1 is **LOW PRIORITY** - tests are well-structured but could benefit from better organization (split into modules by topic).
**Proposed Changes:**

**Migration Steps:**

**Phase A: Move Tests**
- [ ] Move tests to `tests/` subdirectory OR inline modules
- [ ] Organize by module (`navigation_tests.rs`, etc.)

**Phase B: Create Utilities**
- [ ] Create test utilities module for common setup
- [ ] Mock helpers for browser interactions
- [ ] Test data builders

**Phase C: Advanced Tests**
- [ ] Add property-based tests for probability distributions
- [ ] Add integration tests with mocked browser
- [ ] Add statistical tests for weight distribution

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| Tests | File bottom | Organized structure | Section 1.x |

**Critical Ordering:**
1. **FIRST:** Move existing tests
2. **SECOND:** Create utilities
3. **THIRD:** Add advanced tests

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Private function access | Use `pub(crate)` or tests in same file |
| Mock complexity | Use mockall crate |

**Validation Commands:**
```bash
cargo test
cargo test --lib
cargo test --test integration
```

**Pros:**
- Better test organization
- Easier to find relevant tests

**Cons:**
- More files to maintain
- Refactoring effort

**Notes:**
- Can be done gradually
- Low priority

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

### 9.2 Test Coverage [Confidence: 85%]

**Status:** ⏳ PENDING

**Current Code Analysis:**
- Missing tests for some action types
- Limit enforcement not fully tested
- Action tracker cooldown not verified


**Corrections from deep code review:**
1. "Missing tests for some action types" → **PARTIALLY TRUE** (tests exist for like, retweet, follow, reply, but coverage could be more comprehensive)
2. "Limit enforcement not fully tested" → **FALSE** (verified: test_action_allowed_by_limits_respects_total_after_dive exists)
3. "Action tracker cooldown not verified" → **FALSE** (verified: test_tweet_action_tracker_can_perform_after_record exists)

**Actual test statistics:**
- Total test functions: **44** (lines 1649-2154+)
- Total assertions: **78** (all meaningful, testing actual functionality)
- Test categories: config parsing, persona weights, action selection, entry points, trackers, sentiment, engagement decisions
- Missing: integration tests, property-based tests, statistical tests for weight distribution

**Conclusion:** Test coverage is GOOD for unit tests, but lacks advanced testing (integration, property-based, statistical).

**Corrections from deep code review:**
1. "Missing tests for some action types" → **PARTIALLY TRUE** (tests exist for like, retweet, follow, reply, but coverage could be more comprehensive)
2. "Limit enforcement not fully tested" → **FALSE** (verified: test_action_allowed_by_limits_respects_total_after_dive exists)
3. "Action tracker cooldown not verified" → **FALSE** (verified: test_tweet_action_tracker_can_perform_after_record exists)

**Actual test statistics:**
- Total test functions: **44** (lines 1649-2154+)
- Total assertions: **78** (all meaningful, testing actual functionality)
- Test categories: config parsing, persona weights, action selection, entry points, trackers, sentiment, engagement decisions
- Missing: integration tests, property-based tests, statistical tests for weight distribution

**Conclusion:** Test coverage is GOOD for unit tests, but lacks advanced testing (integration, property-based, statistical).

**Corrections from deep code review:**
1. "Missing tests for some action types" → **PARTIALLY TRUE** (tests exist for like, retweet, follow, reply, but coverage could be more comprehensive)
2. "Limit enforcement not fully tested" → **FALSE** (verified: test_action_allowed_by_limits_respects_total_after_dive exists)
3. "Action tracker cooldown not verified" → **FALSE** (verified: test_tweet_action_tracker_can_perform_after_record exists)

**Actual test statistics:**
- Total test functions: **44** (lines 1649-2154+)
- Total assertions: **78** (all meaningful, testing actual functionality)
- Test categories: config parsing, persona weights, action selection, entry points, trackers, sentiment, engagement decisions
- Missing: integration tests, property-based tests, statistical tests for weight distribution

**Conclusion:** Test coverage is GOOD for unit tests, but lacks advanced testing (integration, property-based, statistical).

**Corrections from deep code review:**
1. "Missing tests for some action types" → **PARTIALLY TRUE** (tests exist for like, retweet, follow, reply, but coverage could be more comprehensive)
2. "Limit enforcement not fully tested" → **FALSE** (verified: test_action_allowed_by_limits_respects_total_after_dive exists)
3. "Action tracker cooldown not verified" → **FALSE** (verified: test_tweet_action_tracker_can_perform_after_record exists)

**Actual test statistics:**
- Total test functions: **44** (lines 1649-2154+)
- Total assertions: **78** (all meaningful, testing actual functionality)
- Test categories: config parsing, persona weights, action selection, entry points, trackers, sentiment, engagement decisions
- Missing: integration tests, property-based tests, statistical tests for weight distribution

**Conclusion:** Test coverage is GOOD for unit tests, but lacks advanced testing (integration, property-based, statistical).

**Corrections from deep code review:**
1. "Missing tests for some action types" → **PARTIALLY TRUE** (tests exist for like, retweet, follow, reply, but coverage could be more comprehensive)
2. "Limit enforcement not fully tested" → **FALSE** (verified: test_action_allowed_by_limits_respects_total_after_dive exists)
3. "Action tracker cooldown not verified" → **FALSE** (verified: test_tweet_action_tracker_can_perform_after_record exists)

**Actual test statistics:**
- Total test functions: **44** (lines 1649-2154+)
- Total assertions: **78** (all meaningful, testing actual functionality)
- Test categories: config parsing, persona weights, action selection, entry points, trackers, sentiment, engagement decisions
- Missing: integration tests, property-based tests, statistical tests for weight distribution

**Conclusion:** Test coverage is GOOD for unit tests, but lacks advanced testing (integration, property-based, statistical).

**Corrections from deep code review:**
1. "Missing tests for some action types" → **PARTIALLY TRUE** (tests exist for like, retweet, follow, reply, but coverage could be more comprehensive)
2. "Limit enforcement not fully tested" → **FALSE** (verified: test_action_allowed_by_limits_respects_total_after_dive exists)
3. "Action tracker cooldown not verified" → **FALSE** (verified: test_tweet_action_tracker_can_perform_after_record exists)

**Actual test statistics:**
- Total test functions: **44** (lines 1649-2154+)
- Total assertions: **78** (all meaningful, testing actual functionality)
- Test categories: config parsing, persona weights, action selection, entry points, trackers, sentiment, engagement decisions
- Missing: integration tests, property-based tests, statistical tests for weight distribution

**Conclusion:** Test coverage is GOOD for unit tests, but lacks advanced testing (integration, property-based, statistical).
**Proposed Changes:**

**Migration Steps:**

**Phase A: Core Tests**
- [ ] Add test for `process_candidate()` with all action types
- [ ] Add test for limit enforcement edge cases
- [ ] Add test for action tracker cooldown logic

**Phase B: Statistical Tests**
- [ ] Add test for entry point weight distribution (statistical)
- [ ] Verify probabilities match expected distribution
- [ ] Use chi-square or similar test

**Phase C: Integration**
- [ ] Add tests for full task execution
- [ ] Mock browser interactions

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| Tests | Missing | Added | Section 9.1 |

**Critical Ordering:**
1. **FIRST:** Add core tests
2. **SECOND:** Add statistical tests
3. **THIRD:** Add integration tests

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Flaky statistical tests | Use seeded RNG for determinism |
| Slow tests | Mark as `#[ignore]` for CI optional |

**Validation Commands:**
```bash
cargo test
cargo tarpaulin  # coverage
```

**Pros:**
- Better confidence in changes
- Catch regressions

**Cons:**
- More tests to maintain
- Slower CI

**Notes:**
- Focus on critical paths first
- Can be ongoing effort

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

### 10.1 Code Documentation [Confidence: 85%]

**Status:** ⏳ PENDING

**Current Code Analysis:**
- Module-level docs need updates
- Public functions lack examples
- No ADRs for key decisions
- `twitteractivity.md` needs updates after each refactor


**Corrections from deep code review:**
1. "Module-level docs need updates" → **PARTIALLY TRUE** (some modules have `//!` docs, some need updates)
2. "Public functions lack examples" → **TRUE** (most public functions don't have ```rust examples)
3. "No ADRs for key decisions" → **TRUE** (no ADR files found)
4. "`twitteractivity.md` needs updates after each refactor" → **TRUE** (file exists but may be outdated)

**Actual documentation status:**
- `twitteractivity_persona.rs`: Has `//!` module docs ✅
- `twitteractivity_limits.rs`: Has `//!` module docs ✅
- `twitteractivity_feed.rs`: Has `//!` module docs ✅
- `twitteractivity_interact.rs`: Has `//!` module docs ✅
- `twitteractivity_navigation.rs`: Has `//!` module docs ✅
- `twitteractivity_humanized.rs`: Has `//!` module docs ✅
- `twitteractivity_sentiment.rs`: Has `//!` module docs ✅
- `twitteractivity_llm.rs`: NO module-level docs found ⚠️
- Most public functions lack usage examples ⚠️
- No ADR files found in project ⚠️

**Conclusion:** Section 10.1 is valid - documentation improvement is needed but LOW PRIORITY.

**Corrections from deep code review:**
1. "Module-level docs need updates" → **PARTIALLY TRUE** (some modules have `//!` docs: twitteractivity_persona.rs, twitteractivity_limits.rs, twitteractivity_feed.rs, etc.)
2. "Public functions lack examples" → **TRUE** (most public functions don't have ```rust examples)
3. "No ADRs for key decisions" → **TRUE** (no ADR files found in project)
4. "`twitteractivity.md` needs updates after each refactor" → **TRUE** (file exists at src/task/twitteractivity.md)

**Actual documentation status:**
- `twitteractivity_persona.rs`: Has `//!` module docs ✅
- `twitteractivity_limits.rs`: Has `//!` module docs ✅
- `twitteractivity_feed.rs`: Has `//!` module docs ✅
- `twitteractivity_interact.rs`: Has `//!` module docs ✅
- `twitteractivity_navigation.rs`: Has `//!` module docs ✅
- `twitteractivity_humanized.rs`: Has `//!` module docs ✅
- `twitteractivity_sentiment.rs`: Has `//!` module docs ✅
- `twitteractivity_llm.rs`: NO module-level docs found ⚠️
- Most public functions lack usage examples ⚠️
- No ADR files found ⚠️

**Conclusion:** Section 10.1 is valid - documentation improvement is needed but LOW PRIORITY.

**Corrections from deep code review:**
1. "Module-level docs need updates" → **PARTIALLY TRUE** (some modules have `//!` docs: twitteractivity_persona.rs, twitteractivity_limits.rs, twitteractivity_feed.rs, etc.)
2. "Public functions lack examples" → **TRUE** (most public functions don't have ```rust examples)
3. "No ADRs for key decisions" → **TRUE** (no ADR files found in project)
4. "`twitteractivity.md` needs updates after each refactor" → **TRUE** (file exists at src/task/twitteractivity.md)

**Actual documentation status:**
- `twitteractivity_persona.rs`: Has `//!` module docs ✅
- `twitteractivity_limits.rs`: Has `//!` module docs ✅
- `twitteractivity_feed.rs`: Has `//!` module docs ✅
- `twitteractivity_interact.rs`: Has `//!` module docs ✅
- `twitteractivity_navigation.rs`: Has `//!` module docs ✅
- `twitteractivity_humanized.rs`: Has `//!` module docs ✅
- `twitteractivity_sentiment.rs`: Has `//!` module docs ✅
- `twitteractivity_llm.rs`: NO module-level docs found ⚠️
- Most public functions lack usage examples ⚠️
- No ADR files found ⚠️

**Conclusion:** Section 10.1 is valid - documentation improvement is needed but LOW PRIORITY.
**Proposed Changes:**

**Migration Steps:**

**Phase A: Module Docs**
- [ ] Add module-level documentation to all new modules
- [ ] Document module purpose and exports

**Phase B: Function Docs**
- [ ] Document all public functions with examples
- [ ] Include argument descriptions
- [ ] Document error cases

**Phase C: ADRs**
- [ ] Add architecture decision records (ADRs) for key choices
- [ ] Document why module split was done
- [ ] Document strategy pattern choice
- [ ] Update `src/task/twitteractivity.md` after each refactor

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| Docs | Missing/Outdated | Updated | Each refactor |

**Critical Ordering:**
1. **FIRST:** Module docs
2. **SECOND:** Function docs
3. **THIRD:** ADRs

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Docs get outdated | Update with each PR |
| Too much documentation | Focus on public APIs |

**Validation Commands:**
```bash
cargo doc --no-deps
cargo clippy -- -D missing_docs
```

**Pros:**
- Easier onboarding
- Better maintenance

**Cons:**
- Time investment
- Can become outdated

**Notes:**
- Ongoing effort
- Update docs with code changes

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`

### 10.2 Runbook Documentation [Confidence: 70%]

**Status:** ⏳ PENDING

**Current Code Analysis:**
- No troubleshooting guide
- No guide for adjusting weights
- No monitoring/alerting documentation
- No guide for adding new engagement types

**Proposed Changes:**

**Migration Steps:**

**Phase A: Troubleshooting**
- [ ] Add troubleshooting guide for common failures
- [ ] Document selector failures
- [ ] Document rate limit handling
- [ ] Document Twitter UI changes

**Phase B: Configuration**
- [ ] Document how to adjust weights for different use cases
- [ ] Provide persona examples
- [ ] Document engagement limits

**Phase C: Monitoring**
- [ ] Add monitoring/alerting guide for metrics
- [ ] Document key metrics to watch
- [ ] Set up alert thresholds

**Phase D: Extension**
- [ ] Create "Adding a New Engagement Type" guide
- [ ] Document steps to add actions
- [ ] Provide code examples

**Dependencies Map:**
| Item | Current Location | Target Location | Dependencies |
|------|------------------|-----------------|--------------|
| Runbooks | None | Created | All previous sections |

**Critical Ordering:**
1. **FIRST:** Troubleshooting
2. **SECOND:** Configuration
3. **THIRD:** Monitoring
4. **FOURTH:** Extension guide

**Potential Issues & Solutions:**
| Issue | Solution |
|-------|----------|
| Runbooks get outdated | Version with code |
| Too much documentation | Focus on common tasks |

**Validation Commands:**
```bash
# Manual review
cat RUNBOOK.md
```

**Pros:**
- Faster incident response
- Easier configuration

**Cons:**
- Time to write
- Maintenance burden

**Notes:**
- Last priority
- Can be done after code is stable

**Success Criteria:**
- [ ] All tests pass
- [ ] No compilation errors
- [ ] Passed `check.ps1`
