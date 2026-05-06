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
