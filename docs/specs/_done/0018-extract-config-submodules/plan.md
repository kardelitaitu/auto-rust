# Plan

## What Is the Solution

Extract 13 struct definitions, 2 enums, Default impls, and env override logic into 4 submodules under `src/config/`:

```
src/config/
  mod.rs        — load_config() + ConfigValidationReport + re-exports (≤200 excl. tests)
  types.rs      — All 13 struct + 2 enum definitions + their direct impls (≤800)
  defaults.rs   — All standalone Default impls + default_*() helpers (≤600)
  env.rs        — load_dotenv_defaults + load_code_config + apply_env_overrides (≤300)
  tests.rs      — All config tests extracted from mod.rs (≤3000)
  validation.rs — unchanged (already separate)
```

### Extraction mapping

| Type/Function | Lines (est.) | → Target |
|---|---|---|
| `NativeClickCalibrationMode` enum + `impl` (+ from_env_value, as_str) | ~35 | `types.rs` |
| `NativeInputBackend` enum + `impl` (+ from_env_value, as_str) | ~35 | `types.rs` |
| `Config` struct | ~20 | `types.rs` |
| `BrowserConfig` struct | ~110 | `types.rs` |
| `NativeInteractionConfig` struct | ~30 | `types.rs` |
| `CircuitBreakerConfig` struct | ~15 | `types.rs` |
| `BrowserProfile` struct | ~12 | `types.rs` |
| `RoxybrowserConfig` struct | ~12 | `types.rs` |
| `OrchestratorConfig` struct | ~25 | `types.rs` |
| `TwitterActivityConfig` struct | ~40 | `types.rs` |
| `TwitterProbabilitiesConfig` struct + Default impl | ~50 | `types.rs` |
| `TwitterLLMConfig` struct (uses `#[derive(Default)]`) | ~60 | `types.rs` |
| `TracingConfig` struct (uses `#[derive(Default)]`) | ~23 | `types.rs` |
| `TaskDiscoveryConfig` struct + Default impl | ~45 | `types.rs` |
| `EngagementLimitsConfig` struct + Default impl | ~75 | `types.rs` |
| `NativeInteractionConfig` Default impl | ~15 | `defaults.rs` |
| `TwitterActivityConfig` Default impl | ~20 | `defaults.rs` |
| `BrowserConfig` Default impl | ~20 | `defaults.rs` |
| `OrchestratorConfig` Default impl | ~15 | `defaults.rs` |
| `CircuitBreakerConfig` Default impl | ~12 | `defaults.rs` |
| `RoxybrowserConfig` Default impl | ~10 | `defaults.rs` |
| `BrowserProfile` Default impl | ~8 | `defaults.rs` |
| `default_*()` helper functions (~35 functions) | ~300 | `defaults.rs` |
| `load_dotenv_defaults()` | ~35 | `env.rs` |
| `load_code_config()` | ~55 | `env.rs` |
| `apply_env_overrides()` + `parse_env_float()` | ~90 | `env.rs` |
| `load_config()` (orchestrates load_dotenv + load_code or toml + env) | ~20 | `mod.rs` |
| `ConfigValidationReport` + impl | ~150 | `mod.rs` |
| All tests (~60 test functions) | ~2900 | `tests.rs` |

### Wire `mod.rs` with re-exports

```rust
mod defaults;
mod env;
mod types;
#[cfg(test)]
mod tests;
pub mod validation;

pub use defaults::*;
pub use env::*;
pub use types::*;
pub use validation::*;
```

`load_config()` (a free function) and `ConfigValidationReport` stay in `mod.rs`.
They call into `env::load_dotenv_defaults()`, `env::load_code_config()`, `env::apply_env_overrides()`.

### Test distribution

All ~60 test functions move to `tests.rs` as a `#[cfg(test)]` submodule.
Tests use `use super::*;` to access re-exported types from all submodules.
`ConfigValidationReport` stays in `mod.rs` since tests import it via `super::*`.

### Verify

```bash
cargo check -p auto-rust
cargo test --lib config
cargo clippy --all-targets --all-features
```

### Files changed

| File | Action | Target lines |
|------|--------|-------------|
| `src/config/mod.rs` | Shrink to re-exports + load_config + ConfigValidationReport | ≤200 (excl. tests) |
| `src/config/types.rs` | New | ≤800 |
| `src/config/defaults.rs` | New | ≤600 |
| `src/config/env.rs` | New | ≤250 |
| `src/config/tests.rs` | New (extracted from mod.rs) | ≤3000 |
| `src/config/validation.rs` | Unchanged | — |
