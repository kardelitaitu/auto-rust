# Spec Workspace

This is the contract between spec planning and implementation.

## Contract

- One initiative per folder.
- Spec agent writes the spec package only.
- Implementer edits code, tests, and docs only after approval.
- Keep `implementation-notes.md` append-only.

## Lifecycle

| Status | Folder |
|---|---|
| `draft` | `_template/` working copy |
| `approved` | `_active/<initiative>/` |
| `implementing` | `_active/<initiative>/` |
| `done` | `_done/<initiative>/` |

## Enforcement

- `_active/` may contain only `approved` and `implementing` specs.
- `_done/` may contain only `done` specs.
- Use `.\check-fast.ps1` during implementation.
- Move a spec to `_done/` only after `.\check.ps1` passes.
- Run `spec-lint.ps1` before handoff.
