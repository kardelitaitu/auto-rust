# Validation Checklist

- [ ] `cargo check` — 0 errors
- [ ] `cargo test --lib mouse` — all 63 mouse tests pass
- [ ] `cargo test --lib` — full test suite passes
- [ ] `cargo clippy --all-targets --all-features` — 0 new warnings
- [ ] `mouse.rs` ≤ 2100 lines
- [ ] `mouse/curves.rs` ≤ 300 lines
- [ ] `mouse/cdp.rs` ≤ 150 lines
- [ ] `mouse/overlay.rs` ≤ 100 lines
- [ ] `mouse/adaptive.rs` ≤ 350 lines
- [ ] Curve generators accessible from trajectory.rs via pub(crate)
- [ ] Public API unchanged: set_overlay_enabled, is_overlay_enabled still pub
