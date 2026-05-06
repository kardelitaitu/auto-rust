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
