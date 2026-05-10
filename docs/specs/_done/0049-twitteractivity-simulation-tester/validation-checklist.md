## Validation Checklist

- [ ] Simulation does not call browser-only APIs.
- [ ] Simulation accepts the same payload shape as the task, including `simulate_only=true`, or a clearly documented simulation payload.
- [ ] Simulation generates a fresh random seed each run and logs it.
- [ ] Simulation logs persona weights, scan cadence, candidate budget, and action roll outcomes using the exact log schema.
- [ ] Simulation uses stable persona labels: `config_default` or `payload_custom`.
- [ ] Simulation logs a final stop reason using the exact log schema.
- [ ] Simulation completes quickly and does not wait for browser timing.
- [ ] The live `twitteractivity` path still works unchanged when simulation is off.
- [ ] New tests cover repeatability and output shape.
- [ ] `spec-lint.ps1` passes.
- [ ] `./check-fast.ps1` passes.
- [ ] `./check.ps1` passes.
