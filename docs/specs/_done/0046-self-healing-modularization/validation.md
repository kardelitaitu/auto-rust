# Validation Checklist

- [ ] `cargo check`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Verify `src/adaptive/self_healing.rs` file no longer exists.
- [ ] Verify `src/adaptive/self_healing/` contains the 5 targeted modules (`health`, `strategy`, `history`, `state`, `system`).
- [ ] Verify `cargo test --lib adaptive` passes.
- [ ] `.\check-fast.ps1` passes.
- [ ] `.\spec-lint.ps1` passes.

# CI Commands

```powershell
cargo test --lib adaptive
.\check-fast.ps1
.\spec-lint.ps1
```

# Quality Rules

- Keep the new modules strictly focused on their specific domains.
- Use `pub(crate)` or private visibility for types that do not need to leave the `self_healing` directory.
- Re-export the primary interfaces in `self_healing/mod.rs` to minimize disruption to other crates.
