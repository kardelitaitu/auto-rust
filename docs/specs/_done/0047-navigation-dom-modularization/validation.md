# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Verify `src/utils/dom.rs` exists and contains the DOM interaction functions.
- [ ] Verify `src/utils/navigation.rs` only contains functions like `goto` and `wait_for_load`.
- [ ] Verify `cargo test --lib utils` passes.
- [ ] `.\check-fast.ps1` passes.
- [ ] `.\spec-lint.ps1` passes.

# CI Commands

```powershell
cargo test --lib utils
.\check-fast.ps1
.\spec-lint.ps1
```

# Quality Rules

- Ensure no tests are lost during the migration of functions.
- The `TaskContext` struct must still be able to expose its unified API, so internal import updates must be thorough.
