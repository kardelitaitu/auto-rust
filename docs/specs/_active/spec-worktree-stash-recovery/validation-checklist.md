# Validation Checklist

- Run `.\spec-stash.ps1` on a dirty worktree and verify it prints the created stash ref.
- Confirm the stash includes untracked files by creating a temporary untracked file before checkpointing.
- Run `.\spec-restore.ps1` with no ref and verify it restores the latest checkpoint.
- Run `.\spec-restore.ps1 -Ref stash@{0} -Pop` and verify the checkpoint is removed only after a successful restore.
- Confirm both scripts refuse to run outside the repository root.
- Verify the workflow docs tell agents to checkpoint before handoff and restore after a mistake.
