# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Verify `src/utils/twitter/twitteractivity_sentiment*.rs` files no longer exist.
- [ ] Verify `src/utils/twitter/sentiment/strategies/` contains `emoji.rs`, `domain.rs`, and `llm.rs`.
- [ ] Verify `cargo test --lib utils::twitter::sentiment` passes.
- [ ] `.\check-fast.ps1` passes.
- [ ] `.\spec-lint.ps1` passes.

# CI Commands

```powershell
.\check-fast.ps1
.\spec-lint.ps1
cargo test --lib utils::twitter::sentiment
```

# Quality Rules

- Ensure no tests are lost during the migration of files.
- The `src/utils/twitter/mod.rs` must be clean of any old sentiment references.
