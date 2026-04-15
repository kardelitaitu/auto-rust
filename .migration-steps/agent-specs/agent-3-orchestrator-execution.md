# Agent 3 Spec: Orchestrator Scheduling And Task Execution

## Scope
Own group execution, global concurrency, retries, task timeout handling, and dispatch integration.

## Files
- `src/orchestrator.rs`
- `task/mod.rs` integration points if needed

## Functional Requirements
- Execute groups sequentially.
- Execute tasks within a group in parallel.
- Enforce global concurrency limit.
- Apply task timeout and group timeout.
- Retry failed task attempts using configured limits.
- Continue processing remaining groups after per-group failures (log and continue).

## Acceptance Checks
- Smoke command executes two groups in correct order.
- A failing task does not crash entire process.
- Timeout path produces explicit error and clean release of worker/page.

## Out Of Scope
- CLI parsing details
- Utility behavior internals

## Report Format
- Scheduling model implemented
- Concurrency and timeout settings used
- Known starvation or fairness caveats
