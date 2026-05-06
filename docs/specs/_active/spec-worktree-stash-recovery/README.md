# Spec Worktree Stash Recovery

Status: `implementing`

Owner: `spec-agent`
Implementer: `gemini-cli`

## Summary

Add a small git-stash checkpoint workflow for spec-package driven development. Before handing a package to a lesser agent, create a named worktree snapshot with a repo-root helper. If the worktree gets damaged, restore from that snapshot with a second helper instead of guessing at the right reset command.

## Scope

- In scope:
  - repo-root checkpoint helper that creates a named stash with untracked files
  - repo-root restore helper that applies or pops a named stash checkpoint
  - short workflow guidance in the spec docs and agent instructions
  - safe output that tells a smaller agent what ref to use next
- Out of scope:
  - changing spec schema or lint rules
  - changing runtime code or task behavior
  - adding branch management automation
  - auto-resetting the worktree without an explicit restore command

## Files

- `spec.yaml`
- `baseline.md`
- `internal-api-outline.md`
- `plan.md`
- `validation-checklist.md`
- `ci-commands.md`
- `decisions.md`
- `quality-rules.md`
- `implementation-notes.md`

## Next Step

Implement the checkpoint and restore scripts, then update the spec workflow docs to show the recovery path.
