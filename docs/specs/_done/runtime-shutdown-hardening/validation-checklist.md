# Validation Checklist

- [x] Cancellation before worker acquisition leaves no stale `Busy` session.
- [x] Cancellation during task execution unwinds cleanly.
- [x] Cancellation during backoff exits without hiding the cancellation cause.
- [x] Cancellation after page acquisition releases owned runtime resources.
- [x] Final session state resolves to `Idle` or `Failed`.
- [x] Cancellation is explicit and not inferred from error text.
- [x] `.\check-fast.ps1` passes during implementation.
- [x] `.\check.ps1` passes before handoff.
