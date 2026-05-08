## Implementation Notes

- Added `metadata: Option<BTreeMap<String, String>>` to `TaskResult` with serde defaults and backward-compatible constructors.
- Threaded optional metadata through `MetricsCollector::task_completed_from_result` into `TaskMetrics`.
- Exported task metadata records in `run-summary.json` as a top-level `task_metadata` array.
- Added round-trip and export tests for present and absent metadata cases.
