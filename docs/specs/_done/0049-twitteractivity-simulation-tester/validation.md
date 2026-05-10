## Validation

- Simulation logs a deterministic plan for the same seed and payload.
- Simulation reports the expected phase sequence without browser access.
- Simulation emits action roll decisions and stop reason using the exact log schema.
- Simulation completes without page navigation or DOM scans.
- Live task behavior remains unchanged unless `simulate_only=true` is selected.
- `spec-lint.ps1`, `./check-fast.ps1`, and `./check.ps1` pass.
