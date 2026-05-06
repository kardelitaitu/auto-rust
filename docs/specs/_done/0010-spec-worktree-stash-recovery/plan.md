# Plan

1. Add `spec-stash.ps1` as a repo-root checkpoint helper that creates a named stash and prints the ref.
2. Add `spec-restore.ps1` as a repo-root restore helper that applies or pops a stash ref.
3. Update `AGENTS.md`, `docs/specs/README.md`, and `docs/specs/_template/README.md` with the recovery workflow.
4. Validate the scripts with a checkpoint/restore smoke test.
5. Run `spec-lint.ps1` and `.\check-fast.ps1` after the workflow lands.

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

# Decisions

- Use git stash instead of git reset because it preserves the original worktree state.
- Include untracked files so new edits are recoverable too.
- Default restore mode should apply, not pop, because it is safer.
- Keep the scripts at the repo root so they are easy to find and reuse.
- Keep the workflow short enough for smaller agents to follow without extra context.

