# 0024-fix-clippy-warnings

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Fix the 6 remaining clippy warnings introduced during the config module extraction (spec 0018). The extraction created new module boundaries that left unused imports and glob re-export visibility issues. These are all trivial fixes — removing unused `use` statements and fixing visibility modifiers.

## Scope

- `src/config/mod.rs` — remove unused DurationMs, TwitterLLMConfig imports; fix glob re-export visibility
- `src/config/types.rs` — remove unused NativeClickCalibrationMode, NativeInputBackend, TracingConfig, log::info imports
- `src/config/defaults.rs` — fix glob re-export visibility
- Target: 0 clippy warnings across the codebase

## Next Steps

1. Review spec package
2. Remove unused imports and fix visibility
3. Verify: cargo clippy --all-targets --all-features = 0 warnings
