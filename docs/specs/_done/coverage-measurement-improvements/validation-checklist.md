# Validation Checklist

- `coverage.ps1` runs from the repo root.
- The coverage run includes integration tests.
- A machine-readable summary file or report is created.
- The CI job fails below the 40% floor.
- A trend artifact is produced with stable fields.
- `.\check-fast.ps1` still works for local iteration.
- `.\check.ps1` still passes before handoff.
- `spec-lint.ps1` stays green for the package.
