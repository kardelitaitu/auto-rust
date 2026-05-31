# DSL Task System — Audit & Improvement Plan

## Summary

The DSL system (`src/task/dsl/`) is a declarative task engine with 10 source files (~3120 lines),
23 action types, 20 condition types, 7 control flow constructs (If/Else, Loop, Foreach, While,
Retry, Parallel, Try/Catch/Finally), LRU+TTL selector caching, profiling infrastructure,
breakpoint debugging, YAML/TOML parsing, and task composition with variable inheritance.
It's consumed by 8 files outside the DSL module and has 34 integration tests + ~36 unit tests.

This document covers bugs, design issues, gaps, and improvements, ordered by severity.

---

## Files Covered

| File | Lines | Purpose |
|------|-------|---------|
| `src/task/dsl/types.rs` | 506 | `TaskDefinition`, `Action` (23 variants), `Condition` (20 variants), support types |
| `src/task/dsl/parser.rs` | 324 | YAML/TOML parsing, `validate_task_definition()`, task lookup |
| `src/task/dsl/evaluator.rs` | 318 | `${var}` substitution, condition evaluation |
| `src/task/dsl/executor.rs` | 792 | `DslExecutor`, `execute()` loop, per-action dispatch, cached DOM wrappers |
| `src/task/dsl/control_flow.rs` | 369 | If/Else, Loop, Foreach, While, Retry, Parallel |
| `src/task/dsl/cache.rs` | 231 | `SelectorCache` with LRU + 5s TTL |
| `src/task/dsl/profiling.rs` | 271 | `ActionProfiler`, `ActionMetrics`, `ExecutionReport` |
| `src/task/dsl/debug.rs` | 219 | `DebugEvent`, `Breakpoint`, `DebugEventType` (8 variants) |
| `src/task/dsl/dsl_executor.rs` | 8 | Compatibility re-export shim |
| `src/task/dsl/mod.rs` | 74 | Module declarations, public re-exports |
| `tests/dsl_integration_tests.rs` | 703 | 16 structural tests |
| `tests/dsl_translation_tests.rs` | 182 | 5 parsing tests |
| `tests/task_composition_integration.rs` | 584 | 13 composition tests |

---

## 1. Bugs & Panic Risks

### 1a. Parallel execution is a scaffold — doesn't actually run in parallel

**File:** `src/task/dsl/control_flow.rs`
**Lines:** ~260–280

**Problem:** The `execute_parallel()` function creates a `tokio::sync::Semaphore` and spawns
tasks, but `DslExecutor` takes `&mut self` — you can't share `&mut self` across concurrent
tasks. The current implementation logs the actions but doesn't execute them in parallel
executors. It's a non-functional scaffold.

**Impact:** Users who write `parallel:` blocks expecting concurrent execution get sequential
execution with no warning that parallelism is disabled.

**Fix:** Either:
1. Make `DslExecutor` use interior mutability (`Arc<Mutex<...>>` or `tokio::sync::RwLock`)
   for its mutable state (variables, cache, counters) so parallel branches can share it.
2. Or document that parallel is future work and reject parallel blocks at validation time
   with a clear error message.

### 1b. `retry_on` filter is too broad (substring match)

**File:** `src/task/dsl/control_flow.rs`
**Lines:** ~230–250

**Problem:** `retry_on` is an `Option<Vec<String>>` where each pattern is checked with
`last_error_string.contains(pattern)`. A pattern like `"time"` matches `"timeout"`,
`"overtime"`, `"bedtime"` — any error containing those characters.

**Impact:** Users specify `retry_on: ["timeout"]` expecting to retry only on timeouts,
but it also retries on unrelated errors containing the substring "timeout" in their message.

**Fix:** Use full-word matching or regex. At minimum, document that `retry_on` uses
substring matching so callers know to be specific (e.g., `"Error: timeout"` not just
`"timeout"`).

### 1c. `evaluate_condition()` swallows all errors

**File:** `src/task/dsl/evaluator.rs`
**Lines:** ~70–250

**Problem:** Inside `evaluate_condition()`, each individual condition evaluation catches
and converts errors to `Ok(false)`:

```rust
// Pattern repeated across ElementExists, ElementVisible, TextEquals, etc.:
match self.api.exists(&resolved_selector).await {
    Ok(exists) => Ok(exists),
    Err(_) => Ok(false),  // <-- all errors silently swallowed
}
```

Note: The call sites (`execute_if`, `execute_loop`, `execute_while`) use `?` to propagate
`evaluate_condition()`'s `Result<bool>`, so top-level errors from `And`/`Or`/`Not` do
propagate. But the individual DOM/parsing errors caught inside `evaluate_condition()`
never reach the call site.

**Impact:** A browser crash mid-DSL-task is invisible for direct condition checks —
`ElementExists`/`ElementVisible`/`TextEquals`/`NumericGreaterThan` etc. just start
returning `Ok(false)`, and the task silently takes the `else` branch or skips actions.

**Fix:** Add error classification. Fatal errors (browser disconnected, CDP crash) should
propagate. Only DOM-related errors (element not found, stale reference) should be swallowed.

### 1d. `DateBefore`/`DateAfter` conditions silently fail on parse errors

**File:** `src/task/dsl/evaluator.rs`
**Lines:** ~200–235

**Problem:** Date condition parsing quietly returns `Ok(false)` on any parse failure.
Invalid variable values or format mismatches silently produce "condition is false"
rather than "condition evaluation failed."

**Fix:** Return `Err` for date parse failures so the error propagates.

### 1e. Variable stringification loses type information

**File:** `src/task/dsl/executor.rs`
**Lines:** ~120–140

**Problem:** `with_parameters()` stringifies all values to `String`. Numbers `42` and
`"42"` become identical. Boolean `false` becomes `"false"`. `NumericGreaterThan`
conditions try `"false".parse::<f64>()` which silently returns `Ok(false)`.

**Fix:** Store variable values as `serde_yml::Value` (or an enum) rather than `String`,
preserving type information for conditions that need it.

---

## 2. Logic & Design Issues

### 2a. Two `TaskDefinition` types cause import confusion

**Files:** `src/task/dsl/types.rs` + `src/cli/parser.rs`

**Problem:** Both files define a `TaskDefinition` struct with completely different fields.
Any code importing both needs aliasing. Accidental type confusion risk.

**Fix:** Rename `src/cli/parser.rs`'s `TaskDefinition` to `CliTaskDefinition` or `TaskRequest`.

### 2b. Two `dsl_executor.rs` shims create confusion

**Files:** `src/task/dsl_executor.rs`, `src/task/dsl/dsl_executor.rs`

**Problem:** Two backward-compatibility shim files that exist only for legacy import paths.
`DslExecutor` is findable in 3 places.

**Fix:** Remove both shims. Update any remaining legacy imports.

### 2c. `IncludeSpec` type exists but isn't wired to execution

**File:** `src/task/dsl/types.rs`
**Lines:** ~100–120

**Problem:** `IncludeSpec` with conditional `condition` field is parsed and stored but
never resolved during execution. Users writing `includes:` get a silent no-op.

**Fix:** Either implement include resolution (parse included files, merge actions) or
remove `IncludeSpec` and reject at validation with a clear error.

### 2d. `VariableEquals`/`VariableMatches` evaluate differently than `TextEquals`/`TextMatches`

**File:** `src/task/dsl/evaluator.rs`
**Lines:** ~150–180

**Problem:** `TextEquals` trims whitespace; `VariableEquals` doesn't. `TextMatches`/
`VariableMatches` are actually "Contains" — misleading naming.

**Fix:** Add `.trim()` to `VariableEquals`, or rename `Matches` variants to `Contains`.

### 2e. Selector cache not invalidated after DOM-changing actions

**File:** `src/task/dsl/executor.rs`

**Problem:** Cache stores results for 5 seconds. Click, Type, Navigate, etc. change the
DOM immediately but don't invalidate the cache. Subsequent checks return stale data.

**Fix:** Call `self.clear_cache()` before executing DOM-changing actions.

### 2f. No action-level timeout

**File:** `src/task/dsl/executor.rs`

**Problem:** `Click`, `Type`, `Hover`, etc. have no configurable timeout. If CDP hangs,
the task blocks indefinitely.

**Fix:** Add optional `timeout_ms` field to relevant actions. Wrap in `tokio::time::timeout()`.

### 2g. `Foreach` with `Elements` generates fragile nth-of-type selectors

**File:** `src/task/dsl/control_flow.rs`
**Lines:** ~130–160

**Problem:** `:nth-of-type()` selector is fragile with mixed DOM structures and doesn't
handle dynamic element changes between iterations.

**Fix:** Document the limitation and suggest `data-testid` selectors instead.

---

## 3. Performance Issues

### 3a. Variable copies on every Call duplicate the entire HashMap

**File:** `src/task/dsl/executor.rs`, `execute_call()`

**Problem:** Every `Call` clones ALL variables parent→child and child→parent. O(N*D)
work for deep chains with large variable maps.

**Fix:** Use diff-based copy: only copy back variables that changed.

### 3b. `with_parameters()` iterates and clones full payload repeatedly

**File:** `src/task/dsl/executor.rs`

**Problem:** Parameters are re-processed at each call depth level.

**Fix:** Pass pre-processed `HashMap<String, String>` from parent.

### 3c. Hardcoded cache TTL (5 seconds)

**File:** `src/task/dsl/cache.rs`

**Problem:** TTL is hardcoded. Not adaptable to different page speeds.

**Fix:** Make TTL configurable in `SelectorCache::new()` with default 5s.

---

## 4. Observability & Debugging

### 4a. No action-level error context in logs

**Problem:** Error logs show action index but not type or selector.

```
ERROR: Failed to execute action 3 in task 'login': Element not found
```

**Fix:** Include action type and key parameters:

```
ERROR: Failed to execute action 3 (Click: #submit-btn) in task 'login'
```

### 4b. Breakpoint infrastructure has no consumer

**File:** `src/task/dsl/debug.rs` (~200 lines)

**Problem:** Sophisticated breakpoint system (8 event types, step-through, watch variables)
with zero consumers. No CLI flag, no debugger, no API to interact with it.

**Fix:** Either add a `--debug` CLI flag, or remove the dead code, or document as "reserved."

### 4c. `ExecutionReport::to_json()` not emitted anywhere

**File:** `src/task/dsl/profiling.rs`

**Problem:** Full execution metrics collected but never persisted or displayed outside tests.

**Fix:** Write report JSON to file after task completion.

---

## 5. Testing Gaps

### 5a. Executor has placeholder tests only

**File:** `src/task/dsl/executor.rs` (761+)

**Problem:** 450 lines of core execution logic — zero meaningful unit tests.

**Fix:** Add mock-based tests for `execute()`, action dispatch, error handling.

### 5b. Control flow has empty test block

**File:** `src/task/dsl/control_flow.rs` (366+)

**Problem:** 300 lines of control flow logic — zero unit tests.

**Fix:** Add tests for If/Else, Loop, Foreach, While, Retry, Parallel, Try/Catch/Finally.

### 5c. No property/fuzz testing for parser

**Problem:** YAML/TOML parsing is user-facing with no fuzz coverage.

**Fix:** Add `proptest`/`quickcheck` tests for round-trip serialization.

---

## 6. Maintainability Issues

### 6a. `execute_action()` is a single 300-line match

**File:** `src/task/dsl/executor.rs`

**Problem:** Same anti-pattern that was fixed in `twitteractivity.rs` — the main dispatch
is a giant match with 23 arms containing complex logic (lines 198–430).

**Fix:** Split into per-action methods. Match becomes thin dispatch.

### 6b. Duplicate compatibility shims

**Files:** `src/task/dsl_executor.rs`, `src/task/dsl/dsl_executor.rs`

See 2b. Remove both.

### 6c. `DslExecutionStats` defined but not populated

**File:** `src/task/dsl/executor.rs`

**Problem:** Public struct with basic counters, never populated or returned from `execute()`.

**Fix:** Populate during execution or remove in favor of `ExecutionReport`.

---

## 7. Cross-Module Integration Issues

### 7a. No schema versioning in `TaskDefinition`

**Problem:** No version field. Old task files silently misbehave as DSL evolves.

**Fix:** Add optional `version` field with `min_version` constant. Warn on mismatch.

### 7b. No URI scheme for task references in `Call`

**Problem:** `Call` only supports registry names. No file path or URL references.

**Fix:** If task name contains `/` or `.`, treat it as a file path and parse inline.

---

## Implementation Order

### Phase 1 — Bugs (low effort, high impact)
1a. Document `Parallel` as future work / reject at validation
1d. Fix date condition error propagation
2e. Invalidate cache after DOM-changing actions
2g. Document nth-of-type limitation

### Phase 2 — Design (medium effort, high impact)
2a. Rename CLI `TaskDefinition` to `CliTaskDefinition`
2b. Remove both `dsl_executor.rs` shims
2c. Implement `IncludeSpec` or remove it
6a. Split `execute_action()` into per-action methods

### Phase 3 — Performance (medium effort, medium impact)
3a. Diff-based variable copy in Call
3b. Pre-processed parameter passing
3c. Configurable cache TTL

### Phase 4 — Testing (large effort, high impact)
5a. Mock-based executor tests
5b. Control flow unit tests
5c. Parser fuzz testing

### Phase 5 — Polish (small effort, low impact)
4a. Better error context
4b. Either wire up breakpoints or remove them
4c. Emit execution report
6c. Populate or remove `DslExecutionStats`
7a. Schema versioning
7b. URI-based task references
