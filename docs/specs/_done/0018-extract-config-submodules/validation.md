# Validation

## Pre-implementation Checklist

- [ ] `cargo check -p auto-rust` passes
- [ ] All tests pass (`cargo test --lib config`)
- [ ] `cargo clippy --all-targets --all-features` — 0 warnings
- [ ] Public API unchanged — all type paths preserved via re-exports
- [ ] Each submodule ≤ target: types ≤800, defaults ≤600, env ≤300
- [ ] mod.rs ≤200 lines (re-exports + load_from_file)
- [ ] No behavioral changes — Default impls, env overrides, load_from_file unchanged

## Post-implementation Verification

```bash
cargo check -p auto-rust
cargo test --lib config
cargo clippy --all-targets --all-features
```
