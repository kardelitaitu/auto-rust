# Plan

## What Is the Solution

Extract 3 groups from `src/result.rs`:

| New File | Content | Source Lines | Target |
|----------|---------|-------------|--------|
| `src/result/types.rs` | TaskStatus, TaskResult + impl | 13-140 | ≤200 |
| `src/result/errors.rs` | TaskErrorKind enum + impl + Display for TaskErrorKind | 141-220, 294-306 | ≤200 |
| `src/result/summary.rs` | RunSummary struct + impl + Default + Display for TaskStatus | 222-293 | ≤100 |
| `src/result/tests.rs` | Both test modules (tests + tdd_tests) | 307-1400 | ≤1100 |
| `src/result/mod.rs` | Module decls + re-exports + RunSummary::default() | shrunken | ≤150 |

**Directory restructure**: `src/result.rs` → `src/result/mod.rs` (with `types.rs`, `errors.rs`, `tests.rs` as siblings).

**Cross-reference**: `types.rs` imports `errors::TaskErrorKind` via `use super::errors::TaskErrorKind`. `summary.rs` imports both `TaskStatus` and `TaskErrorKind`. Re-export chain: `pub use types::*; pub use errors::*; pub use summary::*;` in `mod.rs`.

**Test access**: `#[cfg(test)] mod tests;` in `mod.rs` loads `tests.rs` which uses `use super::*;` for all types.
