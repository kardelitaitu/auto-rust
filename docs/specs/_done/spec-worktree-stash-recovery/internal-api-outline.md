# Internal API Outline

## `spec-stash.ps1`

- Input: optional spec id and reason
- Responsibility: create a named git stash checkpoint from the repo root
- Expected behavior:
  - include tracked and untracked changes
  - no-op when the worktree is already clean
  - print the created stash ref so the agent can restore it later

## `spec-restore.ps1`

- Input: optional stash ref and an apply/pop mode
- Responsibility: restore a saved checkpoint from the repo root
- Expected behavior:
  - default to applying the most recent checkpoint
  - support a named stash ref when a later checkpoint needs to be restored
  - keep the stash unless the caller explicitly asks to pop it

## `AGENTS.md`

- Add a short instruction for spec agents and implementers to checkpoint before risky handoff and restore from a named stash after a mistake.

## `docs/specs/README.md` and `_template/README.md`

- Show the checkpoint-before-handoff and restore-after-mistake flow in the repo-root spec instructions.
