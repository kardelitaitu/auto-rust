//! Demo: Keyboard Interaction
//!
//! This example demonstrates the keyboard utilities for human-like typing and key presses.
//! Run with: cargo run --example demo-interaction-keyboard
//!
//! Demo runtime budget is configurable here for quick local edits.
//!
//! Note: This example shows the API. Since it's in an examples/ folder, it doesn't have
//! direct access to the crate's internal modules. In actual usage, you would use these
//! functions with a chromiumoxide Page.

use auto::utils::timing::DEFAULT_DEMO_DURATION_MS;

fn main() {
    println!("=== Keyboard Interaction Demo ===\n");
    println!(
        "Recommended demo runtime: {}s",
        DEFAULT_DEMO_DURATION_MS / 1000
    );

    println!("Available functions:");
    println!("  press(page, key) - Press a single key");
    println!("  press_with_options(page, key, options) - Press with options");
    println!("  press_with_modifiers(page, key, modifiers) - Press with modifier keys");
    println!("  focus(page, selector) - Focus an element before typing");
    println!("  r#type(page, selector, text) - Type text with human timing");
    println!("  type_text_with_options(page, text, base_delay) - Type with custom delay");
    println!("  hold(page, key, duration_ms) - Hold a key for duration");
    println!("  release_all(page) - Release all modifier keys");
    println!("  natural_typing(page, selector, text, typo_rate) - Natural typing with typos");
    println!();

    println!("Key features implemented:");
    println!("  - PressOptions struct for configuring key presses");
    println!("  - Modifier support (ctrl, shift, alt, meta)");
    println!("  - Key chords (multiple keys pressed together)");
    println!("  - Repeat and delay options");
    println!("  - Human-like typing with variable delays");
    println!("  - Typo simulation with auto-correction");
    println!("  - Hold key for game controls");
    println!("  - Release all safety function");
    println!();

    println!("Usage examples:");
    println!();
    println!("  // Press a single key");
    println!("  press(&page, \"Enter\").await?;");
    println!("  press(&page, \"Escape\").await?;");
    println!("  press(&page, \"a\").await?;");
    println!();

    println!("  // With modifiers (Ctrl+S, Ctrl+Shift+C)");
    println!("  press_with_modifiers(&page, \"s\", &[\"ctrl\"]).await?;");
    println!("  press_with_modifiers(&page, \"c\", &[\"ctrl\", \"shift\"]).await?;");
    println!();

    println!("  // Key chords (WASD)");
    println!("  press(&page, \"w\").await?;");
    println!("  press(&page, \"a\").await?;");
    println!("  press(&page, \"s\").await?;");
    println!("  press(&page, \"d\").await?;");
    println!();

    println!("  // Special keys");
    println!("  press(&page, \"ArrowUp\").await?;");
    println!("  press(&page, \"F1\").await?;");
    println!("  press(&page, \"Tab\").await?;");
    println!("  press(&page, \"Backspace\").await?;");
    println!();

    println!("  // With options (repeat, delay)");
    println!("  let options = PressOptions {{");
    println!("      modifiers: vec![\"ctrl\".to_string()],");
    println!("      delay: 50,");
    println!("      repeat: 3,");
    println!("      down_and_up: true,");
    println!("  }};");
    println!("  press_with_options(&page, \"s\", &options).await?;");
    println!();

    println!("  // Type text with human-like timing");
    println!("  api.focus(\"input, textarea\").await?;");
    println!("  api.r#type(\"input, textarea\", \"Hello World!\").await?;");
    println!();

    println!("  // Type with custom base delay (ms)");
    println!("  type_text_with_options(&page, \"Fast typing\", 50).await?;");
    println!("  type_text_with_options(&page, \"Slow typing\", 200).await?;");
    println!();

    println!("  // Hold key (for games)");
    println!("  hold(&page, \"w\", 500).await?;  // Hold 'w' for 500ms");
    println!();

    println!("  // Release all modifier keys (safety)");
    println!("  release_all(&page).await?;");
    println!();

    println!("  // Natural typing with typo simulation");
    println!("  natural_typing(&page, \"#username\", \"testuser\", 0.1).await?;");
    println!();

    println!("=== Demo Complete ===");
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_demo_runs() {
        super::main();
    }
}
