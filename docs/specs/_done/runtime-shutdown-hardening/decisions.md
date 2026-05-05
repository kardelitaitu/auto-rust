# Decisions

- Use explicit cancellation state instead of parsing error message text.
- Keep shutdown coordination centralized in the runtime shutdown manager.
- Prefer deterministic tests over browser-heavy timing tests whenever possible.
- Keep this spec focused on shutdown correctness, not on broader task or CLI refactors.
