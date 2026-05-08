## Validation

- `TaskResult` serde round-trips `metadata: Option<BTreeMap<String, String>>` when present.
- `run-summary.json.task_metadata` includes records only when a task provides metadata.
- Existing summary counters and breakdowns remain unchanged without metadata.
- `spec-lint.ps1` passes before handoff.
- `./check.ps1` passes before archival.
