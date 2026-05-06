# Plan

## What Is the Solution
**Extract Tests**: Move the inline tests to a dedicated `tests/runtime_integration_tests.rs` or a private submodule like `src/runtime/tests/task_context_tests.rs` using `mod tests;` to keep the core file lean.
