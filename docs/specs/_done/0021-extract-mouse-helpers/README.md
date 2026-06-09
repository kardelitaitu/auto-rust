# 0021-extract-mouse-helpers

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Further extract the 2,705-line `src/utils/mouse.rs` into 4 new submodules: curves, CDP mapping, overlay logic, and adaptive config. The mouse module already has 3 submodules (native 758, trajectory 611, types 268 = 1,637 lines), but the main file still holds curve generators, CDP protocol helpers, overlay state, and adaptive cursor logic — all mixed together.

This is a P2 follow-up to the initial mouse modularization (which created native/, trajectory/, types/).

## Scope

- `src/utils/mouse/` directory only
- Extraction targets: `curves.rs`, `cdp.rs`, `overlay.rs`, `adaptive.rs`
- Existing submodules (native, trajectory, types) unchanged

## Next Steps

1. Review spec package
2. Implement extraction preserving function visibility
3. Verify: cargo check, cargo test --lib mouse, cargo clippy
