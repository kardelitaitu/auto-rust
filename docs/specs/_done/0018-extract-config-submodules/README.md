# Extract Config Submodules

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Extract 13 struct definitions, 2 enums, Default impls, and env override logic from the 3631-line `src/config/mod.rs` — the largest file in the codebase — into 3 focused submodules under `src/config/`.

## Scope

Extract into submodules without behavioral changes:
- `types.rs` — all 13 struct + 2 enum definitions
- `defaults.rs` — all Default impl blocks + `default_*()` helper functions
- `env.rs` — `apply_env_overrides()` + `from_env_value()` helpers

`config/mod.rs` becomes a re-export layer + `Config::load()` + `load_from_file()`.

`config/validation.rs` already exists as a separate file — no changes needed.

## Next Steps

1. Implementer reads `baseline.md` and `plan.md`
2. Extract each type group into its target submodule
3. Verify `cargo check && cargo test --lib`
4. Run `cargo clippy --all-targets --all-features`
5. Archive spec to `_done/`
