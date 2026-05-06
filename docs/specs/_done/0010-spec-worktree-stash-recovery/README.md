# Spec Worktree Stash Recovery

Status: `done`

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

## Implementation

✅ **Completed**: The checkpoint and restore scripts have been implemented and validated:
- `spec-stash.ps1` - Creates named git stash checkpoints
- `spec-restore.ps1` - Restores from stash checkpoints
- `AGENTS.md` - Updated with workflow guidance

**Usage**:
```powershell
# Create checkpoint
.\spec-stash.ps1 "checkpoint-name"

# Restore checkpoint
.\spec-restore.ps1
```

# Baseline

- There is no repo-root helper for creating a consistent git stash checkpoint before handing work to another agent.
- Recovery after a broken handoff is manual today, which means agents have to remember the right `git stash` command and message format.
- The spec workflow docs mention handoff and handback, but they do not give a concrete checkpoint/restore path.
- `AGENTS.md` does not yet tell agents to snapshot the worktree before a risky handoff.
- The current gap is process safety, not runtime behavior.

