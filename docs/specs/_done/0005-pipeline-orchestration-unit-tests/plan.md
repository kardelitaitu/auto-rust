# Plan

## What Is the Solution

Add test-only code to two files — no production logic changes.

### `core/agent.rs` — MockAgent + orchestration tests

Create a `MockAgent` struct that implements `PipelineAgent`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockAgent {
        name: &'static str,
        dry_run: bool,
        auto: bool,
        fast: bool,
        resume: Option<Stage>,
        pipeline_cfg: PipelineConfig,
        observer_result: Result<PipelineCtx>,
        strategist_result: Result<PipelineCtx>,
        coder_result: Result<PipelineCtx>,
        auditor_result: Result<PipelineCtx>,
    }

    impl PipelineAgent for MockAgent { /* return stored fields */ }
}
```

Cover these test groups (~30 tests total):

**run() — happy path (5 tests)**
1. All 4 stages execute in sequence
2. Fast path skips strategist + auditor
3. Resume from Strategist skips Observer
4. Resume from Coder skips Observer + Strategist
5. Dry run short-circuits to no-op

**run() — confidence (3 tests)**
6. High confidence continues without prompt
7. Low confidence in auto mode: warns but continues
8. Low confidence in non-auto mode: confirm() returns false → aborts

**run() — observer no-op (2 tests)**
9. "No clear improvement found" in auto mode → early return
10. Non-auto mode continues past no-op

**run() — strategist decisions (3 tests)**
11. No spec produced in auto mode → early return
12. Missing file refs in spec → needs-human-approval, return
13. Normal strategist output → continues to coder

**run() — coder outcomes (4 tests)**
14. Coder refused → bail with "Coder refused"
15. Auto-apply gate fails → bail with "requires human approval"
16. Retries exhausted → writes failure report, bail
17. Normal coder → continues to auditor

**run() — run log (2 tests)**
18. "done" outcome on successful completion
19. "coder-refused" outcome on coder refusal

**run() — stage_delay (1 test)**
20. delay_ms > 0 in config → sleep called between stages

### `agent/pipeline.rs` — Pipeline struct tests (2 tests)

**Pipeline::new() (2 tests)**
21. Validation passes with default config path
22. Validation fails with LLM_PROVIDER=openrouter

### Test strategy notes

- `check_confidence()` calls `confirm()` (reads stdin) — test only in auto mode.
  The non-auto confidence path validated through `check_confidence()` free function.
- `run()` uses `check_stale_in_progress()` which reads filesystem — mock via
  temp dir with no stale in-progress specs.
- `run()` instantiates `RealFileSystem/Runner/LlmClient` internally — MockAgent
  stage methods never invoke them; they return canned `PipelineCtx` results.
- `Pipeline::run()` integration with real LLM is covered by integration tests
  in `tests/` (bacon_pipeline_integration.rs, bacon_dry_run_smoke.rs).
  Orchestration branching (resume, fast-path, coder outcomes) is unit-tested
  entirely via MockAgent in core/agent.rs.
