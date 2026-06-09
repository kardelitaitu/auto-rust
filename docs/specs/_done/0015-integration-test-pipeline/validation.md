## Acceptance Criteria
1. `tests/common/mod.rs` exports `connect_test_browser()` (not duplicated) — ✅
2. `tests/navigation_integration.rs` imports `connect_test_browser` from `common` (not local) — ✅
3. `tests/task_context_integration.rs` imports `connect_test_browser` from `common` (not local) — ✅
4. `tests/orchestrator_integration.rs` — NOT modified (uses `discover_browsers()` which is architecturally correct for `Vec<Session>`-based tests; `connect_test_browser()` returns a single `Browser`, not `Session`)
5. `scripts/run-integration-tests.ps1` exists and is executable — ✅
6. Script launches browser and runs navigation + task_context integrations — ✅
7. `cargo test` (without --ignored) still passes all non-ignored tests — ✅

## Test Commands
- `cargo test` (verify non-ignored tests still pass)
- `.\scripts\run-integration-tests.ps1` (verify navigation + task_context tests)
- `.\scripts\run-integration-tests.ps1 -IncludeOrchestrator` (include orchestrator tests with configured profiles)
- `cargo check --lib` (verify compilation)
- `cargo clippy --lib` (verify no new warnings)

## Visual Inspection
- `tests/common/mod.rs` contains `connect_test_browser()` — one definition, not duplicated
- Both navigation and task_context integration test files import from `common` module
- `scripts/run-integration-tests.ps1` has clear sections: detect → launch → run → cleanup
- Orchestrator tests left as-is; they require `Vec<Session>` from discover_browsers()
