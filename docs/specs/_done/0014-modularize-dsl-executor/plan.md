# Modularize dsl/executor.rs into Domain-Specific Submodules

## Baseline

### What I Find
1. `src/task/dsl/executor.rs` is **4736 lines** — the largest file in the codebase (next largest: task_context/mod.rs at 2903)
2. It contains **198 `fn` definitions** (including test functions), making navigation and review difficult
3. It has **149 `unwrap()` calls**, concentrated in helper methods that could benefit from Result propagation
4. The `dsl/` directory already has **8 established submodules** (types, cache, debug, profiling, evaluator, control_flow, parser, api) — the split pattern is proven
5. `executor.rs` currently handles: action dispatch, element interaction (click/type/hover), page navigation, screenshot capture, JS execution, logging, and task call orchestration — mixing browser, wait, inspection, and media concerns
6. **No http_get, clipboard, cookie, or file I/O methods exist** in executor.rs — the original spec's proposed `network.rs`, `io.rs`, and `interact.rs` modules are not needed
7. **3872 of 4736 lines are `#[cfg(test)]` tests** — moving tests inline with extracted handlers is essential to reduce executor.rs size

### What I Claim
Splitting executor.rs into focused submodules will reduce cognitive load, improve compile times via parallel compilation, make action handlers independently testable, and establish clear ownership boundaries — without changing any runtime behavior.

### What Is the Proof
1. **File size**: 4736 lines is 63% larger than the next biggest file (task_context/mod.rs at 2903)
2. **Established pattern**: The `dsl/` directory already hosts 8 focused submodules; `actions/` follows the same pattern used by `utils/twitter/`
3. **Cross-cutting concerns**: Action dispatch for browser actions (click/type/navigate) lives alongside screenshots, JS execution, and wait timers — each should be its own module
4. **Test co-location**: 52 test functions in executor.rs all test specific action handlers — each belongs next to the handler it tests

## What Is the Solution

### High-Level Plan
Extract action handler implementations and their tests from executor.rs into focused submodules under `actions/`:

| Submodule | Handlers extracted | Est. lines saved |
|-----------|-------------------|------------------|
| `actions/browser.rs` | navigate, click, type, hover, select, scroll_to, right_click, double_click, clear + tests | ~1500 |
| `actions/wait.rs` | wait, wait_for + tests | ~500 |
| `actions/inspection.rs` | extract + tests | ~300 |
| `actions/media.rs` | screenshot + tests | ~300 |
| **Total** | **16 handlers → 4 modules** | **~2600** |

### Files to Change
- **`src/task/dsl/executor.rs`** — Remove handler functions and their tests; keep struct def, constructors, execute(), execute_action() dispatch, execute_call(), cache methods, debug/profiling methods, and dispatch-level tests (~1000 lines remaining)
- **`src/task/dsl/mod.rs`** — Add `pub mod actions;` declaration
- **New**: `src/task/dsl/actions/mod.rs` — Declare submodules, re-export for backward compatibility
- **New**: `src/task/dsl/actions/browser.rs` — Browser interaction handlers + inline tests
- **New**: `src/task/dsl/actions/wait.rs` — Wait/pause handlers + inline tests
- **New**: `src/task/dsl/actions/inspection.rs` — Element inspection handlers + inline tests
- **New**: `src/task/dsl/actions/media.rs` — Screenshot handler + inline tests

### Extraction Pattern

Each extracted handler module follows this pattern:

```rust
//! <category> actions for DSL executor.

use crate::task::dsl::executor::DslExecutor;
use crate::task::dsl::api::DslApi;
use anyhow::Result;

impl<'a, T: DslApi> DslExecutor<'a, T> {
    pub(super) async fn execute_<action>(&mut self, ...) -> Result<()> {
        // ... handler body
    }
}

#[cfg(test)]
mod tests {
    // ... tests moved from executor.rs
}
```

The `pub(super)` visibility keeps the dispatch in `executor.rs` while allowing extraction. The `impl` block in each submodule augments `DslExecutor` with the handler method — this works because Rust allows `impl` blocks for a struct to span multiple files within the same crate.

### What Stays in executor.rs
- Module-level docs, imports, struct definition, constants
- `new()`, `with_depth()`, `with_parameters()` constructors
- `execute()` orchestration loop
- `execute_action()` dispatch match (calls `self.execute_*` from extracted modules)
- `execute_call()` sub-task invocation
- Cache methods: `cached_exists`, `cached_visible`, `cached_text`, `invalidate_cache`, `enable_caching`, `disable_caching`, `get_cache_stats`, `get_profiler_stats`, `clear_cache`, `cache_size`, `set_cache_ttl`, `get_cache_ttl`
- Debug/profiling methods: `record_profile`, `watch_variable`, `record_debug_event`, `check_breakpoints`
- `substitute_variables()` helper (used by many handlers — stays for shared access)
- Dispatch-level tests that test `execute_action()` routing or `execute_call()`

### Implementation Steps
1. Create `src/task/dsl/actions/` directory and `actions/mod.rs` with submodule declarations
2. Create `actions/browser.rs` — extract navigate, click, type, hover, select, scroll_to, right_click, double_click, clear + their tests
3. Create `actions/wait.rs` — extract wait, wait_for + their tests
4. Create `actions/inspection.rs` — extract extract + its tests
5. Create `actions/media.rs` — extract screenshot + its tests
6. Update `dsl/mod.rs` to declare `pub mod actions;`
7. Remove extracted code and test functions from executor.rs
8. Run `cargo check` after each step

## API Changes
No public API changes. All function signatures are preserved. Extracted handlers use `pub(super)` visibility. Re-exports in `dsl/mod.rs` are unchanged.

## Design Decisions and Risks
- **Why `pub(super)` on extracted handlers?** The dispatch in execute_action() calls `self.execute_navigate(...)` etc. — these must be visible within the `dsl` module but not outside it. `pub(super)` on `impl DslExecutor` methods achieves exactly this.
- **Why not extract cache/debug methods?** They are tightly coupled to DslExecutor's fields (selector_cache, action_profilers, debug_events, watched_variables). Extracting them would require passing &mut self to helper structs, adding complexity with minimal line savings (~200 lines).
- **Why leave execute_call in executor.rs?** It's a single ~70-line function that's tightly coupled to the execute() orchestration flow. Extracting it to its own module would add more boilerplate than it saves.
- **Risk — Cross-module field access**: Extracted `impl` blocks in submodules access `self.api`, `self.cache_ttl`, `self.cache_enabled`, `self.selector_cache`, `self.variables` directly. Since they're `impl` blocks for the same struct, this works without any accessor methods.
- **Risk — stub_warnings**: After extraction, `cached_visible` and `cached_text` are `#[allow(dead_code)]` in executor.rs and their tests are gone. They may produce dead_code warnings. Acceptable.
- **Confidence**: **High**

## Validation
- `cargo check` passes with no dead_code warnings
- `cargo test` passes all 3466+ existing tests
- `cargo clippy --all-targets --all-features` shows no new warnings
- `cargo fmt --check` passes
- Manual review: executor.rs non-test code ≤500 lines, each new module has a clear doc comment
- Count: `rg "fn " src/task/dsl/executor.rs | wc -l` shows ≤80 after extraction (down from 198)
