# TwitterActivity Candidate Planner Split

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Extract the candidate decision logic out of `process_candidate()` into a shared planner so live execution and simulation use the same decision contract. Keep browser side effects in a small executor, and use the planner from `twitteractivity_simulation.rs` so simulation stays aligned with the live task.

## Scope

- In scope:
  - shared candidate planning data and decision flow
  - extraction of pure planning logic from `twitteractivity_engagement.rs`
  - reuse of the same plan in `twitteractivity_simulation.rs`
  - small executor for browser-only side effects
  - deterministic plan tests and regression coverage
- Out of scope:
  - rewriting navigation or feed discovery
  - changing selector strategy
  - changing action probabilities or limit defaults
  - adding new Twitter actions

## Files

- `spec.yaml`
- `plan.md`
- `validation.md`
- `notes.md`

## Notes

- Keep the plan contract small and explicit.
- Keep live behavior unchanged unless the planner exposes a real bug.
- Prefer one source of truth for candidate decisions.
- Executor owns browser work only: scroll, hover, click, retry, and side-effect logging.
- Planner owns pure decisions only: candidate evaluation, action order, limit checks, and stop reasons.
- Deterministic tests must snapshot the full planner input, not just the seed.
