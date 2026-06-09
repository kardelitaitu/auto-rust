# Pipeline orchestration unit tests

Status: `done`

Owner: `spec-agent`
Implementer: `implementation-agent`

## Summary

Add comprehensive unit tests for the two untested orchestration modules in `bacon-pipeline`:
`core/agent.rs` (the `PipelineAgent::run()` default method, 242 lines) and `agent/pipeline.rs`
(the `Pipeline` struct, its stage implementations, `run_parallel()`, and `run_single_spec()`,
288 lines). Together **530 lines of critical pipeline lifecycle code** with zero coverage.

A `MockAgent` struct implementing `PipelineAgent` will return controlled `PipelineCtx` values
to test every branch of the orchestration loop without network access or real LLM calls.

## Scope

- `bacon-pipeline/src/core/agent.rs` — add `#[cfg(test)] mod tests` with MockAgent
- `bacon-pipeline/src/agent/pipeline.rs` — add `#[cfg(test)] mod tests`
- No production code changes (test-only addition)

## Next Steps

1. Implementer writes MockAgent and ~30 test functions
2. Run `cargo nextest run --all-features -p bacon-pipeline --lib`
3. Run `check-fast.ps1`
4. Move spec to `_done/` on green
