# Plan

## What Is the Solution (VERIFIED APPROACH)

**Modularize DSL Engine**: Break `dsl_executor.rs` (2,362 lines) into focused modules.

### Phase 1: Create Directory Structure
```bash
mkdir -p "src/task/dsl"
# Create mod.rs for the new dsl module
```

### Phase 2: Extract Components

1. **`dsl/cache.rs`** (~140 lines)
   - Move `SelectorCacheEntry`, `SelectorCache`, `CacheStats`
   - Keep LRU eviction logic, TTL handling
   - Export: `SelectorCache`, `CacheStats`

2. **`dsl/evaluator.rs`** (~250 lines)
   - Move variable substitution logic
   - Move condition evaluation (`evaluate_condition`)
   - Export: variable substitution functions, `evaluate_condition`

3. **`dsl/executor.rs`** (~300 lines)
   - Move `DslExecutor` struct (keep in main file or move)
   - Keep `execute()`, `execute_action()` 
   - Move action dispatch logic
   - Export: `DslExecutor`, main execution methods

4. **`dsl/control_flow.rs`** (~200 lines)
   - Move `If`/`Else` execution
   - Move `Loop`, `Foreach`, `While` handling
   - Move `Call`, `Parallel`, `Retry` execution
   - Export: control flow handlers

5. **`dsl/debug.rs`** (~150 lines)
   - Move `DebugEventType`, `DebugEvent`, `Breakpoint`
   - Move debug/tracing infrastructure
   - Export: debug types and functions

6. **`dsl/profiling.rs`** (~50 lines)
   - Move `ActionProfiler`, `ActionMetrics`, `ExecutionReport`
   - Export: profiling types

### Phase 3: Update Module Declarations

**`src/task/mod.rs`**:
```rust
pub mod dsl {
    pub mod cache;
    pub mod evaluator;
    pub mod executor;
    pub mod control_flow;
    pub mod debug;
    pub mod profiling;
    // Re-export main executor
    pub use executor::DslExecutor;
}
```

### Phase 4: Wire Up and Test

1. Update all `use` statements in moved code
2. Ensure `DslExecutor` is accessible as `crate::task::dsl::DslExecutor`
3. Run `cargo test` after each extraction
4. Update `dsl.rs` to re-export types if needed

# Internal API Outline

### dsl/cache.rs
```rust
pub struct SelectorCacheEntry { /* ... */ }
pub struct SelectorCache { /* ... */ }
pub struct CacheStats { /* ... */ }
impl SelectorCache {
    pub fn new() -> Self { /* ... */ }
    pub fn get(&mut self, selector: &str) -> Option<&SelectorCacheEntry> { /* ... */ }
    pub fn insert(&mut self, selector: String, entry: SelectorCacheEntry) { /* ... */ }
    pub fn stats(&self) -> CacheStats { /* ... */ }
}
```

### dsl/evaluator.rs
```rust
pub fn substitute_variables(input: &str, variables: &HashMap<String, String>) -> String { /* ... */ }
pub async fn evaluate_condition(/* ... */) -> Result<bool> { /* ... */ }
```

### dsl/executor.rs
```rust
pub struct DslExecutor<'a> { /* ... */ }
impl<'a> DslExecutor<'a> {
    pub fn new(api: &'a TaskContext, task_def: &'a TaskDefinition) -> Self { /* ... */ }
    pub async fn execute(&mut self) -> Result<()> { /* ... */ }
}
```

### dsl/control_flow.rs
```rust
pub async fn execute_if(/* ... */) -> Result<()> { /* ... */ }
pub async fn execute_loop(/* ... */) -> Result<()> { /* ... */ }
pub async fn execute_foreach(/* ... */) -> Result<()> { /* ... */ }
```

# Decisions

## Decision 1: Extraction Order
**Status**: Recommended

**Approach**: Extract in dependency order:
1. `cache.rs` (no dependencies on other extractions)
2. `debug.rs` (independent)
3. `profiling.rs` (independent)
4. `evaluator.rs` (depends on cache maybe)
5. `control_flow.rs` (depends on evaluator)
6. `executor.rs` (depends on all above)

**Why**: Minimizes compilation errors during refactoring.

## Decision 2: Keep DslExecutor in Main File or Move?
**Status**: Decision needed

**Option A**: Keep `DslExecutor` in `dsl_executor.rs` (original file becomes thin wrapper)
- Pros: Less code movement, easier git history
- Cons: Original file still exists (confusing)

**Option B**: Move `DslExecutor` to `dsl/executor.rs`
- Pros: Clean modularization
- Cons: More file movements

**Recommendation**: Option B for full modularization.

## Verification & Testing

- **Incremental Testing**: Run `cargo test` after each extraction
- **Cache Preservation**: Ensure `SelectorCache` behavior identical
- **DSL Compatibility**: Run existing DSL task tests
- **Performance**: Benchmark before/after to ensure no regression
- **Debug Features**: Verify breakpoints, tracing still work

# Notes

**Unlike specs 0023/0024**, this spec identifies a **real problem**:
- File is genuinely 2,362 lines (measurements off by ~240, but problem exists)
- Structure claims are accurate
- Plan is reasonable and actionable

**Proceed with this spec** - it has merit.
