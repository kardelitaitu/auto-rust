## Validation

- Same seed, same payload snapshot, and same runtime state snapshot produce the same planner output.
- Live candidate execution still follows the current action ordering and limit checks.
- Simulation uses the planner contract instead of a separate decision path.
- Browser calls stay in the executor, not in the planner.
- The integration tests stay green.
- `spec-lint.ps1`, `./check-fast.ps1`, and `./check.ps1` pass.
