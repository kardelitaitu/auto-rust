## Internal API Outline

### Proposed simulation API

- `simulate(api_or_config, payload, config)`
  - accepts a payload and config
  - requires `simulate_only=true` in the payload contract
  - generates a fresh random seed internally on each run
  - does not require a browser page
  - emits a full log-only plan
- `build_simulation_plan(task_config, persona, limits, seed)`
  - computes phase order
  - computes candidate budget usage
  - rolls action probabilities in memory
  - returns structured log lines that match `log-schema.md`
- `SimulationPlan`
  - phase list
  - candidate batches
  - roll outcomes
  - estimated stop reason
  - generated seed value for traceability

### Shared inputs

- `TaskConfig`
- `PersonaWeights`
- `EngagementLimits`
- `EngagementCounters`

### Shared outputs

- human-readable summary line
- per-phase preview lines
- final stop reason
- generated seed reference for traceability
- exact `simulate_only` payload flag
- stable persona label semantics:
  - `config_default` when no payload weights are supplied
  - `payload_custom` when payload weights override config
