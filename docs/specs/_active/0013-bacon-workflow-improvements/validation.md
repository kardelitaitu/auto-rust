# Validation Criteria — Bacon Pipeline Workflow Improvements

## Acceptance Criteria Checklist

### Phase 1: Foundation
- [ ] `NVIDIA_TOP_P` env var is read by `load_nvidia_config_from_env()` and applied as config override
- [ ] `PipelineCtx` has `stage_durations: Vec<(Stage, Duration)>` field
- [ ] Each stage call in `PipelineAgent::run()` is wrapped in `Instant::now()` timing
- [ ] Stage durations are logged at info level after each stage completes

### Phase 2: CLI & Testability
- [ ] `bacon test` subcommand exists and runs `cargo test -p bacon-pipeline`
- [ ] `bacon test --list` lists available bacon-pipeline test targets
- [ ] An end-to-end test runs all 4 pipeline stages in dry-run mode without `#[ignore]`
- [ ] All existing tests still pass (`cargo test -p bacon-pipeline`)

### Phase 3: Parallel & Multi-Provider
- [ ] LLM client can parse both NVIDIA and Ollama response formats based on provider config
- [ ] Ollama provider works end-to-end with a fake test server (verified by test)
- [ ] `--parallel` flag spawns independent specs as concurrent tokio tasks
- [ ] Parallel execution achieves 2x+ speedup on 2+ independent specs

### Phase 4: Observability
- [ ] `.bacon/sessions/run-log.yaml` exists after each pipeline run
- [ ] Run log contains timestamp, spec_id, per-stage duration, and outcome
- [ ] `--ci` flag emits GitHub Actions-compatible JSON annotations
- [ ] Non-goals explicitly documented and enforced

## Verification Commands

```bash
# Phase 1
cargo test -p bacon-pipeline          # all tests pass
cargo clippy -p bacon-pipeline         # no new warnings

# Phase 2
cargo run --bin bacon test             # runs pipeline test suite
cargo run --bin bacon test --list      # lists test targets

# Phase 3
cargo run --bin bacon -- --parallel -p "spec1" -p "spec2"   # parallel execution

# Phase 4
ls .bacon/sessions/run-log.yaml       # run log exists
cargo run --bin bacon -- --ci -p "fix"  # CI-formatted output
```

## Rollback Plan

If any phase introduces regressions:
1. `git checkout HEAD -- bacon-pipeline/src/` to restore pipeline code
2. Revert CLI changes via `git revert <commit>`
3. Run `cargo test -p bacon-pipeline` to verify rollback
