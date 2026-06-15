# Validation

## Pre-implementation Checklist

- [ ] `cargo check -p auto-rust` passes
- [ ] All orchestrator tests pass (`cargo test --lib orchestrator`)
- [ ] `cargo clippy --all-targets --all-features` — 0 warnings
- [ ] Public API unchanged: `Orchestrator::new()`, `execute_group()` signatures identical
- [ ] Each submodule ≤ target line count: execution ≤400, guards ≤250, retry ≤250, health ≤150, mod ≤100
- [ ] No behavioral changes — Drop impls, concurrency semantics, retry logic preserved

## Post-implementation Verification

```bash
# Verify compilation
cargo check -p auto-rust

# Run orchestrator tests
cargo test --lib orchestrator

# Full lint
cargo clippy --all-targets --all-features

# Full test suite
cargo test --lib
```
