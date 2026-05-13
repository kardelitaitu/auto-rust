# <initiative name>

Status: `draft`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Explain the problem, why it matters, and the target outcome in one paragraph.

## Scope

- In scope:
- Out of scope:

## Files

This template contains 5 files. The strategist adds `baseline.md` during generation, for a total of 6 files in the final package:

- `spec.yaml` — initiative metadata, acceptance criteria, risks
- `plan.md` — step-by-step implementation plan
- `validation.md` — how to verify the implementation
- `notes.md` — design decisions, risks, context
- `README.md` — this file, overview and rules

## Rules

- New spec packages start as `draft` in this template.
- Keep the spec short.
- Put only approved specs in `_active/`.
- Put only done specs in `_done/`.
- Do not include `spec-lint.ps1` in a normal feature spec; it is read-only.
- Before handing off to another agent, checkpoint the worktree with `.\spec-stash.ps1`.
- Restore with `.\spec-restore.ps1` if the handoff breaks the tree.
- Run `spec-lint.ps1` before handoff; it prints the exact package and fix to apply.
- Use `.\check-fast.ps1` while iterating and `.\check.ps1` before push.
- **Archive rule**: Use the archive helper `.\spec-archive.ps1` to move completed specs to `_done/`. This ensures both status fields are properly synchronized to `done` and the implementer field is normalized to `archived-*`.

## Next Step

Write the spec package before any code changes.
