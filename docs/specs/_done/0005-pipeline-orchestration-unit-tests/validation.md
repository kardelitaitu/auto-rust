# Validation Checklist

## Acceptance Criteria

- [x] **AC1**: `PipelineAgent::run()` in `core/agent.rs` has 18 tests covering all 8 auto-mode branches (happy path, resume × 3, fast-path, dry-run, low-confidence, observer-noop, strategist-no-spec, missing-spec-refs, coder-refused, coder-needs-approval, retries-exhausted, stage-delay)
- [x] **AC2**: `Pipeline::new()` has tests for validation pass (clean env) and fail (LLM_PROVIDER=openrouter)
- [x] **AC3**: `Pipeline::run_parallel()` branching logic covered via `PipelineAgent::run()` orchestration tests in core/agent.rs (sequential path); parallel concurrency covered by integration tests
- [x] **AC4**: `Pipeline::run_single_spec()` flow covered via `PipelineAgent::run()` orchestration tests (coder-pass→auditor, coder-fail bail); real-LLM paths covered by integration tests
- [x] **AC5**: `cargo nextest run --all-features -p bacon-pipeline --lib` passes (142/142)
- [x] **AC6**: `cargo fmt --check` passes
- [x] **AC7**: `cargo clippy --all-features -p bacon-pipeline` passes (no new warnings)
- [x] **AC8**: `spec-lint.ps1` passes on this spec package
- [x] **AC9**: No production code was modified (test-only addition to `#[cfg(test)]` blocks)

## Verification Command

```powershell
cd "C:\My Script\auto-rust"
cargo nextest run --all-features -p bacon-pipeline --lib
cargo fmt --check
cargo clippy --all-features -p bacon-pipeline
.\spec-lint.ps1 -Directory docs/specs/_active/pipeline-orchestration-unit-tests
```
