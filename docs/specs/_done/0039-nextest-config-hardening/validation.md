## Validation

- `.config/nextest.toml` includes the intended CI profile settings.
- `.github/workflows/ci.yml` still runs `cargo nextest run --all-features --profile ci`.
- `./check.ps1` passes after the config change.
- `spec-lint.ps1` passes before handoff.
