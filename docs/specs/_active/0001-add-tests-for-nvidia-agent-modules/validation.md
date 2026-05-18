# Validation

## Validation Criteria — All Passed ✅

1. ✅ `cargo check` — passes without errors
2. ✅ `cargo test --lib bacon_agent_nvidia::auditor::tests` — **6/6 passed**
3. ✅ `cargo test --lib bacon_agent_nvidia::observer::tests` — **3/3 passed**
4. ✅ `cargo test --lib bacon_agent_nvidia::pipeline::tests` — **8/8 passed**
5. ✅ `cargo test` (full suite) — all existing + new tests pass

## Final Test Counts

| Module | Tests Added |
|---|---|
| `auditor.rs` | 6 |
| `observer.rs` | 3 |
| `pipeline.rs` | 8 |
| **Total** | **17** |

## Test Details

### auditor.rs (6 tests)
- `test_role_prompt_returns_content` — verifies non-empty + contains "auditor"
- `test_role_prompt_not_empty` — simple non-empty check
- `test_promote_to_done_fails_when_spec_lint_unavailable` — temp spec dir, expects spec-lint failure
- `test_mark_needs_approval_writes_validation` — temp dir, verifies spec.yaml status + validation.md creation
- `test_mark_needs_approval_appends_to_existing_validation` — temp dir with pre-existing validation.md, checks report prepended
- `test_mark_needs_approval_creates_spec_dir_if_missing` — non-existent path, expects Err

### observer.rs (3 tests)
- `test_system_prompt_returns_content` — non-empty + contains "observer"
- `test_system_prompt_not_empty` — simple non-empty check
- `test_system_prompt_contains_role_instructions` — length check (> 100 chars)

### pipeline.rs (8 tests)
- `test_pipeline_name_is_nvidia` — name() returns "nvidia"
- `test_pipeline_dry_run_flag` — dry_run() returns true
- `test_pipeline_not_dry_run_when_flag_false` — dry_run() returns false
- `test_pipeline_auto_flag` — auto() returns true
- `test_pipeline_fast_flag` — fast() returns true
- `test_pipeline_resume_stage_is_none_by_default` — resume_stage() is None
- `test_pipeline_cfg_loaded_from_real_toml` — all 4 agent strings non-empty
- `test_pipeline_cfg_agent_for_maps_stages` — agent_for() maps correctly
