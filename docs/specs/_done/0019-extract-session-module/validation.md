# Validation Checklist

- [ ] `cargo check` — 0 errors
- [ ] `cargo test --lib session` — all session tests pass
- [ ] `cargo test --lib` — full test suite passes (no regressions)
- [ ] `cargo clippy --all-targets --all-features` — 0 new warnings
- [ ] `session/mod.rs` ≤ 200 lines (excl. test module)
- [ ] `session/duration.rs` ≤ 150 lines
- [ ] `session/session_core.rs` ≤ 400 lines
- [ ] `session/session_ops.rs` ≤ 400 lines
- [ ] `session/permits.rs` ≤ 70 lines
- [ ] `session/state.rs` ≤ 100 lines
- [ ] All 15 files importing `crate::session::DurationMs` still compile
- [ ] Re-exports visible: `DurationMs`, `WorkerPermit`, `Session`, `SessionState`
