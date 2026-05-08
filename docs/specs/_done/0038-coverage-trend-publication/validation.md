## Validation

- CI coverage output includes a machine-readable summary at the expected path.
- The summary is published in a retained form that can be compared across runs.
- Local `coverage.ps1` still produces HTML and JSON coverage output.
- `spec-lint.ps1` passes before handoff.
- `./check.ps1` passes before the spec moves to `_done/`.
