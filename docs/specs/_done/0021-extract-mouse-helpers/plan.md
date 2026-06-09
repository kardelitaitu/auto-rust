# Plan

## What Is the Solution

Extract 4 groups from `utils/mouse.rs` into new submodules under `src/utils/mouse/`:

| New File | Content | Source Lines | Target |
|----------|---------|-------------|--------|
| `curves.rs` | 6 curve generators (bezier, arc, zigzag, overshoot, stopped, muscle) + curriculum_dispatch | 473-563 | ≤300 |
| `cdp.rs` | map_cdp_button(), mouse_button_mask(), map_cdp_event_type() | 704-795 | ≤150 |
| `overlay.rs` | set_overlay_enabled(), is_overlay_enabled(), overlay_state_for_page(), debug helpers | 176-307 | ≤100 |
| `adaptive.rs` | calculate_adaptive_cursor_config(), detect_element_type() + element classifiers | 975-1296 | ≤350 |
| `mod.rs` | Module decls + re-exports + core orchestration (collision-free path, click points, position helpers) | shrunken | ≤2100 |

**Visibility strategy**: Functions called across submodules (e.g., curve generators called from trajectory.rs) marked `pub(crate)`. Public API preserved through re-exports.

**Test distribution**: All 63 tests (2252-2705) stay in mouse.rs `#[cfg(test)] pub mod tests`.
