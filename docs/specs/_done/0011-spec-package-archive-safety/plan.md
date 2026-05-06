# Plan

1. Define the archive command contract and decide how the helper normalizes status fields.
2. Build `spec-archive.ps1` so the move and both status updates happen together.
3. Improve `spec-lint.ps1` messages so archive drift points at the exact package and field mismatch.
4. Update `docs/specs/README.md` and the template README with the archive workflow.
5. Validate with `spec-lint.ps1`, then `.\check-fast.ps1`, then `.\check.ps1`.

# Internal API Outline

## `spec-archive.ps1`

- Input: a spec package path or id under `_active/`
- Responsibility: archive one package in a single operation
- Expected behavior:
  - validate the package has the required spec files
  - confirm the package is in an archiveable state
  - rewrite `README.md` and `spec.yaml` status fields to `done`
  - normalize the implementer field to the archived convention
  - move the folder from `_active/` to `_done/`

## `spec-lint.ps1`

- Keep the rule read-only.
- Keep failing on `_done` packages that are not actually done.
- Improve the error text so the package path and the exact status mismatch are obvious.

## `docs/specs/README.md`

- Document the archive helper as the normal handoff step.
- Keep the active/done lifecycle table accurate.
- Tell authors to sync both status fields before or during archive.

## `docs/specs/_template/README.md`

- Keep the archive rule visible in the starter copy.
- Keep the template short and easy for weaker agents to follow.

# Decisions

- Keep `spec-lint.ps1` strict and read-only.
- Add a dedicated archive helper instead of asking authors to remember the two status edits by hand.
- Keep the archive helper at the repo root so it is easy to find and reuse.
- Keep the active/done lifecycle unchanged.
- Keep the archive rule in the docs as a visible checklist item.

