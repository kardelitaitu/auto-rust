//! Demo: Mouse Interaction
//!
//! This example demonstrates the mouse utilities for human-like cursor movement and clicking.
//! Run with: cargo run --example demo-interaction-mouse
//!
//! Demo runtime budget is configurable here for quick local edits.
//!
//! Note: This example shows the API. Since it's in an examples/ folder, it doesn't have
//! direct access to the crate's internal modules. In actual usage, you would use these
//! functions with a chromiumoxide Page.

use crate::utils::timing::DEFAULT_DEMO_DURATION_MS;

fn main() {
    println!("=== Mouse Interaction Demo ===\n");
    println!(
        "Recommended demo runtime: {}s",
        DEFAULT_DEMO_DURATION_MS / 1000
    );

    println!("Cursor movement functions:");
    println!("  cursor_move_to(page, x, y) - Move cursor with default bezier path");
    println!("  cursor_move_to_with_config(page, x, y, config) - Move with custom config");
    println!();

    println!("Path styles (CursorMovementConfig):");
    println!("  PathStyle::Bezier - Smooth bezier curve (default)");
    println!("  PathStyle::Arc - Curved arc movement");
    println!("  PathStyle::Zigzag - Slight back-and-forth");
    println!("  PathStyle::Overshoot - Go past target, come back");
    println!("  PathStyle::Stopped - Micro-stops along the way");
    println!("  PathStyle::Muscle - PID-controlled biological movement");
    println!();

    println!("Precision levels:");
    println!("  Precision::Exact - No randomization");
    println!("  Precision::Safe - ±3px randomization");
    println!("  Precision::Rough - ±10px randomization");
    println!();

    println!("Speed levels:");
    println!("  Speed::Fast - 0.3x speed multiplier");
    println!("  Speed::Normal - 1.0x speed (default)");
    println!("  Speed::Slow - 2.0x speed multiplier");
    println!();

    println!("Click functions:");
    println!("  click_at(page, x, y) - Click at coordinates (left button)");
    println!("  click_at_with_options(page, x, y, button, move_first, precision, hover_ms)");
    println!("  left_click_at(page, x, y) - Left click with movement");
    println!("  right_click_at(page, x, y) - Right click");
    println!("  middle_click_at(page, x, y) - Middle click");
    println!("  click_selector(page, selector) - Click element by CSS selector");
    println!();

    println!("Key features implemented:");
    println!("  - CursorMovementConfig for customizing movement");
    println!("  - PathStyle enum for different movement patterns");
    println!("  - Precision enum for click accuracy");
    println!("  - Speed enum for movement speed");
    println!("  - MouseButton enum for left/right/middle clicks");
    println!("  - Fitts's Law optimal target size calculation");
    println!();

    println!("=== Usage Examples ===\n");

    println!("1. Basic cursor movement:");
    println!("  cursor_move_to(&page, 450.0, 320.0).await?;");
    println!();

    println!("2. Move with custom path style:");
    println!("  let config = CursorMovementConfig::default()");
    println!("      .with_path_style(PathStyle::Arc)");
    println!("      .with_speed(Speed::Slow);");
    println!("  cursor_move_to_with_config(&page, 450.0, 320.0, &config).await?;");
    println!();

    println!("3. Fast click (no human movement):");
    println!("  let config = CursorMovementConfig::default()");
    println!("      .with_speed(Speed::Fast);");
    println!("  cursor_move_to_with_config(&page, 100.0, 100.0, &config).await?;");
    println!();

    println!("4. Click at coordinates:");
    println!("  click_at(&page, 450.0, 320.0).await?;");
    println!();

    println!("5. Click with options:");
    println!("  click_at_with_options(");
    println!("      &page,");
    println!("      450.0, 320.0,");
    println!("      MouseButton::Left,");
    println!("      true,  // move_to_first");
    println!("      Precision::Safe,");
    println!("      200   // hover_ms");
    println!("  ).await?;");
    println!();

    println!("6. Right click:");
    println!("  right_click_at(&page, 450.0, 320.0).await?;");
    println!();

    println!("7. Click CSS selector:");
    println!("  click_selector(&page, \"#submit-button\").await?;");
    println!("  click_selector(&page, \".login-btn\").await?;");
    println!("  click_selector(&page, \"input[type=submit]\").await?;");
    println!();

    println!("8. Fitts's Law calculation:");
    println!("  // For 200px distance and 1000ms movement time");
    let optimal = 2.0 * 200.0 / (2.0_f64.powf(1000.0 / 100.0));
    println!("  let optimal_size = fitts_law_optimal_size(200.0, 1000.0);");
    println!("  // Result: {:.1}px (optimal target width)", optimal);
    println!();

    println!("=== Demo Complete ===");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_demo_runs() {
        main();
    }

    #[test]
    fn test_fitts_law_calc() {
        let size = 2.0 * 100.0 / (2.0_f64.powf(500.0 / 100.0));
        assert!(size > 0.0);
    }
}
