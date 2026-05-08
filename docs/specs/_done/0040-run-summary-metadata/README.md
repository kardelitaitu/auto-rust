# Run Summary Metadata

Status: `done`

Owner: `spec-agent`
Implementer: `archived-pending`

## Summary

Add a small optional string-map metadata payload to task results so `run-summary.json` can carry task-specific context without changing the existing counters or making non-Twitter tasks heavier. Keep the shape optional and source-compatible.

## Scope

- In scope:
  - optional `BTreeMap<String, String>` metadata on task result and task metrics shapes
  - propagation through `MetricsCollector::task_completed_from_result`
  - JSON export of metadata as `run-summary.json.task_metadata`
  - tests for absent and present metadata cases
- Out of scope:
  - new dashboards or viewers
  - changing success/failure/timing counters
  - attaching metadata to every task immediately
  - task execution semantics

## Baseline

- `src/result.rs` has `TaskResult`, but it only carries status, retry, error, and duration fields.
- `src/metrics.rs` exports `run-summary.json` with task/session breakdowns and Twitter counters, but no per-task metadata.
- `MetricsCollector::task_completed_from_result` currently copies the existing `TaskResult` fields into `TaskMetrics` with no extra payload.
- `run-summary.json` is already part of the repo’s reporting path, so the missing piece is payload shape, not a new file format.

## Why This Was Needed

- The summary export already captures aggregate run data, but task-specific context still gets dropped.
- A small string-map metadata field is the smallest way to preserve richer task details without forcing all tasks to pay for it.
- Keeping the field optional preserves compatibility for existing task result constructors and summary tests.

## Files

- `spec.yaml`
- `plan.md`
- `validation.md`
- `notes.md`

## Archive Notes

This package is complete and kept as the reference record for run-summary metadata publication.
