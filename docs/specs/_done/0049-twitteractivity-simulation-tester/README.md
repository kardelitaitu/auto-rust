# TwitterActivity Simulation Tester

Status: `done`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Add a fast, browser-free simulation mode for `twitteractivity` that logs the task’s likely behavior over the full duration, including persona rolls, scan cadence, candidate budget, action selection, limit exhaustion, and stop reasons. Enable it with `simulate_only=true` in the payload. The task generates a fresh random seed for each run and logs it for traceability.

## Scope

- In scope:
  - pure simulation path with no browser or DOM access
  - random per-run seed generation with logged traceability
  - simulated persona selection and probability decisions
  - simulated scan loop timing and candidate budget usage
  - log-only task preview output
  - regression tests for simulation output shape and log schema
  - explicit `simulate_only` payload contract
- Out of scope:
  - live browser execution
  - DOM-driven candidate discovery
  - clicking, scrolling, or other side effects
  - changing the real engagement logic beyond simulation hooks

## Files

- `spec.yaml`
- `baseline.md`
- `internal-api-outline.md`
- `plan.md`
- `validation-checklist.md`
- `validation.md`
- `notes.md`
- `log-schema.md`
- `ci-commands.md`
- `decisions.md`
- `quality-rules.md`
- `implementation-notes.md`

## Notes

- Keep the simulation fast and traceable.
- Keep live task behavior unchanged unless `simulate_only=true` is explicit.
- Prefer a separate simulation entry point over overloading the live path.
- Treat `persona=config_default` as config-only weights and `persona=payload_custom` as payload overrides.
- Treat the logged seed as trace data, not user input.
