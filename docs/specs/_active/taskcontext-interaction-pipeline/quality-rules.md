# Quality Rules

- Do not break the public TaskContext verb surface.
- Do not duplicate pause or verification logic across verbs if a shared helper can own it.
- Keep the pipeline small enough to test in isolation.
- Prefer explicit result types over boolean-only flow when the action needs verification context.
- Keep browser-backed coverage focused on representative selectors and edge cases.
- Keep the spec short and easy to review.
