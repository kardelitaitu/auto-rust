# Implementation Notes

## Session Date: 2026-05-07

### Codebase Analysis Results

#### Source File Structure (VERIFIED)
**`src/task/dsl_executor.rs`** - 2,362 lines total:

| Lines | Component | Description |
|-------|-----------|-------------|
| 1-36 | Imports | Standard library, external crates |
| 37-73 | `SelectorCacheEntry` | LRU cache entry with TTL (37 lines) |
| 75-175 | `SelectorCache` | LRU cache with eviction (101 lines) |
| 176-190 | `CacheStats` | Cache statistics (15 lines) |
| 191-235 | `ActionProfiler` | Performance profiling (45 lines) |
| 238-266 | `DebugEventType` | Debug event types (29 lines) |
| 268-285 | `DebugEvent` | Debug event struct (18 lines) |
| 288-385 | `Breakpoint` | Breakpoint with conditions (98 lines) |
| 389-438 | `ActionMetrics` | Per-action metrics (50 lines) |
| 440-505 | `ExecutionReport` | Full execution report (66 lines) |
| 510-722 | `DslExecutor` struct | Main executor with all fields (213 lines) |
| 724-1952 | `impl DslExecutor` | Methods: `new`, `execute`, `execute_action`, variable substitution, condition evaluation (1229 lines) |
| 1953-2362 | Tests | Unit tests (~410 lines) |

**Total**: 2,362 lines (genuinely large file)

#### Dependencies Between Components

1. **`SelectorCache`** - No dependencies (leaf module)
2. **`DebugEvent`/`Breakpoint`** - No dependencies (leaf module)
3. **`ActionProfiler`/`ActionMetrics`** - No dependencies (leaf module)
4. **`DslExecutor`** - Depends on ALL above + variable substitution + control flow
5. **Variable substitution** - Uses `SelectorCache` for cached values
6. **Control flow** - Uses variable substitution + condition evaluation

#### Extraction Order (Minimize Compilation Errors)

**Phase 1**: Extract leaf modules (no dependencies)
- `dsl/cache.rs` (SelectorCache, CacheStats)
- `dsl/debug.rs` (DebugEvent, Breakpoint, DebugEventType)
- `dsl/profiling.rs` (ActionProfiler, ActionMetrics, ExecutionReport)

**Phase 2**: Extract mid-level modules
- `dsl/evaluator.rs` (variable substitution, condition evaluation)
  - Depends on: cache (optional)
  
**Phase 3**: Extract control flow
- `dsl/control_flow.rs` (if, loop, foreach, while, retry, parallel)
  - Depends on: evaluator
  
**Phase 4**: Extract executor
- `dsl/executor.rs` (DslExecutor struct + main methods)
  - Depends on: cache, debug, profiling, evaluator, control_flow

### Key Findings

#### 1. Cache Implementation (Lines 37-175)
- Custom LRU implementation (NOT using `lru` crate)
- TTL support: entries expire after 5 seconds
- Tracks: hits, misses, evictions
- Methods: `get()`, `insert()`, `invalidate()`, `clear()`, `stats()`
- **Risk**: Must preserve exact LRU behavior after extraction

#### 2. Debug Infrastructure (Lines 247-385)
- `Breakpoint` supports: action index, action type, variable watch, custom conditions
- `DebugEvent` logs: action start/complete/error, breakpoints, variable changes
- **Risk**: Closure in `Breakpoint::condition` can't be cloned (documented limitation)

#### 3. Execution Logic (Lines 724-1952)
- **Variable substitution**: `${variable}` syntax, resolves from `self.variables`
- **Condition evaluation**: Supports `Exists`, `NotExists`, `Visible`, `NotVisible`, `Equals`, `NotEquals`, `Contains`, `And`, `Or`, `Not`
- **Action dispatch**: 15+ action types (Navigate, Click, Type, Wait, WaitFor, ScrollTo, Extract, Execute, Log, If, Loop, Foreach, While, Call, Screenshot, Clear, Hover, Select, RightClick, DoubleClick, Parallel, Retry)
- **Risk**: Most complex part - must preserve exact behavior

#### 4. Test Coverage (Lines 1953-2362)
- Tests for: cache, profiler, debug events, variable substitution, control flow
- Tests use `#[cfg(test)]` block inside the module
- **Strategy**: Move tests to `tests/dsl/` directory OR keep inline in new modules

### Open Questions

1. **Tests location**: Keep in new modules vs. move to `tests/dsl/`?
   - Option A: Keep inline (easier during refactoring)
   - Option B: Move to `tests/dsl/` (cleaner, but more work)
   - **Recommendation**: Keep inline initially, move later if desired

2. **Re-export strategy**: How to maintain backward compatibility?
   - Option A: `pub use executor::DslExecutor` in `dsl/mod.rs`
   - Option B: Update all imports in codebase
   - **Recommendation**: Option A for smooth transition

3. **Original file**: Delete or keep as thin wrapper?
   - **Recommendation**: Delete after all extractions complete

### Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|-------------|
| Cache behavior changes | High | Run cache-specific tests after extraction |
| Variable substitution breaks | High | Test DSL tasks with `${var}` syntax |
| Control flow incorrect | High | Test if/else, loop, foreach, while |
| Debug features break | Medium | Test breakpoints, variable watch |
| Performance regression | Medium | Benchmark before/after |
| Test failures | High | Run `cargo test` after EACH step |

### Next Steps

1. **Run baseline checks**:
   ```bash
   cd "C:\My Script\auto-rust"
   cargo test > baseline_test_results.txt
   (Get-Content "src/task/dsl_executor.rs" | Measure-Object -Line).Lines
   ```

2. **Create directory structure**:
   ```bash
   mkdir -p "src/task/dsl"
   # Create mod.rs
   ```

3. **Start extraction** (follow order in plan.md):
   - Phase 1: Leaf modules (cache, debug, profiling)
   - Phase 2: Mid-level (evaluator)
   - Phase 3: Control flow
   - Phase 4: Executor

4. **Verify after each**:
   ```bash
   cargo check
   cargo test --lib dsl
   ```

### Progress Tracking

- [ ] Baseline established (tests pass, line counts recorded)
- [ ] Directory structure created
- [ ] `dsl/cache.rs` extracted
- [ ] `dsl/debug.rs` extracted
- [ ] `dsl/profiling.rs` extracted
- [ ] `dsl/evaluator.rs` extracted
- [ ] `dsl/control_flow.rs` extracted
- [ ] `dsl/executor.rs` extracted
- [ ] Original `dsl_executor.rs` deleted
- [ ] Full `.\check.ps1` passes
- [ ] Documentation updated
