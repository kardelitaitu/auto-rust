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

