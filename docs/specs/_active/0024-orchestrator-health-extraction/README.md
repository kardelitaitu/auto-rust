# Orchestrator Health Monitor Extraction

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary
The Orchestrator is mixing high-level concurrency coordination with low-level, highly specific business rules. This makes the core runner fragile. A 600+ line health-check function is impossible to safely modify without risking regressions in unrelated orchestrator flows.

## Scope
- **In scope**: Refactoring described in the plan.
- **Out of scope**: Changing core logic.

## Next Step
Begin implementation according to plan.md.

# Baseline

## What I Find
The core `src/orchestrator.rs` file suffers from extreme logic clustering. The function `should_mark_session_unhealthy` is **638 lines long**, and `execute_task_with_retry` is **315 lines long**.

## What I Claim
The Orchestrator is mixing high-level concurrency coordination with low-level, highly specific business rules. This makes the core runner fragile. A 600+ line health-check function is impossible to safely modify without risking regressions in unrelated orchestrator flows.

## What Is the Proof
1. Code analysis confirms the extreme lengths of these two functions, accounting for ~65% of the production logic in the file.
2. `should_mark_session_unhealthy` likely contains massive switch statements or deep `if/else` chains evaluating specific DOM errors, network timeouts, and CDP crashes inline.

