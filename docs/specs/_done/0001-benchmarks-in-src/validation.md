# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo bench --all-features`
- [ ] `cargo bench --all-features --bench trajectory`
- [ ] `cargo bench --all-features --bench accessibility_locator`
- [ ] `cargo bench --all-features --bench predictive_scorer`
- [ ] `./check.ps1`

# CI Commands

Use these exact commands during implementation:

```powershell
cargo check
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo bench --all-features
./check.ps1
```

If the accessibility locator benchmark stays feature-gated, also run:

```powershell
cargo bench --all-features --bench accessibility_locator
```

# Quality Rules

- Keep the move mechanical and small.
- Do not reimplement production logic inside benchmark files.
- Keep benchmark helpers reusable only when two or more benchmarks need them.
- Do not add benchmark code to `lib.rs`.
- Keep feature-gated code explicit in Cargo and in the spec.
- Do not delete the root `benches/` folder until the moved targets pass validation.

