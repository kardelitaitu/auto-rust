# Plan: Add unit tests for untested NVIDIA agent modules

## Acceptance Criteria
- auditor.rs gains a `#[cfg(test)] mod tests` block with tests covering `role_prompt()`, `promote_to_done()`, and `mark_needs_approval()`
- observer.rs gains a `#[cfg(test)] mod tests` block with tests covering `system_prompt()` and the early-return spec-fast-path / duplicate-skip paths
- pipeline.rs gains a `#[cfg(test)] mod tests` block with tests covering `Pipeline::new()` validation and stage dispatch routing
- All 3 files compile cleanly under `cargo check`
- All existing + new tests pass under `cargo test`
- No production code behavior is changed

## Implementation Steps

### 1. auditor.rs tests (`src/bacon_agent_nvidia/auditor.rs`)

Add a `#[cfg(test)] mod tests` block at the end:

- **`test_role_prompt_returns_content`**: Verify `role_prompt()` returns a non-empty string containing "auditor" or role instructions.
- **`test_role_prompt_not_empty`**: Verify the string length is > 0.
- **`test_promote_to_done_requires_spec_lint`** (fails gracefully): Call `promote_to_done()` on a temp spec directory with no valid plan.md — expect an error since spec-lint.ps1 won't be found or will fail (PowerShell script may not exist on all systems). This tests the spec-lint gate.
- **`test_promote_to_done_expected_failure_spec_lint`**: Same as above but verify the `anyhow::bail!` error message mentions spec-lint.
- **`test_mark_needs_approval_writes_validation`**: Create a temp spec directory with a valid spec.yaml, call `mark_needs_approval()` with a report string — verify that `spec.yaml` status is updated to `needs-human-approval` and `validation.md` contains the report.
- **`test_mark_needs_approval_appends_report`**: Create a temp spec directory with an existing `validation.md`, call `mark_needs_approval()` — verify the report is prepended to existing content.

### 2. observer.rs tests (`src/bacon_agent_nvidia/observer.rs`)

Add a `#[cfg(test)] mod tests` block at the end:

- **`test_system_prompt_returns_content`**: Verify `system_prompt()` returns a non-empty string containing "observer" or role instructions.
- **`test_system_prompt_not_empty`**: Verify the string length is > 0.

### 3. pipeline.rs tests (`src/bacon_agent_nvidia/pipeline.rs`)

Add a `#[cfg(test)] mod tests` block at the end:

- **`test_pipeline_new_creates_instance`**: Create a Pipeline with `Pipeline::new(...)` using appropriate args — verify it returns Ok with valid fields. Note: this depends on the current `bacon.toml` existing.
- **`test_pipeline_name_is_nvidia`**: Verify `pipe.name()` returns "nvidia".
- **`test_pipeline_dry_run_flag`**: Verify `pipe.dry_run()` matches the flag passed.
- **`test_pipeline_auto_flag`**: Verify `pipe.auto()` matches the flag passed.
- **`test_pipeline_stage_routes_to_bacon_or_nvidia`**: Verify `run_observer` delegates to the correct agent based on pipeline_cfg. This is structural — the actual dispatch test can verify agent strings from config match expectations.

## Risks and Mitigations
- `spec-lint.ps1` may not be in PATH — tests should handle this gracefully and not block CI
- `promote_to_done()` reads/writes filesystem state — all such tests must use `tempfile::tempdir()`
- `Pipeline::new()` calls `validate_bacon_local_only()` which reads `bacon.toml` from `CARGO_MANIFEST_DIR` — this will work in the test environment since `CARGO_MANIFEST_DIR` is set correctly but may fail if the local TOML has odd content
