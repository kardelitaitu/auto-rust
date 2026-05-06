# Validation Checklist

- Archive an approved spec package and verify both `README.md` and `spec.yaml` become `done`.
- Confirm the helper moves the folder into `_done/` without leaving a stale active copy behind.
- Confirm the helper refuses to archive a package that is missing required files or has inconsistent status fields.
- Run `spec-lint.ps1` against a deliberately stale `_done` package and verify the error names the exact mismatch.
- Verify `docs/specs/README.md` explains the archive helper and the required status sync step.
- Verify the template README still stays short and visible as the starter contract.
