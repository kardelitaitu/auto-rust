# Plan

## What Is the Solution

Fix all 6 clippy warnings with targeted edits:

| File | Fix | Change |
|------|-----|--------|
| `config/mod.rs` | Remove `use crate::session::DurationMs;` | -1 line |
| `config/mod.rs` | Remove `TwitterLLMConfig` from unused import list | -1 token |
| `config/types.rs` | Remove unused imports: `NativeClickCalibrationMode`, `NativeInputBackend`, `TracingConfig` | -3 tokens |
| `config/types.rs` | Remove `use log::info;` (not used in types.rs) | -1 line |
| `config/mod.rs` | Change `pub use env::*;` to `pub(crate) use env::*;` (env functions are pub(crate)) | visibility fix |
| `config/mod.rs` | Change `pub use defaults::*;` to keep or adjust visibility | verify correct |

**Post-fix verification**: `cargo clippy --all-targets --all-features` must show 0 warnings.
