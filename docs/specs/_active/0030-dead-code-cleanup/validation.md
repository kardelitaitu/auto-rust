# validation checklist#

Implementation details to be defined during active development.

# ci commands#

```bash
# Run clippy for dead code warnings
cargo clippy --all-targets --all-features -- -D warnings

# Run tests to verify no regressions
cargo test

# Full CI check
./check.ps1
```

# quality rules#

1. **Test after each removal**
2. **Only remove confirmed dead code**
3. **Keep public API stable**
