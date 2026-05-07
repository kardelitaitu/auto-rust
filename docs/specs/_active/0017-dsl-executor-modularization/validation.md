# Validation Checklist

## Pre-Implementation Checks
- [ ] Run `cargo test` - establish baseline (ALL tests must pass)
- [ ] Run `cargo check` - verify clean compilation
- [ ] Run `cargo clippy` - no warnings initially
- [ ] Count lines: `dsl_executor.rs` = 2,362 lines (source of truth)
- [ ] Identify all consumers: `Select-String -Path "src/**/*.rs" -Pattern "use.*dsl_executor|DslExecutor"`

## During Implementation (CRITICAL: Run After Each Extraction)

### After Extracting `dsl/cache.rs` (Phase 2.1)
- [ ] `cargo check` passes
- [ ] `SelectorCache` accessible from `dsl::cache`
- [ ] Cache behavior identical (LRU eviction, TTL)
- [ ] Hit/miss counters work
- [ ] Run `cargo test --lib dsl` for cache-related tests

### After Extracting `dsl/debug.rs` (Phase 2.2)
- [ ] `cargo check` passes
- [ ] `DebugEvent`, `Breakpoint` accessible
- [ ] Debug mode still works (if you have integration tests)
- [ ] Breakpoint triggering logic preserved

### After Extracting `dsl/profiling.rs` (Phase 2.3)
- [ ] `cargo check` passes
- [ ] `ActionMetrics`, `ExecutionReport` accessible
- [ ] Profiling data collection works
- [ ] JSON export works

### After Extracting `dsl/evaluator.rs` (Phase 2.4)
- [ ] `cargo check` passes
- [ ] Variable substitution works (`${variable}` syntax)
- [ ] Condition evaluation works (`if/else` blocks)
- [ ] Run DSL task with variables - verify behavior

### After Extracting `dsl/control_flow.rs` (Phase 2.5)
- [ ] `cargo check` passes
- [ ] `if/else` execution works
- [ ] `loop` with counter works
- [ ] `foreach` with array/range/elements works
- [ ] `while` with condition works
- [ ] `retry` with backoff works
- [ ] `parallel` execution works

### After Extracting `dsl/executor.rs` (Phase 2.6)
- [ ] `cargo check` passes
- [ ] `DslExecutor` struct accessible
- [ ] `execute()` method works
- [ ] All action types dispatch correctly
- [ ] Cache integration works (calls to `dsl::cache`)

## Post-Implementation Verification

### Line Count Verification
- [ ] `dsl_executor.rs` is GONE or reduced to <500 lines (was 2,362)
- [ ] `dsl/cache.rs` ≈ 140 lines
- [ ] `dsl/debug.rs` ≈ 150 lines
- [ ] `dsl/profiling.rs` ≈ 55 lines
- [ ] `dsl/evaluator.rs` ≈ 200 lines
- [ ] `dsl/control_flow.rs` ≈ 300 lines
- [ ] `dsl/executor.rs` ≈ 300 lines
- [ ] Total lines in `dsl/` directory ≈ 1,145 lines (less than original 2,362 due to removed tests)

### Functional Verification
- [ ] Run `cargo test` - ALL 2242+ tests pass
- [ ] Run `cargo test --test dsl_*` - all DSL tests pass
- [ ] Run `cargo test --lib` - all unit tests pass
- [ ] Run `.\check.ps1` - FULL CI GATE PASSES

### Performance Verification
- [ ] Cache hit rate similar (run existing DSL tasks)
- [ ] Execution time similar (no regression)
- [ ] Memory usage stable (no leaks from refactoring)

### API Compatibility
- [ ] `DslExecutor` accessible via `crate::task::dsl::executor::DslExecutor`
- [ ] OR `crate::task::dsl::DslExecutor` (if re-exported)
- [ ] All public types accessible (`CacheStats`, `ActionMetrics`, etc.)
- [ ] No breaking changes to task definitions

### Code Quality
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` applied to all new files
- [ ] Rustdoc comments added to all new public APIs
- [ ] No `dead_code` warnings for moved items

## Behavioral Verification (Critical)

### Cache Functionality
- [ ] LRU eviction works (insert >100 items, verify oldest evicted)
- [ ] TTL expiration works (wait 6 seconds, verify entry invalid)
- [ ] `invalidate()` clears specific selector
- [ ] `clear()` clears all entries
- [ ] Hit/miss counters accurate

### Control Flow
- [ ] `if/else` with true condition executes `then` block
- [ ] `if/else` with false condition executes `else` block (if present)
- [ ] `loop` with count executes exactly N times
- [ ] `loop` with condition evaluates correctly
- [ ] `foreach` with array iterates all items
- [ ] `foreach` with range iterates correctly
- [ ] `while` loops until condition false
- [ ] `retry` attempts up to max_attempts
- [ ] `retry` stops on success
- [ ] `parallel` executes all actions

### Debug Features
- [ ] Breakpoints trigger at correct action index
- [ ] Breakpoints trigger at correct action type
- [ ] Variable watch notifies on change
- [ ] Debug events logged correctly
- [ ] Pause/resume works

## Integration Testing
- [ ] Run existing DSL task YAML files
- [ ] Verify task execution produces same results
- [ ] Check execution reports are identical
- [ ] Profile data matches before/after

# CI Commands

```bash
# Full validation gate (MUST PASS before commit)
cd "C:\My Script\auto-rust"
.\check.ps1

# Individual checks (run after each phase)
cd "C:\My Script\auto-rust"
cargo check                    # Quick compilation check
cargo test --lib dsl          # DSL-specific tests
cargo test                   # ALL tests
cargo clippy -- -D warnings   # Lint check
cargo fmt --all -- --check    # Format check

# Line count verification
(Get-Content "src/task/dsl_executor.rs" | Measure-Object -Line).Lines
(Get-Content "src/task/dsl/*.rs" | Measure-Object -Line).Lines
```

# Quality Rules

1. **No logic changes**: Refactoring ONLY - behavior must be identical
2. **Preserve tests**: All 2242+ existing tests must pass without modification
3. **Document new modules**: Add rustdoc for all extracted modules
4. **Maintain performance**: Cache hit rate and execution time must not regress
5. **Keep public API stable**: `DslExecutor` must remain accessible at same path (or provide re-export)
6. **Incremental verification**: Run `cargo check` and `cargo test` after EACH extraction
7. **No dead code**: Remove original `dsl_executor.rs` after extraction complete
8. **Proper module structure**: Use `mod.rs` or proper `mod` declarations
