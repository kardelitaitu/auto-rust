# Internal API Outline

The orchestrator module's public API flows through `Orchestrator`:

```
Orchestrator::new(config) → Orchestrator
  ├── execute_group(tasks, sessions, metrics, shutdown) → Result<GroupOutcome>
  │     └── execute_group_with_cancel(...) → Result<GroupOutcome>
  │           └── acquire_global_execution_slot(...) → Result<GlobalExecutionSlot>
  │                 └── execute_task_on_session(...) → Result<()>
  │                       └── execute_task_with_retry(...) → Result<()>
  │                             └── SessionExecutionGuard (Drop: mark_idle/mark_failed)
  └── [health helpers for metrics/logging]
```

All submodules are `pub(super)` — only `mod.rs` exposes the public API surface.
