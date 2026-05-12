# Validation Checklist

- [ ] cargo check passes
- [ ] cargo nextest run --all-features --lib passes (2099 tests)
- [ ] cargo clippy --all-targets --all-features -- -D warnings
- [ ] cargo fmt --all -- --check
- [ ] spec-lint.ps1 passes
- [ ] task_context.rs reduced from ~5600 to ~3500 lines
- [ ] No public API changes
