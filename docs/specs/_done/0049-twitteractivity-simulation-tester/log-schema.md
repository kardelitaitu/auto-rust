## Log Schema

Simulation output must use one line per event with a stable prefix and key/value pairs:

```text
simulation | key=value key=value key=value
```

Rules:

- Prefix is always `simulation | `.
- Keys use lowercase snake_case.
- Values use unquoted scalars when possible.
- Fields are space-separated `key=value` pairs.
- Order is fixed per event type.
- Do not add ad hoc fields without updating this schema.

Event types:

- `simulation | seed=<u64> simulate_only=true duration_ms=<u64> persona=<config_default|payload_custom> profile=<name>`
- `simulation | phase=navigation entry_point=<home|explore|notifications|...> action=<simulate|skip>`
- `simulation | phase=scan scan_index=<u32> candidate_budget=<u32> candidates_found=<u32>`
- `simulation | roll candidate_index=<u32> action=<like|retweet|quote|follow|reply|bookmark|dive> p=<f64> r=<f64> result=<hit|miss>`
- `simulation | budget action=<name> used=<u32> limit=<u32> result=<allow|block>`
- `simulation | stop_reason=<reason> total_actions=<u32> scans=<u32> remaining_ms=<u64>`

Allowed stop reasons:

- `duration_exhausted`
- `limit_reached`
- `candidate_budget_exhausted`
- `no_more_planned_actions`
- `simulated_error`

Example:

```text
simulation | seed=123456789 simulate_only=true duration_ms=120000 persona=config_default profile=Average
simulation | phase=navigation entry_point=home action=simulate
simulation | phase=scan scan_index=1 candidate_budget=5 candidates_found=3
simulation | roll candidate_index=0 action=like p=0.40 r=0.17 result=hit
simulation | budget action=like used=1 limit=5 result=allow
simulation | stop_reason=limit_reached total_actions=10 scans=18 remaining_ms=0
```
