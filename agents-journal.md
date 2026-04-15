# Agents Journal

## Plan (Full Reliability Program)

### Phase 0: Safety baseline (immediate)
- Unify retry ownership in orchestrator only.
- Make timeout cancellation explicit with abort tokens.
- Remove panic paths in page release.
- Wire active_workers decrement and page registry.

### Phase 1: Core execution hardening
- Introduce strict task state machine: Queued -> Running -> Success/Failed/Timeout/Cancelled.
- Add per-task abort controller and group-level cancellation fanout.
- Add deterministic worker acquisition/release guard object.

### Phase 2: Navigation contract
- Replace goto + wait_for_navigation split with a single deterministic navigate_and_wait abstraction.
- Encode retryable navigation errors vs fatal errors.
- Add optional domcontentloaded/load policy per task type.

### Phase 3: Interaction helper reliability
- Implement CDP-native mouse/keyboard path first, JS fallback second.
- Add viewport-aware coordinate model (no fixed 800x300 assumptions).
- Add helper-level health assertions (element exists, page open, context valid).

### Phase 4: Config + policy
- Load config from file + env overrides.
- Move all timeout/retry/concurrency knobs out of code constants.
- Add config validation at startup with fail-fast.

### Phase 5: Observability
- Wire RunSummary end-to-end.
- Emit structured task lifecycle logs and counters.
- Add run report output (run-summary.json) for postmortem.

### Phase 6: Tests for core + helpers
- Integration tests for timeout cancellation and worker cleanup.
- Navigation race tests (redirect, slow load, interrupted nav).
- Interaction tests for click/type reliability on real DOM targets.
