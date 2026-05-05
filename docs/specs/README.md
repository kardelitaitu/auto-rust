# Spec Workspace

This folder is the handoff boundary between the planning agent and implementation agents.

## Roles

- Spec agent: analyze the codebase, write the spec package, and do not change code.
- Implementer agent: follow the approved spec, change code/tests/docs, and update implementation notes.

## Lifecycle

`draft` -> `approved` -> `implementing` -> `verified` -> `done`

## Folder Layout

```text
docs/specs/
  README.md
  _template/
  _active/
  _done/
```

## Required Files For Each Initiative

- `README.md`
- `spec.yaml`
- `baseline.md`
- `internal-api-outline.md`
- `plan.md`
- `validation-checklist.md`
- `ci-commands.md`
- `decisions.md`
- `quality-rules.md`
- `implementation-notes.md`

## Working Rules

- One spec folder per initiative.
- Keep active work in `_active/`.
- Move the folder to `_done/` only after `./check.ps1` passes.
- Update the spec before changing scope.
- Keep implementation notes append-only.
- Do not mix unrelated tasks in one spec.

## Handoff Sequence

1. Spec agent writes the spec package from the current codebase state.
2. Human reviews the spec and approves scope.
3. Implementer agent edits code and tests against the spec.
4. Implementer agent updates docs and implementation notes.
5. Full verification runs, then the spec moves to `_done/`.

## Best Practice

- Make the spec small enough to review.
- Make acceptance criteria measurable.
- Keep commands in `ci-commands.md` exact and copyable.
- Keep decisions in `decisions.md` instead of hiding them in code comments.
- Use `quality-rules.md` for invariants and review rules.

