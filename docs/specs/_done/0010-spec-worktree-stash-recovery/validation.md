# Validation Checklist

- Run `.\spec-stash.ps1` on a dirty worktree and verify it prints the created stash ref.
- Confirm the stash includes untracked files by creating a temporary untracked file before checkpointing.
- Run `.\spec-restore.ps1` with no ref and verify it restores the latest checkpoint.
- Run `.\spec-restore.ps1 -Ref stash@{0} -Pop` and verify the checkpoint is removed only after a successful restore.
- Confirm both scripts refuse to run outside the repository root.
- Verify the workflow docs tell agents to checkpoint before handoff and restore after a mistake.

# CI Commands

- `.\spec-lint.ps1`
- `.\spec-stash.ps1 -Spec <initiative> -Reason "before handoff"`
- `.\spec-restore.ps1`
- `.\check-fast.ps1`
- `.\check.ps1`

# Quality Rules

- Keep the checkpoint ref visible in the console output.
- Keep the restore script explicit about the stash ref it is using.
- Keep the workflow readably named so a smaller agent knows it is a recovery path.
- Avoid silent worktree changes.
- Keep docs short and direct.

