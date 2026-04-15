# Agent Work Queue

## Execution Order (Recommended)
1. Agent 1
2. Agent 2
3. Agent 3
4. Agent 4
5. Agent 5
6. Agent 6

Reason: parser and runtime plumbing first, then task logic, then quality and release hardening.

## Alternative Order
1. Agent 1 and Agent 2 in parallel
2. Agent 3
3. Agent 4 and Agent 5 in parallel
4. Agent 6

Use this order if you want faster throughput with lower conflict risk.

## Queue

### 1) Agent 1: CLI Parser (`agent-1-cli-parser`)
Spec: `migration-steps/agent-specs/agent-1-cli-parser.md`

Files:
- `src/cli.rs`
- `src/tests/integration_test.rs`
- `src/tests/edge_case_test.rs`

Tasks:
- Implement parser contract: `then` grouping, `.js` strip, first `=`, quoted values, URL normalization, numeric to `"value"` string.
- Add parser tests for smoke command and edge cases.
- Add malformed input and helper tests.

### 2) Agent 2: Browser + Session (`agent-2-browser-session`)
Spec: `migration-steps/agent-specs/agent-2-browser-session.md`

Files:
- `src/browser.rs`
- `src/session.rs`
- `src/config.rs`

Tasks:
- Implement discovery, retry, and circuit-breaker flow.
- Implement worker acquire/release and page lifecycle with timeout-safe close.
- Ensure config fields used by browser/session are loaded safely.

### 3) Agent 3: Orchestrator Execution (`agent-3-orchestrator-execution`)
Spec: `migration-steps/agent-specs/agent-3-orchestrator-execution.md`

Files:
- `src/orchestrator.rs`
- `task/mod.rs` (if integration changes are needed)

Tasks:
- Implement sequential group execution with parallel tasks per group.
- Enforce global concurrency, task timeout, group timeout, and retries.
- Keep dispatcher integration compatible.

### 4) Agent 4: Utils + Baseline Tasks (`agent-4-utils-and-tasks`)
Spec: `migration-steps/agent-specs/agent-4-utils-and-tasks.md`

Files:
- `src/utils/navigation.rs`
- `src/utils/timing.rs`
- `src/utils/math.rs`
- `task/pageview.rs`
- `task/cookiebot.rs`
- `task/mod.rs`

Tasks:
- Replace high-use TODO stubs in utils.
- Implement baseline `pageview` and `cookiebot` behavior.
- Keep task registration and dispatch aligned.

### 5) Agent 5: Testing + Quality (`agent-5-testing-and-quality`)
Spec: `migration-steps/agent-specs/agent-5-testing-and-quality.md`

Files:
- `src/tests/*`
- `migration-steps/phase-5-testing-validation.md`

Tasks:
- Align tests with string-normalized payload contract.
- Strengthen smoke and edge parser coverage.
- Keep testing docs synchronized with actual commands and layout.

### 6) Agent 6: Release + Ops (`agent-6-release-and-ops`)
Spec: `migration-steps/agent-specs/agent-6-release-and-ops.md`

Files:
- `migration-steps/phase-6-polish-deployment.md`

Tasks:
- Validate Windows and cross-platform command snippets.
- Verify wrapper script docs (`run.bat`, `run.sh`).
- Ensure CI and release checklist match actual workflow.
