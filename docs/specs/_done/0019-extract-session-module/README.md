# 0019-extract-session-module

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Extract the 1,951-line `src/session/mod.rs` into focused submodules. The file already has 4 submodules (connector, factory, pool, cleanup) totaling 1,800 lines, but the main file still holds `DurationMs`, `WorkerPermit`, `Session`, `SessionState`, and their test suites — all directly mixed together.

This is the largest remaining non-modularized file in the project and the last of the "Big 4" monoliths after orchestrator (0016), twitter state (0017), and config (0018) were already extracted.

## Scope

- `src/session/mod.rs` only — existing submodules (connector, factory, pool, cleanup) unchanged
- Extraction target: `duration.rs`, `permits.rs`, `session.rs`, `state.rs`

## Next Steps

1. Review spec package (spec.yaml, baseline.md, plan.md, validation.md)
2. Approve for implementation
3. Implement extraction preserving re-export chain
4. Verify: cargo check, cargo test --lib session, cargo clippy
