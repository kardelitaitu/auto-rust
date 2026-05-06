# Decisions

- Use git stash instead of git reset because it preserves the original worktree state.
- Include untracked files so new edits are recoverable too.
- Default restore mode should apply, not pop, because it is safer.
- Keep the scripts at the repo root so they are easy to find and reuse.
- Keep the workflow short enough for smaller agents to follow without extra context.
