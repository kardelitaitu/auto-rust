# Decisions

- Keep `browser.rs` as the orchestration entry point.
- Use explicit connector and factory boundaries instead of one large discovery function.
- Attach browser capabilities to `Session` rather than rediscovering them at call sites.
- Preserve filter and retry semantics by default.
- Prefer deterministic tests with mocks over browser-heavy timing tests.
