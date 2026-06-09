# Validation Checklist

- [ ] `cargo clippy --all-targets --all-features` — 0 warnings (lib)
- [ ] `cargo clippy --all-targets --all-features` — 0 warnings (lib test)
- [ ] `cargo check` — 0 errors
- [ ] `cargo test --lib` — full test suite passes
- [ ] No `#[allow(...)]` suppressions added — warnings fixed at root
