# Baseline

## What I Find

### 1. Default `run()` orchestration in `core/agent.rs` (242 lines) has zero tests

`bacon-pipeline/src/core/agent.rs:106-348` contains the `PipelineAgent::run()` default
method — the canonical pipeline orchestrator that drives all 4 stages with resume support,
confidence checks, coder-refusal abort, scope-reduction failure report, spec-file validation,
user confirmation gates, and run-log persistence.

There is no `#[cfg(test)] mod tests` block covering `run()`. The only test in the file
(agent.rs:358-373) tests the private helper `is_no_action_description()`, not the orchestration
itself.

### 2. `Pipeline` struct and stage implementations in `agent/pipeline.rs` (288 lines) have zero tests

Every method on `Pipeline` — `new()`, `llm_for_agent()`, `run()`, `run_parallel()`,
`run_single_spec()`, and all 4 `PipelineAgent` stage methods (`run_observer`,
`run_strategist`, `run_coder`, `run_auditor`) — has no `#[cfg(test)]` module.

### 3. Mocking infrastructure already exists

The `PipelineAgent` trait (agent.rs:28), `PipelineCtx` struct (mod.rs:392), `FileSystem`
trait (traits.rs:9), `CommandRunner` trait (traits.rs:47), and `LlmClient` trait
(traits.rs:75) are already fully abstracted for test injection.

### 4. Integration tests only cover CLI entry point

The 3 integration test files in `tests/` invoke the pipeline through the CLI binary
or dry-runs with fake Ollama. They do not test individual orchestration branches
(resume, confidence, refusal, scope-reduction, etc.) in isolation.

## What I Claim

Adding a `MockAgent` struct and ~30 focused unit tests covering all 9 logical branches
of `PipelineAgent::run()` and all 4 paths of `run_parallel()`/`run_single_spec()` will:

- Catch regressions in the orchestration logic before they reach integration tests
- Document expected behavior for each branch (confidence check, resume, fast-path, etc.)
- Enable safe refactoring of the pipeline lifecycle
- Reduce the "refactor fear" around the 530-line uncovered orchestration layer

## What Is the Proof

1. **File with zero coverage**: `core/agent.rs` — 374 lines total, only 16-line test
   for `is_no_action_description()` at line 358. The 242-line `run()` method at line 106
   has zero tests. (File inspection, commit `a03abf7` HEAD.)

2. **File with zero coverage**: `agent/pipeline.rs` — 288 lines, zero `#[cfg(test)]`
   blocks, zero test functions. No test module appears anywhere in the file. (File
   inspection, current HEAD.)

3. **Test infrastructure ready**: `PipelineCtx::new()` at mod.rs:441 accepts
   `Option<Arc<dyn FileSystem>>`, `Option<Arc<dyn CommandRunner>>`, and
   `Option<Arc<dyn LlmClient>>` — all test doubles. The `PipelineAgent` trait at
   agent.rs:28 has 9 methods that return simple values or `Result<PipelineCtx>`,
   easily mockable with a hand-written struct.

4. **Adjacent modules are well-tested**: Coder has 40 tests (coder.rs), Strategist has 14
   (strategist.rs), core module has ~32 combined — proving the testing infrastructure
   and team practices support this level of coverage. The orchestration layer is a
   conspicuous gap.
