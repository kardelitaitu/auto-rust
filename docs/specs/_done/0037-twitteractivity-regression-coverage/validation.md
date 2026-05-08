## Validation

- Regression coverage includes `TaskConfig::from_payload()` payload clamping and flags.
- The deterministic `select_entry_point_returns_valid_url` task-level unit test remains green.
- Regression coverage includes `log_summary()` output keys.
- The existing config and entry-point smoke tests still pass.
- `spec-lint.ps1` passed before handoff.
