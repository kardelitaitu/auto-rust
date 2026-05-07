# Mouse Utility Refactoring

Status: `done`

Owner: `spec-agent`
Implementer: `implementation-agent`

## Summary
The `mouse.rs` file is 2,877 lines with ~80 functions. The file already has 3 submodules (`native.rs` 680 lines, `trajectory.rs` 500 lines, `types.rs` 74 lines) which handle complex algorithms. However, the root `mouse.rs` file still contains ~1,600 lines of logic that should be refactored into helper functions for better readability.

## Scope
- **In scope**: Extract 1-2 largest functions from `mouse.rs` into helper functions within the same file. Reduce `mouse.rs` from 2,877 to ~2,400-2,500 lines.
- **Out of scope**: Creating new submodules (already has 3), moving code to other files, changing core logic.

## Next Step
Extract largest functions from `mouse.rs` into helper functions within the same file.

# Baseline

## What I Find
The `src/utils/mouse.rs` file is **2,877 lines** long with **~80 functions** and **5 constant declarations** (not 232 lines of constants). The file already exports 3 submodules: `native` (680 lines), `trajectory` (500 lines), `types` (74 lines).

## What I Claim
While the mouse module is already partially modularized, the root `mouse.rs` file is still too large at 2,877 lines. Extracting the largest functions (e.g., `move_cursor_collision_avoidant` at ~150 lines, `dispatch_mouse_action` at ~70 lines) will improve readability without adding unnecessary submodule complexity.

## What Is the Proof
1. `mouse.rs` is 2,877 lines - the root file alone is larger than most modules.
2. The file already has proper submodules - adding more would over-modularize.
3. It contains 41 inline tests in `#[cfg(test)]` - properly placed, should stay.
4. Largest functions: `move_cursor_collision_avoidant` (~150 lines), `dispatch_mouse_action` (~70 lines), `detect_ui_collisions_along_path` (~50 lines).

