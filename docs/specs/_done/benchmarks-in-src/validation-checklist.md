# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo bench --all-features`
- [ ] `cargo bench --all-features --bench trajectory`
- [ ] `cargo bench --all-features --bench accessibility_locator`
- [ ] `cargo bench --all-features --bench predictive_scorer`
- [ ] `./check.ps1`

