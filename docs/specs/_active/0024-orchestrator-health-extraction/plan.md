# Plan

## What Is the Solution
1. **Extract Health Monitor**: Move `should_mark_session_unhealthy` out of the orchestrator and into a dedicated `src/session/health.rs` module. It should be refactored into smaller, pure functions.
2. **Extract Execution Policy**: Move `execute_task_with_retry` into a `task_runner.rs` component to separate task execution from global orchestration.

# internal api outline

Implementation details to be defined during active development.

# decisions

Implementation details to be defined during active development.

