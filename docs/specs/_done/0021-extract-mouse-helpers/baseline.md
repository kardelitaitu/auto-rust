# Baseline

## What I Find

`src/utils/mouse.rs` is 2,705 lines with these groups mixed together:

| Lines | Content |
|-------|---------|
| 176-193 | Overlay debug helpers: with_nativeclick_log_context, nativeclick_debug |
| 296-307 | Overlay state: set_overlay_enabled, is_overlay_enabled, overlay_state_for_page |
| 309-472 | cursor_start_position, now_unix_ms, position helpers |
| 473-563 | 6 curve generators: bezier, arc, zigzag, overshoot, stopped, muscle + config-driven dispatch |
| 704-795 | CDP mapping: map_cdp_button, mouse_button_mask, map_cdp_event_type |
| 975-1149 | detect_element_type + element classifiers |
| 1150-1296 | calculate_adaptive_cursor_config with speed/precision/path-style logic |
| 1297-1722 | generate_collision_free_path with obstacle avoidance |
| 1723-1799 | choose_click_point, native_click_center_bounds, native_click_random_center_point |
| 2252-2705 | `#[cfg(test)] pub mod tests` (~453 lines) |

Existing submodules: native.rs (758), trajectory.rs (611), types.rs (268) = 1,637 lines

## What I Claim

Extracting 4 focused submodules (curves, CDP, overlay, adaptive) will reduce mouse.rs from 2,705 to ≤2,100 lines by moving ~600 lines into submodules while keeping core orchestration logic (collision-free path, click point selection) in the main file. This follows the established mouse/ submodule pattern.

## What Is the Proof

1. **2,705 lines after initial modularization**: The first pass split out types, trajectory, and native — but the main file is still the 2nd largest in the project. Clear extraction boundaries remain.

2. **6 curve generators in one block** (lines 473-563): These are pure math functions that could stand alone in `curves.rs`. They're already called via a dispatch function — no tight coupling to mouse.rs internals.

3. **CDP mapping is a self-contained concern** (lines 704-795): `map_cdp_button`, `mouse_button_mask`, `map_cdp_event_type` are protocol translation functions with no mouse.rs state dependencies.
