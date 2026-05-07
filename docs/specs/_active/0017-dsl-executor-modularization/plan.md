# Plan

## What Is the Solution (DETAILED IMPLEMENTATION)

**Modularize DSL Engine**: Break `dsl_executor.rs` (2,362 lines) into focused modules.

### Phase 1: Create Directory Structure
```bash
cd "C:\My Script\auto-rust"
mkdir -p "src/task/dsl"
# Create src/task/dsl/mod.rs
```

### Phase 2: Extract Components (In Dependency Order)

#### 2.1 `dsl/cache.rs` (~140 lines: Lines 37-175)
**Extract:**
- `SelectorCacheEntry` struct (lines 37-73)
- `SelectorCache` struct (lines 75-175)
- `CacheStats` struct (lines 176-190)

**Export:**
```rust
pub struct SelectorCacheEntry { /* ... */ }
pub struct SelectorCache { /* ... */ }
pub struct CacheStats { /* ... */ }
impl SelectorCache {
    pub fn new() -> Self { /* ... */ }
    pub fn get(&mut self, selector: &str) -> Option<&SelectorCacheEntry> { /* ... */ }
    pub fn insert(&mut self, selector: String, entry: SelectorCacheEntry) { /* ... */ }
    pub fn invalidate(&mut self, selector: &str) { /* ... */ }
    pub fn clear(&mut self) { /* ... */ }
    pub fn stats(&self) -> CacheStats { /* ... */ }
}
```

#### 2.2 `dsl/debug.rs` (~150 lines: Lines 247-385)
**Extract:**
- `DebugEventType` enum (lines 247-266)
- `DebugEvent` struct (lines 268-285)
- `Breakpoint` struct (lines 288-385)

**Export:**
```rust
pub enum DebugEventType { /* ... */ }
pub struct DebugEvent { /* ... */ }
pub struct Breakpoint { /* ... */ }
impl Breakpoint {
    pub fn on_action(index: usize) -> Self { /* ... */ }
    pub fn on_action_type(action_type: impl Into<String>) -> Self { /* ... */ }
    pub fn watch_variable(name: impl Into<String>) -> Self { /* ... */ }
    pub fn should_trigger(&self, /* ... */) -> bool { /* ... */ }
}
```

#### 2.3 `dsl/profiling.rs` (~55 lines: Lines 191-235)
**Extract:**
- `ActionProfiler` struct (lines 191-235)

**Export:**
```rust
pub struct ActionProfiler { /* ... */ }
pub struct ActionMetrics { /* ... */ }
pub struct ExecutionReport { /* ... */ }
impl ActionProfiler {
    pub fn record(&mut self, duration: Duration, success: bool) { /* ... */ }
    pub fn average_duration(&self) -> Option<Duration> { /* ... */ }
}
impl ActionMetrics { /* ... */ }
impl ExecutionReport { /* ... */ }
```

#### 2.4 `dsl/evaluator.rs` (~200 lines: scattered variable/condition logic)
**Extract:**
- Variable substitution logic (search for `substitute_variables`)
- Condition evaluation (search for `evaluate_condition`)
- Move helper functions from `execute_action` related to evaluation

**Export:**
```rust
pub fn substitute_variables(input: &str, variables: &HashMap<String, String>) -> String { /* ... */ }
pub async fn evaluate_condition(/* ... */) -> Result<bool> { /* ... */ }
```

#### 2.5 `dsl/control_flow.rs` (~300 lines: If/Loop/Foreach/While/Retry/Parallel)
**Extract:**
- `Action::If` execution
- `Action::Loop` execution  
- `Action::Foreach` execution
- `Action::While` execution
- `Action::Retry` execution
- `Action::Parallel` execution

**Export:**
```rust
pub async fn execute_if(/* ... */) -> Result<()> { /* ... */ }
pub async fn execute_loop(/* ... */) -> Result<()> { /* ... */ }
pub async fn execute_foreach(/* ... */) -> Result<()> { /* ... */ }
// etc.
```

#### 2.6 `dsl/executor.rs` (~300 lines: DslExecutor struct + execute method)
**Keep/Extract:**
- `DslExecutor` struct (lines 510-722)
- `new()`, `with_depth()`, `execute()`, `execute_action()`
- `cached_exists()`, `cached_visible()`, `cached_text()`

**Export:**
```rust
pub struct DslExecutor<'a> { /* ... */ }
impl<'a> DslExecutor<'a> {
    pub fn new(api: &'a TaskContext, task_def: &'a TaskDefinition) -> Self { /* ... */ }
    pub async fn execute(&mut self) -> Result<()> { /* ... */ }
    // ... other methods
}
```

### Phase 3: Update Module Declarations

**Create `src/task/dsl/mod.rs`:**
```rust
pub mod cache;
pub mod debug;
pub mod profiling;
pub mod evaluator;
pub mod control_flow;
pub mod executor;

// Re-exports for backward compatibility
pub use executor::DslExecutor;
pub use debug::{DebugEvent, DebugEventType, Breakpoint};
pub use profiling::{ActionMetrics, ExecutionReport, ActionProfiler};
pub use cache::{SelectorCache, CacheStats};
```

**Update `src/task/mod.rs`:**
```rust
pub mod dsl {
    pub mod dsl;  // Original dsl.rs with Action/Condition/TaskDefinition
    pub mod executor;  // New modular executor
}
```

### Phase 4: Wire Up and Test

1. Update all `use` statements in moved code
2. Ensure `DslExecutor` is accessible as `crate::task::dsl::executor::DslExecutor`
3. Run `cargo test` after each extraction
4. Update `dsl.rs` to re-export `DslExecutor` for backward compatibility

# Internal API Outline

### dsl/cache.rs
```rust
pub struct SelectorCacheEntry {
    pub exists: bool,
    pub visible: bool,
    pub text: Option<String>,
    pub count: usize,
    pub cached_at: Instant,
    pub ttl: Duration,
}
impl SelectorCacheEntry {
    pub fn new(exists: bool, visible: bool, text: Option<String>, count: usize) -> Self;
    pub fn is_valid(&self) -> bool;
}
// ... other methods
```

### dsl/evaluator.rs
```rust
impl DslExecutor<'_> {
    pub fn substitute_variables(&self, input: &str) -> String;
    pub async fn evaluate_condition(&mut self, condition: &Condition) -> Result<bool>;
}
```

### dsl/executor.rs
```rust
impl<'a> DslExecutor<'a> {
    pub fn new(api: &'a TaskContext, task_def: &'a TaskDefinition) -> Self;
    pub async fn execute(&mut self) -> Result<()>;
    async fn execute_action(&mut self, action: &Action) -> Result<()>;
    async fn cached_exists(&mut self, selector: &str) -> Result<bool>;
}
```

# Decisions

## Decision 1: Extraction Order
**Status**: Confirmed

**Order** (to minimize compilation errors):
1. `cache.rs` (no dependencies)
2. `debug.rs` (independent)
3. `profiling.rs` (independent)
4. `evaluator.rs` (depends on cache maybe)
5. `control_flow.rs` (depends on evaluator)
6. `executor.rs` (depends on all above)

**Why**: Minimizes compilation errors during refactoring.

## Decision 2: Keep DslExecutor in Main File or Move?
**Status**: Decision made

**Choice**: Move `DslExecutor` to `dsl/executor.rs`

**Pros**: Clean modularization, file becomes truly focused
**Cons**: More file movements

**Reasoning**: Since we're already breaking this file apart, let's do it properly.

## Decision 3: Backward Compatibility
**Status**: Decision needed

**Option A**: Re-export everything from `dsl.rs`
- Pros: Existing code continues to work
- Cons: Defeats the purpose of modularization

**Option B**: Update all imports in codebase
- Pros: Clean, explicit dependencies
- Cons: More files to update

**Recommendation**: Option B for clarity, but provide re-exports during transition.

# Verification & Testing

## Pre-Implementation
- [ ] Run `cargo test` to establish baseline (should pass)
- [ ] Run `cargo check` to verify current state
- [ ] Document all public API surfaces in `dsl_executor.rs`
- [ ] Identify all files that import `DslExecutor`

## During Implementation (After Each Extraction)
- [ ] Run `cargo check` immediately
- [ ] Run `cargo test` for DSL-related tests
- [ ] Verify `DslExecutor` still accessible
- [ ] Check that cache behavior hasn't changed
- [ ] Verify debug/tracing features work

## Post-Implementation
- [ ] Run full `.\check.ps1` CI gate
- [ ] Verify `dsl_executor.rs` reduced from 2,362 to <500 lines
- [ ] All DSL task tests pass
- [ ] Performance benchmark (cache hit rate, execution time)
- [ ] Documentation updated (rustdoc for new modules)

## Test Files to Verify
```bash
# Find all DSL tests
cd "C:\My Script\auto-rust"
Select-String -Path "src/**/*.rs" -Pattern "DslExecutor|dsl_executor" | Select-Object Filename | Get-Unique
```
