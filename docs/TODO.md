*last audited 23-06-26 by Buffy, refreshed 23-06-26 by Buffy*

# Phase 8 — Core Module Test Expansion & Doc Auditing

*Previous scope (Layers 1-7: types, proptest, fuzzing, mutants, coverage, miri, lint hardening, twitter orchestrator) is archived below. All completed or blocked.*

---

## Current State

| Metric | Value |
|---|---|
| Lib tests | 4,046 passing ✅ |
| Integration tests | 3,984 passing, 0 failing ✅ |
| Clippy | Clean ✅ |
| `check.ps1` | 8/8 green ✅ |
| Doc auditing | Conveyor belt active — 12+ files stamped |

---

## P1: LLM Module — Test Expansion (`src/llm/`, 4,562 lines, 288 tests)

The LLM module handles all LLM API communication, response parsing, and fallback strategies. Currently 288 tests — room to grow by extracting pure functions from the async orchestration.

| File | Lines | Current Tests | Opportunity |
|---|---|---|---|
| `unified_processor.rs` | 965 | ✗ | Decision parsing, fallback logic — extract pure functions |
| `reply_strategies.rs` | 640 | ✗ | Strategy selection, scoring — pure computation |
| `models.rs` | 564 | ✗ | Model config parsing, validation — pure data |
| `reply_engine.rs` | 402 | ✗ | Reply generation logic — extractable |
| `mod.rs` | 304 | ✗ | Module-level orchestration |
| `client.rs` | 1,687 | ✗ | API client — mostly async I/O, lower ROI |

- [ ] **Analyze** — Identify pure functions in `unified_processor.rs`, `reply_strategies.rs`, `reply_engine.rs`
- [ ] **Extract** — Pull pure computation out of async orchestration
- [ ] **Test** — Add unit tests for extracted functions
- [ ] **Verify** — Run `cargo test --lib llm` + clippy

**Target:** +100-200 tests, measure coverage improvement via `.coverage.ps1 -Target llm`

---

## P2: Session Module — Pool, Permits, Factory (`src/session/`, 3,902 lines, 185 tests)

Manages browser session lifecycle — one of the highest-risk areas for resource leaks and race conditions.

| File | Lines | Tests | Notes |
|---|---|---|---|
| `mod.rs` | 1,799 | ~100 | Core session logic — largest file |
| `connector.rs` | 662 | ✅ | Browser connection — extracted helpers, tests added |
| `pool.rs` | 630 | ✅ | Session pool — normalize_browser_token tests added |
| `factory.rs` | 328 | ✅ | Session factory — constructors, config merging, builder tests added |
| `duration.rs` | 224 | ✗ | Duration math — pure functions, highest ROI |
| `cleanup.rs` | 180 | ✗ | Cleanup logic — state machine, medium ROI |
| `state.rs` | 46 | ✗ | State enum — trivial |
| `permits.rs` | 33 | ✗ | Semaphore wrapper — trivial |

- [x] **Connector** — Extracted helpers (`cdp_version_url`, `extract_ws_url_from_version`, `make_local_browser_capability`), added comprehensive tests ✅
- [x] **Pool** — Added `normalize_browser_token` test suite ✅
- [x] **Factory** — Added timeout clamping, getter, builder tests ✅
- [ ] **Duration** — Extract duration math from `duration.rs` as pure functions with property tests
- [ ] **Pool (remaining)** — Allocation/deallocation, capacity enforcement, timeout behavior
- [ ] **Factory (remaining)** — More session creation config combinations
- [ ] **Verify** — Run `cargo test --lib session` + clippy

**Target:** +50-100 tests

---

## P3: Orchestrator — Retry, Guards, Health (`src/orchestrator/`, 1,690 lines, 50 tests)

Coordinates task execution, retry logic, health monitoring, and pre-condition guards.

| File | Lines | Tests | Notes |
|---|---|---|---|
| `retry.rs` | 412 | ✗ | Retry strategy — backoff calc, max retries, pure functions |
| `mod.rs` | 397 | ✗ | Orchestration core |
| `execution.rs` | 345 | ✗ | Task execution flow |
| `guards.rs` | 328 | ✗ | Pre-condition guards — logic heavy |
| `health.rs` | 159 | ✗ | Health checks — pure predicates |
| `test_utils.rs` | 49 | ✗ | Test helpers |

- [ ] **Retry** — Extract backoff computation as pure functions
- [ ] **Guards** — Test guard logic (permission checks, state validation)
- [ ] **Health** — Test health check predicates
- [ ] **Verify** — Run `cargo test --lib orchestrator` + clippy

**Target:** +50-100 tests

---

## P4: Doc Audit Conveyor Belt

Continue running `find-oldest-files.ps1` to surface and stamp old `.md` and `.rs` files.

- [ ] Run `find-oldest-files.ps1` to get next batch
- [ ] Audit references against current codebase
- [ ] Stamp with `re-audited <date> by Buffy`
- [ ] Commit in batches

---

## Phase 9 — 10 Improvement Opportunities (identified 23-06-26)

*Surfaced by scanning: clippy allow annotations, unwrap/expect hotspots, large file candidates, dead code, std::sync::Mutex usage, .clone() call density.*

| # | Area | File(s) | Opportunity | Est. Impact |
|---|---|---|---|---|
| 1 | **🪦 Dead code** | `src/adaptive/predictive_scorer.rs` | 19 `#[allow(dead_code)]` annotations (lines 21-120). Remove dead fields/methods or add real usage. | Medium |
| 2 | **⚠️ Unwrap hotspots** | `src/adaptive/learning_engine.rs` | 6 direct `.unwrap()` calls (lines 312, 344, 380, 402, 411, 413). Replace with proper error handling or expect with context. | High |
| 3 | **⚠️ Expect hotspots** | `src/api/client.rs` | Multiple `.expect("Should succeed")` calls (lines 900, 936, 984). These will panic on API errors — replace with proper error propagation. | High |
| 4 | **📦 Large file splitting** | `src/session/mod.rs` (1,799 ln) | Second-largest non-test file. Split session creation, lifecycle, and page management into sub-modules. | Medium |
| 5 | **📦 Large file splitting** | `src/utils/profile.rs` (1,746 ln) | Profile loading, parsing, and caching in one file. Split into reader/writer/cache modules. | Medium |
| 6 | **🔒 std::sync::Mutex audit** | `src/logger.rs`, `src/task/dsl/api.rs`, `src/utils/mouse/native.rs` | 3 `std::sync::Mutex` usages. If held across `.await` points, should be `tokio::sync::Mutex` to prevent blocking the runtime. | Medium |
| 7 | **📋 Unnecessary .clone()** | 433 calls across src/ | Audit hot paths for avoidable `.clone()` calls. Likely candidates: large config structs, strings passed by value. | Low-Medium |
| 8 | **🔄 Stream simplification** | `src/orchestrator/`, `src/session/` | `FuturesUnordered` + `StreamExt` patterns could potentially use simpler `tokio::join!` or `JoinSet` where order doesn't matter. | Low-Medium |
| 9 | **🔧 Clippy allow cleanup** | `src/api/client.rs`, `src/capabilities/mod.rs`, `src/health_logger.rs` | 3 `#[allow(...)]` annotations in non-test code that could be resolved instead of suppressed. | Low |
| 10 | **📐 API surface audit** | 947 public items across src/ | Audit `pub` visibility — many items may only need `pub(crate)`. Shrinking surface reduces accidental coupling. | Low |

**Suggested order:** #2 (unwrap), #3 (expect) → #1 (dead code) → #6 (mutex) → #4/5 (large files) → #7/8/9/10

---

## Priority Order

1. **P1: LLM module** — Highest risk/reward (handles all LLM communication, API parsing)
2. **P2: Session module** — Resource management, leak prevention
3. **P3: Orchestrator** — Retry logic, guard conditions
4. **P4: Doc auditing** — Low effort, maintain as background task
5. **Phase 9: Improvement items** — See table above

---

## What NOT to Do (still valid)

| Low-ROI activity | Why |
|---|---|
| More twitter pure-function extraction | Exhausted — remaining gaps are browser-dependent |
| `dom.rs` tests | Feature-gated (`accessibility-locator`), browser-only |
| `bacon-pipeline` coverage | Separate crate, 22 files, tests exist but not measured from main crate |
| `mutants.ps1` on Windows | `cargo-mutants` v27.1.0 `nul` copy bug — blocked |

---

## Archived: Layers 1-7 (Completed)

### Layer 1-3: Type Safety, Property Testing & Fuzzing
- **Newtypes & State Machines** — `TweetId`, `StatusUrl`, `ReplyFlowState`, `EngagementOutcome`, `FollowOutcome`, `PostOutcome` migrated.
- **Property-Based Testing** — Proptests active for timing ranges, persona weights, sentiment modulation, emoji removal, and engagement limits.
- **Fuzzing** — LLM parser and spec file loader fuzz targets verified (found + fixed real LLM parsing bug).

### Layer 4: Mutation Testing
- Install + script created, but **blocked by Windows `nul` copy bug** in `cargo-mutants` v27.1.0. Requires WSL/Linux.

### Layer 5: Coverage-Guided Gap Analysis
- Full project coverage: 40.95% (7,849/19,167 lines)
- Decision engine: 91-100% (276 tests across all modules)
- `extract_tweet_text` consolidated, coordinate parsing centralized, pure functions moved + tested
- Remaining low-coverage areas are browser/LLM-dependent (low ROI)

### Layer 6: Dynamic Analysis & Lint Hardening
- Miri: 18/18 tests, no UB ✅
- `serde_yml` → `serde_yaml` migration (129 occurrences)
- Dead code audit (88 annotations)
- `unwrap_used`, `expect_used`, `unsafe_code` enforced across pipeline

### Layer 7: Twitter Orchestrator Weaknesses
- Loop scheduler starvation, async cancellation safety, dynamic pause scaling, LLM context fallback — all resolved ✅

### Result Module
- `src/result/` — 370 tests across `types.rs` (57), `errors.rs` (106), `summary.rs` (42), all passing ✅
