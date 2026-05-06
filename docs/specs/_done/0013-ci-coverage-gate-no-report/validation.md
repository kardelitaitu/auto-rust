# Validation Checklist

- Coverage job runs without HTML or JSON report generation.
- No coverage artifact upload remains in CI.
- Coverage floor still fails below 40%.
- `spec-lint.ps1` passes.
- `.\check.ps1` passes.

# CI Commands

- `.\spec-lint.ps1`
- `.\check-fast.ps1`
- `.\check.ps1`

# Quality Rules

- Keep the workflow short.
- Do not reintroduce report artifacts.
- Keep the coverage floor visible in CI.

