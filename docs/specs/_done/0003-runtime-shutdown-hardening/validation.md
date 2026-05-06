# Validation Checklist

- [x] Cancellation before worker acquisition leaves no stale `Busy` session.
- [x] Cancellation during task execution unwinds cleanly.
- [x] Cancellation during backoff exits without hiding the cancellation cause.
- [x] Cancellation after page acquisition releases owned runtime resources.
- [x] Final session state resolves to `Idle` or `Failed`.
- [x] Cancellation is explicit and not inferred from error text.
- [x] `.\check-fast.ps1` passes during implementation.
- [x] `.\check.ps1` passes before handoff.

# CI Commands

```powershell
.\check-fast.ps1
.\check.ps1
```

# Quality Rules

- A canceled flow must not leave the session stuck in `Busy`.
- A canceled flow must release owned runtime resources.
- Cancellation handling must be explicit and testable.
- Tests must prove each timing edge separately.
- Do not widen the implementation scope beyond shutdown correctness.
- Keep the spec short and easy to review.

