# API Audit Checklist

last audited 08-05-26 by Kilo

Scope: `src/runtime/task_context.rs`

Goal: review every public `TaskContext` API one by one for correctness, reliability, scalability, and ease of use.

## Constructors and Core State
- [x] `new()` (last arg: `Option<CancellationToken>` for cooperative pause cancel)
- [x] `new_with_metrics()` (orchestrator passes `Some(cancel_token)`)
- [x] `session_id()`
