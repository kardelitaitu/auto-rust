## Baseline

- `twitteractivity_state.rs` already parses `dry_run_actions`, but it is intended for task execution flow, not a browser-free simulation planner.
- The current engagement path still assumes a live `TaskContext` and browser interactions for feed scanning and candidate processing.
- The task orchestrator in `src/task/twitteractivity.rs` is thin enough to host a separate simulation entry point without moving live logic back into the shell.
- There is no dedicated simulation planner or deterministic log-only tester yet.
- Existing task integration tests cover action tracking, persona selection, sentiment helpers, and entry-point selection, but not a pure simulation preview.
