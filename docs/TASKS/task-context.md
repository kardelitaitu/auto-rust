# TaskContext Guide

Use this guide when you change the public task API, `api.*` verbs, or runtime behavior that task authors depend on.

## Read First

- [docs/API_REFERENCE.md](../API_REFERENCE.md)
- [src/runtime/TASK-API.md](../../src/runtime/TASK-API.md)
- [docs/ARCHITECTURE.md](../ARCHITECTURE.md)

## Scope

TaskContext is the public task-facing API.

- Keep it thin.
- Keep runtime and session lifecycle hidden behind the runtime layer.
- Prefer shared capabilities over ad hoc helpers.

## Behavior Rules

- `api.click(selector)` should use the selector pipeline, not a one-off path.
- High-level verbs should include the normal settle pause behavior.
- `api.pause(base_ms)` uses uniform variance.
- `api.pause_human(base_ms, pct)` uses Gaussian delay.
- Cancellation tokens should wake pauses and long waits early.

## Change Rules

- Update `docs/API_REFERENCE.md` when the public API changes.
- Update `src/runtime/TASK-API.md` when the human-readable verb list changes.
- Keep new verbs short and predictable.
- Do not move browser/session lifecycle logic into TaskContext.

## Related Docs

- [docs/TASKS/overview.md](overview.md)
- [docs/TASKS/selectors.md](selectors.md)
- [docs/ARCHITECTURE.md](../ARCHITECTURE.md)

