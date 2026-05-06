# Spec Package Archive Safety

Status: `done`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Make spec archiving a single, low-friction workflow so packages land in `_done/` with `done` status already synced in both `README.md` and `spec.yaml`. Improve lint feedback so stale archive state is obvious, and document the expected archive path in the repo-root spec workflow docs.

## Scope

- In scope:
  - repo-root archive helper for moving an approved or implementing package to `_done/`
  - synchronized status updates in `README.md` and `spec.yaml` during archive
  - clearer `spec-lint.ps1` failure messages for `_done` status drift
  - `docs/specs/README.md` and template guidance for the archive flow
- Out of scope:
  - schema changes to the spec package format
  - weakening lint enforcement
  - changing active or done lifecycle rules
  - feature work outside the spec workflow itself

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

Hand this package to the implementer after the archive flow and lint feedback are approved.

# Baseline

- `spec-lint.ps1` already enforces that `_done` packages must have `Status: done` in `README.md` and `status: done` in `spec.yaml`.
- A recent failure showed the workflow still allows manual archive drift: a package can land in `_done/` before both status fields are updated.
- There is no repo-root archive helper today, so moving the folder and editing the status fields are separate manual steps.
- `docs/specs/README.md` describes the lifecycle, but it does not give a single archive command or checklist that keeps statuses in sync.
- `docs/specs/_template/README.md` reminds authors to keep spec packages short and status-aligned, but it does not prevent stale status after the folder move.
- The current system catches the error late, which is correct for lint but brittle for humans and model-driven edits.

