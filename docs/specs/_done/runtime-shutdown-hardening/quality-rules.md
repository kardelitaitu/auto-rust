# Quality Rules

- A canceled flow must not leave the session stuck in `Busy`.
- A canceled flow must release owned runtime resources.
- Cancellation handling must be explicit and testable.
- Tests must prove each timing edge separately.
- Do not widen the implementation scope beyond shutdown correctness.
- Keep the spec short and easy to review.
