# Internal API Outline

## Goal

Define a pure planner contract that both live execution and simulation can use without browser dependency.

## Proposed Types

### `CandidatePlannerInput`

Minimal immutable input for one candidate planning pass:
- `seed`
- `task_config`
- `persona`
- `limits`
- `counters`
- `scan_index`
- `candidate_index`
- `candidate_snapshot`
- `actions_taken`
- `actions_this_scan`

### `CandidatePlan`

Pure decision output for the executor:
- `planned_actions`
- `should_break`
- `stop_reason`
- `next_scroll_after_ms`

### `PlannedAction`

One decision event in execution order:
- `kind`
- `allowed`
- `probability`
- `roll`
- `limit_before`
- `limit_after`
- `reason`

### `PlannerStopReason`

Stable stop reasons for live and simulated runs:
- `limit_reached`
- `candidate_budget_exhausted`
- `action_budget_exhausted`
- `no_more_planned_actions`
- `duration_exhausted`

## Boundary Rules

- Planner cannot call browser APIs.
- Planner cannot scroll, click, hover, type, or sleep.
- Executor cannot change decision order or override planner stop reasons.
- Simulation must reuse the same planner types as live execution.
