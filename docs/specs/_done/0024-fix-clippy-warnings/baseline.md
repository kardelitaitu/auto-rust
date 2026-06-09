# Baseline

## What I Find

`cargo clippy --all-targets --all-features` produces 6 warnings (lib) + 5 warnings (lib test, 4 duplicates):

| File | Warning | Count |
|------|---------|-------|
| `config/mod.rs` | unused import: `DurationMs` | 1 |
| `config/mod.rs` | unused import: `TwitterLLMConfig` | 1 |
| `config/types.rs` | unused import: `NativeClickCalibrationMode` | 1 |
| `config/types.rs` | unused import: `NativeInputBackend` | 1 |
| `config/types.rs` | unused import: `TracingConfig` | 1 |
| `config/types.rs` | unused import: `log::info` | 1 |
| `config/defaults.rs` | glob import doesn't reexport anything with `pub` visibility | 1 |
| `config/mod.rs` | glob import doesn't reexport anything with `pub` visibility (env) | 1 |

**Root cause**: Spec 0018 extracted config types into submodules but left behind imports that were needed in the monolithic file. After extraction, `DurationMs` is re-exported from `types.rs` (via `pub use types::*`), so the explicit import in `mod.rs` is now redundant. Similarly, `env.rs` has `pub(crate)` functions that `pub use env::*` can't re-export as `pub`.

## What I Claim

Removing 6 unused imports and fixing 2 visibility warnings will bring the project to 0 clippy warnings. This is a trivial cleanup with zero behavioral impact.

## What Is the Proof

1. **All 6 unused imports are post-extraction artifacts**: They were needed in the original 3,637-line `config/mod.rs` but became redundant after types moved to submodules.

2. **Glob re-export warnings**: `env.rs` functions are `pub(crate)` — the `pub use env::*;` in `mod.rs` can't re-export them as `pub`. Fix: either make them `pub` in env.rs or remove the `pub` from the use.

3. **Zero risk**: Removing unused imports and adjusting visibility modifiers cannot change runtime behavior.
