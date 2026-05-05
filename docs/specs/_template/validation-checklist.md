# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo nextest run --all-features --lib`
- [ ] Relevant targeted tests
- [ ] Manual verification if needed

