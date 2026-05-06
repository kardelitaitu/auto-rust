# Baseline

- `spec-lint.ps1` already enforces that `_done` packages must have `Status: done` in `README.md` and `status: done` in `spec.yaml`.
- A recent failure showed the workflow still allows manual archive drift: a package can land in `_done/` before both status fields are updated.
- There is no repo-root archive helper today, so moving the folder and editing the status fields are separate manual steps.
- `docs/specs/README.md` describes the lifecycle, but it does not give a single archive command or checklist that keeps statuses in sync.
- `docs/specs/_template/README.md` reminds authors to keep spec packages short and status-aligned, but it does not prevent stale status after the folder move.
- The current system catches the error late, which is correct for lint but brittle for humans and model-driven edits.
