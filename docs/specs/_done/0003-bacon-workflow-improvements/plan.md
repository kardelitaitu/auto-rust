# Bacon Pipeline — Workflow Hardening & Provider Extensibility

**Priority:** P1 | **Status:** approved | **Owner:** spec-agent

---

## Current State Analysis

The bacon pipeline is functional and production-ready, but several hardening gaps and missing features limit its reliability, testability, and flexibility. Below is a systematic analysis based on code inspection.

### 🔴 Critical Gaps

#### 1. Single LLM Provider (NVIDIA-only)

The `bacon-pipeline/src/llm/client.rs` LLM client is hardcoded to NVIDIA Chat Completions format:
- Parses `body["choices"][0]["message"]["content"]` (NVIDIA format)
- The project's integration tests (`bacon_dry_run_smoke.rs`, `bacon_pipeline_integration.rs`) previously failed because the fake test server returned Ollama format
- No provider dispatch or format abstraction layer
- `validate_bacon_local_only()` allows `provider = "ollama"` but the client can't parse Ollama responses

**Evidence:**
- `bacon-pipeline/src/llm/client.rs` lines 107-108: hardcoded NVIDIA JSON path `body["choices"][0]["message"]`
- The test fix in this session changed fake servers from Ollama to NVIDIA format — confirming the incompatibility
- `bacon-pipeline/src/llm/mod.rs::llm_config_for_agent()` resolves providers but the client ignores them

#### 2. No `bacon test` CLI Command

The `.bacon/workflow.md` and `.bacon/README.md` both document `bacon test — run pipeline test harness`, but:
- `cli_types.rs` only has a `Command::Run` variant in the enum
- No `test` subcommand exists anywhere in `src/bin/bacon.rs`
- Running pipeline tests requires `cargo test -p bacon-pipeline` directly

**Evidence:**
- `bacon-pipeline/src/core/cli_types.rs`: `enum Command { Run(RunArgs) }` — only one variant
- `.bacon/workflow.md` CLI reference lists `bacon test` and `bacon test --list`
- 106 passing unit tests in bacon-pipeline with no CLI entry point

#### 3. Stage Timing Not Tracked

The `PipelineCtx` struct has no fields for stage timing, making it impossible to:
- Measure per-stage latency in the pipeline
- Detect slow stages programmatically
- Generate performance metrics across runs

**Evidence:**
- `bacon-pipeline/src/core/mod.rs::PipelineCtx`: 11 fields but none for timing/duration
- `core/agent.rs::PipelineAgent::run()` calls stage methods but doesn't wrap them in timing
- The `metrics.rs` discussion in docs references pipeline metrics but they're not implemented

### 🟡 Medium-Impact Gaps

#### 4. Parallel Spec Execution Not Implemented

The `--parallel` CLI flag exists and is parsed in both `Cli` and `RunArgs` structs, but:
- `PipelineAgent::run()` serializes all stages sequentially
- No tokio task spawning for parallel spec processing
- `run_external_agent()` blocks on `child.wait_with_output()`

**Evidence:**
- `cli_types.rs`: `pub parallel: bool` flag defined
- `core/agent.rs::PipelineAgent::run()`: sequential `run_observer() -> strategist -> coder -> auditor` with no branching
- No `tokio::spawn` or `join!` usage in the pipeline runner

#### 5. Env Var Drift (NVIDIA_TOP_P)

The `.bacon/workflow.md` documents `NVIDIA_TOP_P` as an environment variable, but:
- `load_nvidia_config_from_env()` never reads it
- Only `NVIDIA_API_KEY`, `NVIDIA_BASE_URL`, `NVIDIA_MODEL`, `NVIDIA_TEMPERATURE`, `NVIDIA_MAX_TOKENS` are read
- The TOML config supports `top_p` per-agent but env override is missing

**Evidence:**
- `bacon-pipeline/src/llm/mod.rs::load_nvidia_config_from_env()` — only 5 env vars supported
- `.bacon/workflow.md` env var table lists `NVIDIA_TOP_P`
- `bacon-pipeline/src/core/mod.rs::AgentLlmConfig` parses `top_p` from TOML

#### 6. No Persistent Run Log

The pipeline has no record of past runs after completion:
- `PipelineCtx` is ephemeral — destroyed after `run()` returns
- The spec filesystem tracks spec state but not pipeline execution events
- Crash recovery (`check_stale_in_progress()`) only handles in-progress specs, not interrupted pipeline execution tokens

**Evidence:**
- `core/agent.rs::PipelineAgent::run()` returns `Result<()>` with no side-effect logging
- No writes to `.bacon/sessions/` from the pipeline runner
- `check_stale_in_progress()` only scans `_active/` specs — doesn't check for zombie pipeline processes

#### 7. Test Coverage: 19+ `#[ignore]` Tests

The bacon-pipeline has good unit test coverage (106 passing) but many tests require the host project filesystem:
- `#[ignore = "requires host project filesystem"]` appears on 19+ tests
- `#[ignore = "requires ProjectConfig initialization"]` appears on several spec_io tests
- No mock filesystem abstraction for isolated testing

**Evidence:**
- `core/mod.rs` tests: 6 `#[ignore]` requiring filesystem
- `agent/strategist.rs` tests: 2 `#[ignore]`
- `agent/coder.rs` tests: 3 `#[ignore]`
- `core/spec_io.rs` tests: 4 `#[ignore]`
- `core/agent.rs` tests: 0 `#[ignore]` (well-tested module)

---

## Recommended Improvements

### Phase 1: Foundation (High Impact, Low Effort)

#### 1.1 Implement `NVIDIA_TOP_P` env var
- **File:** `bacon-pipeline/src/llm/mod.rs`
- **Change:** Add to `load_nvidia_config_from_env()`:
  ```rust
  if let Ok(top_p) = std::env::var("NVIDIA_TOP_P") {
      if let Ok(v) = top_p.parse::<f64>() {
          config.top_p = v;
      }
  }
  ```
- **Est:** 15 minutes
- **Risk:** None — pure additive change

#### 1.2 Add stage timing metrics to PipelineCtx
- **Files:** `bacon-pipeline/src/core/mod.rs`, `core/agent.rs`
- **Change:** Add `stage_durations: Vec<(Stage, Duration)>` field to `PipelineCtx`, wrap stage calls in `Instant::now()` timing, log durations
- **Est:** 1 hour
- **Risk:** Low — purely additive, no behavioral change

### Phase 2: CLI & Testability (Medium Impact, Medium Effort)

#### 2.1 Implement `bacon test` subcommand
- **Files:** `bacon-pipeline/src/core/cli_types.rs`, `src/bin/bacon.rs`
- **Change:** Add `Command::Test(TestArgs)` variant, execute `cargo test -p bacon-pipeline` with output streaming
- **Est:** 2-3 hours
- **Risk:** Low — standalone subcommand, no pipeline interference

#### 2.2 Add end-to-end pipeline integration test
- **Files:** `tests/bacon_pipeline_integration.rs` (existing)
- **Change:** Write a test that initializes `ProjectConfig`, creates a temp spec, and runs all 4 stages in dry-run mode without `#[ignore]`
- **Est:** 3-4 hours
- **Risk:** Medium — requires careful temp-dir setup to avoid polluting real filesystem

### Phase 3: Parallel & Multi-Provider (High Impact, Higher Effort)

#### 3.1 Ollama response parsing in LLM client
- **Files:** `bacon-pipeline/src/llm/client.rs`, `llm/mod.rs`
- **Change:** Add response format dispatch based on provider config. NVIDIA path stays as-is. Add `parse_ollama_response()` for `{"message":{"role":"assistant","content":"..."},"done":true}` format.
- **Est:** 4-6 hours
- **Risk:** Medium — must handle both formats without breaking existing NVIDIA pipeline

#### 3.2 Implement `--parallel` spec execution
- **File:** `bacon-pipeline/src/core/agent.rs`
- **Change:** When `parallel=true`, spawn independent approved specs as concurrent tokio tasks with `tokio::spawn` and `futures::future::join_all`. Each task gets its own `PipelineCtx` clone.
- **Est:** 4-6 hours
- **Risk:** High — concurrent git operations could conflict. Need per-spec GitSnapshot isolation and locking.

### Phase 4: Observability (Medium Impact, Medium Effort)

#### 4.1 Persistent run log
- **File:** `bacon-pipeline/src/core/mod.rs` or new module `core/run_log.rs`
- **Change:** Write `.bacon/sessions/run-log.yaml` with entries like:
  ```yaml
  - timestamp: 2026-06-03T10:00:00Z
    spec_id: 0013-bacon-workflow-improvements
    stages:
      observer: { duration_ms: 1200, confidence: High }
      strategist: { duration_ms: 3400, confidence: Medium }
      coder: { duration_ms: 15000, attempts: 2, passed: true }
      auditor: { duration_ms: 800, result: PASS }
    outcome: done
  ```
- **Est:** 3-4 hours
- **Risk:** Low — additive, no behavioral change

#### 4.2 `--ci` flag for machine-readable output
- **Files:** `bacon-pipeline/src/core/cli_types.rs`, `core/agent.rs`
- **Change:** Add `--ci` flag that emits GitHub Actions annotations (`::notice`, `::warning`, `::error`) instead of human-readable logs
- **Est:** 2-3 hours
- **Risk:** Low — additive output format

---

## Implementation Order

```
Week 1 ─► Phase 1 (env vars + timing)
              │
              ▼
Week 2 ─► Phase 2 (bacon test + e2e test)
              │
              ▼
Week 3 ─► Phase 3 (Ollama + parallel)
              │
              ▼
Week 4 ─► Phase 4 (observability + CI)
```

**Quick wins** (Phase 1) can be done immediately and provide immediate value.
**Phase 2** unlocks proper CI testing of the pipeline itself.
**Phase 3** delivers the most user-facing value (multi-provider, parallel throughput).
**Phase 4** rounds out production readiness.

---

## Files Changed Summary

| Phase | Files | Lines Changed |
|-------|-------|--------------|
| 1.1 | `bacon-pipeline/src/llm/mod.rs` | +5 |
| 1.2 | `bacon-pipeline/src/core/mod.rs`, `core/agent.rs` | +30 |
| 2.1 | `bacon-pipeline/src/core/cli_types.rs`, `src/bin/bacon.rs` | +50 |
| 2.2 | `tests/bacon_pipeline_integration.rs` | +80 |
| 3.1 | `bacon-pipeline/src/llm/client.rs`, `llm/mod.rs`, `llm/models.rs` | +60 |
| 3.2 | `bacon-pipeline/src/core/agent.rs` | +80 |
| 4.1 | `bacon-pipeline/src/core/` (new module) | +120 |
| 4.2 | `bacon-pipeline/src/core/cli_types.rs`, `core/agent.rs` | +40 |
