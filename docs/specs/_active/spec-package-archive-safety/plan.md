# Plan

1. Define the archive command contract and decide how the helper normalizes status fields.
2. Build `spec-archive.ps1` so the move and both status updates happen together.
3. Improve `spec-lint.ps1` messages so archive drift points at the exact package and field mismatch.
4. Update `docs/specs/README.md` and the template README with the archive workflow.
5. Validate with `spec-lint.ps1`, then `.\check-fast.ps1`, then `.\check.ps1`.
