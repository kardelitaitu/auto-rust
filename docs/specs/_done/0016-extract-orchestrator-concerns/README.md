# Extract Orchestrator Concerns

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Extract distinct concerns from the 1623-line monolith `src/orchestrator.rs` into focused submodules under `src/orchestrator/`. The orchestrator is the core task execution engine — every CLI task flow depends on it. Currently it bundles 5 structs, 6+ async functions, helpers, and 756 lines of tests in a single file.

## Scope

Extract into submodules without behavioral changes:
- `execution.rs` — `execute_group`, `execute_group_with_cancel`, `execute_task_on_session`
- `guards.rs` — `GlobalExecutionSlot`, `SessionExecutionGuard`, `acquire_global_execution_slot`
- `retry.rs` — `execute_task_with_retry`, `TaskAttemptFailure`
- `health.rs` — `should_mark_session_unhealthy`, `format_duration`, `broadcast_execution_count`
- `test_utils.rs` — shared test helpers (`create_test_config`, `connect_test_session`)
- `mod.rs` — `Orchestrator` struct, `new()`, public re-exports

## Next Steps

1. Implementer reads `baseline.md` and `plan.md`
2. Extract each concern into its target submodule
3. Verify `cargo check && cargo test --lib orchestrator`
4. Run `cargo clippy --all-targets --all-features`
5. Archive spec to `_done/` after auditor approval
