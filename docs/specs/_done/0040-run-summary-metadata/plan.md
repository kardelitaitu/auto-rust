## Plan

1. Confirm `TaskResult.metadata`, `TaskMetrics.metadata`, and `RunSummary.task_metadata` as the only new fields.
2. Thread the field through the task-to-metrics export path.
3. Add focused JSON shape tests for present and absent metadata.
4. Verify `./check.ps1` and keep the payload optional.
