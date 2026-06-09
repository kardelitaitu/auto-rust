# Validation

## Pre-implementation Checklist

- [ ] `cargo check -p auto-rust` passes
- [ ] All tests pass (`cargo test --lib`)
- [ ] `cargo clippy --all-targets --all-features` — 0 warnings
- [ ] Public API unchanged — all type paths preserved via re-exports
- [ ] Each submodule ≤ target: types ≤250, session ≤300, tracking ≤200, config ≤250
- [ ] No behavioral changes — Default impls, Drop semantics, pub fn signatures unchanged

## Post-implementation Verification

```bash
cargo check -p auto-rust
cargo test --lib
cargo clippy --all-targets --all-features
```
