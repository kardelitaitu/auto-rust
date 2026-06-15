# Quality Rules

1. **No behavioral changes** — extracted code must be byte-for-byte equivalent in behavior
2. **Import hygiene** — each submodule imports only what it needs; avoid `use super::*`
3. **Test co-location** — tests for guard types live in `guards.rs`, retry tests in `retry.rs`, etc.
4. **Mod.rs is a facade** — only `Orchestrator` struct, `new()`, and `pub` re-exports
5. **Async compatibility** — all async fns remain async, tokio is already a workspace dependency
