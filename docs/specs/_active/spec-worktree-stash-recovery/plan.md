# Plan

1. Add `spec-stash.ps1` as a repo-root checkpoint helper that creates a named stash and prints the ref.
2. Add `spec-restore.ps1` as a repo-root restore helper that applies or pops a stash ref.
3. Update `AGENTS.md`, `docs/specs/README.md`, and `docs/specs/_template/README.md` with the recovery workflow.
4. Validate the scripts with a checkpoint/restore smoke test.
5. Run `spec-lint.ps1` and `.\check-fast.ps1` after the workflow lands.
