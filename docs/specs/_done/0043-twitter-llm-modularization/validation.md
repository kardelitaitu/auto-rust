# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Verify `twitteractivity_llm.rs` line count is significantly reduced.
- [ ] Verify unit tests for validation still run and pass: `cargo test --lib utils::twitter::twitteractivity_llm_validation::tests`
- [ ] `.\check-fast.ps1` passes.
- [ ] `.\spec-lint.ps1` passes.

# CI Commands

```powershell
.\check-fast.ps1
.\spec-lint.ps1
```

# Quality Rules

- Extracted modules should handle their own imports.
- Do not change the core logic of the extracted functions.
