# Plan

## What Is the Solution

Refactor `mouse.rs` within the same file by extracting helper functions. No new files or directories.

### Step 1: Extract `move_cursor_collision_avoidant` Logic
The function at lines 1143-1280 (~138 lines) handles collision avoidance with UI elements. Extract the core logic into:
```rust
/// Check if a point collides with any UI element
fn check_point_collision(
    page: &Page,
    point: &Point,
    viewport: &Viewport,
) -> Result<bool> {
    // ... extracted collision detection
}

/// Generate alternative path to avoid UI collisions
fn generate_avoidance_path(
    start: &Point,
    end: &Point,
    collisions: &[Point],
) -> Vec<Point> {
    // ... extracted path generation
}
```

### Step 2: Simplify `dispatch_mouse_action`
The function at lines 667-731 (~65 lines) handles multiple mouse action types. Simplify by extracting:
```rust
/// Dispatch a single mouse event with proper button mapping
async fn dispatch_single_mouse_event(
    page: &Page,
    x: f64,
    y: f64,
    event_type: DispatchMouseEventType,
    button: MouseButton,
) -> Result<()> {
    // ... extracted event dispatch
}
```

### Step 3: Extract `detect_ui_collisions_along_path`
The function at lines 1281-1320 (~40 lines) can be simplified.

## Internal API Outline

```rust
// All functions remain in mouse.rs (same file)

/// Check if a point collides with UI elements
fn check_point_collision(
    page: &Page,
    point: &Point,
    viewport: &Viewport,
) -> Result<bool>;

/// Generate alternative path to avoid collisions
fn generate_avoidance_path(
    start: &Point,
    end: &Point,
    collisions: &[Point],
) -> Vec<Point>;

/// Dispatch single mouse event with proper button mapping
async fn dispatch_single_mouse_event(
    page: &Page,
    x: f64,
    y: f64,
    event_type: DispatchMouseEventType,
    button: MouseButton,
) -> Result<()>;
```

## Decisions

1. **Keep in same file**: Mouse module already has 3 submodules (native, trajectory, types). Adding more = over-modularization.
2. **Extract functions, not modules**: Reduce `mouse.rs` from 2,877 to ~2,400-2,500 lines.
3. **Keep tests in file**: The 41 tests are properly placed in `#[cfg(test)]`.
4. **Don't create `interact.rs`**: The spec originally proposed this, but `native.rs` already handles native interaction.

## Expected Outcome

After refactoring:
- `mouse.rs`: ~2,400-2,500 lines (from 2,877)
- `native.rs`: 680 lines (unchanged)
- `trajectory.rs`: 500 lines (unchanged)
- `types.rs`: 74 lines (unchanged)
- Readability: Improved (largest functions broken into helpers)
- Testability: Improved (helpers can be tested independently)


