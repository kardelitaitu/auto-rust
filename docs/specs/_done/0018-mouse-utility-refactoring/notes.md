# Implementation Notes

## Session Date: 2026-05-07

### Approach Change (After Review)

**Original plan** (REJECTED): Create `interact.rs` and move trajectory code (but `trajectory.rs` already exists!).

**New plan** (APPROVED): Refactor within `mouse.rs` by extracting helper functions.

### Rationale

1. **Mouse module already well-modularized**: 3 submodules already exist
   - `native.rs`: 680 lines (native input, calibration)
   - `trajectory.rs`: 500 lines (Bezier, Arc, Zigzag, etc.)
   - `types.rs`: 74 lines (Point, PathStyle, etc.)
2. **Adding more submodules = over-modularization**: No benefit
3. **Root file still too large**: 2,877 lines needs reduction
4. **Helper functions sufficient**: Extract 3-4 helpers to reduce by ~400-500 lines

### Current Code Structure (VERIFIED)

**`mouse.rs`** - 2,877 lines total:

| Lines | Component | Description |
|-------|-----------|-------------|
| 1-150 | Imports + constants | 5 const/static declarations |
| 150-530 | Overlay + curve generation | `generate_*_curve` functions |
| 530-667 | Mouse dispatch helpers | `dispatch_mouse*`, `map_cdp_*` |
| 667-731 | `dispatch_mouse_action` | ~65 lines, handles all mouse actions |
| 731-1143 | Cursor movement | `move_cursor*`, `sync_cursor*` |
| 1143-1280 | `move_cursor_collision_avoidant` | ~138 lines, collision avoidance |
| 1281-1320 | `detect_ui_collisions_along_path` | ~40 lines |
| 1320-2428 | Additional helpers | `wait_for_element_stability`, etc. |
| 2428-2877 | Tests | 41 test functions (~449 lines) |

### Extraction Plan

#### 1. Extract from `move_cursor_collision_avoidant` (Lines 1143-1280, ~138 lines)

**Purpose**: Break down collision avoidance into smaller functions.

**New functions:**
```rust
/// Check if a point collides with any UI element
fn check_point_collision(
    page: &Page,
    point: &Point,
    viewport: &Viewport,
) -> Result<bool> {
    // Extracted from lines 1186-1230
}

/// Generate alternative path to avoid UI collisions
fn generate_avoidance_path(
    start: &Point,
    end: &Point,
    collisions: &[Point],
) -> Vec<Point> {
    // Extracted from lines 1230-1270
}
```

**Benefits**:
- Removes ~80 lines from `move_cursor_collision_avoidant`
- Makes collision logic testable independently
- Easier to understand individual pieces

#### 2. Simplify `dispatch_mouse_action` (Lines 667-731, ~65 lines)

**Purpose**: Extract repeated patterns in mouse action dispatch.

**New function:**
```rust
/// Dispatch a single mouse event with proper button mapping
async fn dispatch_single_mouse_event(
    page: &Page,
    x: f64,
    y: f64,
    event_type: DispatchMouseEventType,
    button: MouseButton,
) -> Result<()> {
    // Extracted from lines 690-720
}
```

**Benefits**:
- Removes ~30 lines from `dispatch_mouse_action`
- Eliminates repeated dispatch patterns
- Cleaner action handling

#### 3. Simplify `detect_ui_collisions_along_path` (Lines 1281-1320, ~40 lines)

**Purpose**: This function is already small, but can be made clearer.

**Approach**: Possibly inline or add documentation.

### Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|-------------|
| Breaking mouse movement | High | Run `cargo test` after EACH extraction |
| Incorrect collision detection | Medium | Verify collision avoidance works |
| Losing overlay sync | Low | Test cursor overlay separately |

### Progress Tracking

- [ ] Read full `mouse.rs` file
- [ ] Extract `check_point_collision` (~30 lines)
- [ ] Extract `generate_avoidance_path` (~50 lines)
- [ ] Simplify `dispatch_mouse_action` (~30 lines)
- [ ] Simplify `detect_ui_collisions_along_path` (~20 lines)
- [ ] Run `cargo test` - all tests pass
- [ ] Run `.\check.ps1` - full CI passes
- [ ] Update line counts in spec

### Key Reminders

1. **Keep all code in same file** - no new files
2. **Run `cargo check` after each change**
3. **Don't change behavior** - only restructure
4. **Add rustdoc** to new helper functions
5. **Verify tests pass** - especially mouse tests


