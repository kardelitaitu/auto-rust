*last audited 23-06-26 by Buffy*

# Phase 8 — Core Module Test Expansion & Doc Auditing

*Previous scope (Layers 1-7: types, proptest, fuzzing, mutants, coverage, miri, lint hardening, twitter orchestrator) is archived below. All completed or blocked.*

---

## Current State

| Metric | Value |
|---|---|
| Lib tests | 3,985 passing ✅ |
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

**Target:** +100-200 tests, measure coverage improvement via `.\coverage.ps1 -Target llm`

---

## P2: Session Module — Pool, Permits, Factory (`src/session/`, 3,902 lines, 185 tests)

Manages browser session lifecycle — one of the highest-risk areas for resource leaks and race conditions.

| File | Lines | Tests | Notes |
|---|---|---|---|
| `mod.rs` | 1,799 | ~100 | Core session logic — largest file |
| `connector.rs` | 662 | ✗ | Browser connection — mostly async, lower ROI |
| `pool.rs` | 630 | ✗ | Session pool — resource management, high ROI |
| `factory.rs` | 328 | ✗ | Session factory — constructors, config merging |
| `duration.rs` | 224 | ✗ | Duration math — pure functions, highest ROI |
| `cleanup.rs` | 180 | ✗ | Cleanup logic — state machine, medium ROI |
| `state.rs` | 46 | ✗ | State enum — trivial |
| `permits.rs` | 33 | ✗ | Semaphore wrapper — trivial |

- [ ] **Duration** — Extract duration math from `duration.rs` as pure functions with property tests
- [ ] **Pool** — Test pool allocation/deallocation, capacity enforcement, timeout behavior
- [ ] **Factory** — Test session creation with various config combinations
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

## Priority Order

1. **P1: LLM module** — Highest risk/reward (handles all LLM communication, API parsing)
2. **P2: Session module** — Resource management, leak prevention
3. **P3: Orchestrator** — Retry logic, guard conditions
4. **P4: Doc auditing** — Low effort, maintain as background task

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
