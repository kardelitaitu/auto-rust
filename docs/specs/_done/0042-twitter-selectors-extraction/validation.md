# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Verify `src/utils/twitter/js/` contains the extracted files.
- [ ] Verify `cargo test --lib utils::twitter::twitteractivity_selectors::tests` passes to confirm string generation is identical.
- [ ] `.\check-fast.ps1` passes.
- [ ] `.\spec-lint.ps1` passes.

# CI Commands

```powershell
cargo test --lib utils::twitter::twitteractivity_selectors::tests
.\check-fast.ps1
.\spec-lint.ps1
```

# Quality Rules

- Pure JavaScript files must be syntactically valid JS (no Rust `format!` escaping).
- The public API of `twitteractivity_selectors.rs` must not change.
