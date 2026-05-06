# Plan

## What Is the Solution
1. **Modularize DSL Engine**: Break `dsl_executor.rs` into a `dsl/` directory.
2. **Subcomponents**:
   - `dsl/cache.rs` (for the LRU selector cache)
   - `dsl/evaluator.rs` (for condition and variable substitution)
   - `dsl/executor.rs` (just the action dispatch loop)
   - `dsl/control_flow.rs` (handling loops and conditionals)

# internal api outline

Implementation details to be defined during active development.

# decisions

Implementation details to be defined during active development.

