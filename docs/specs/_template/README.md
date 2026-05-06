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

- `spec.yaml`
- `baseline.md`
- `internal-api-outline.md`
- `plan.md`
- `validation-checklist.md`
- `ci-commands.md`
- `decisions.md`
- `quality-rules.md`
- `implementation-notes.md`

## Rules

- Keep the spec short.
- Put only approved or implementing specs in `_active/`.
- Put only done specs in `_done/`.
- Start `implementation-notes.md` empty.
- Do not include `spec-lint.ps1` in a normal feature spec.
- Run `spec-lint.ps1` before handoff; it prints the exact package and fix to apply.
- Use `.\check-fast.ps1` while iterating and `.\check.ps1` before push.

## Next Step

Write the spec package before any code changes.
