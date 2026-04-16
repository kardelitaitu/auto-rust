//! Scrolling and page interaction utilities.
//!
//! Provides functions for simulating human-like scrolling behavior:
//! - Random scrolling with variable speed and direction
//! - Smooth scrolling to specific positions
//! - Human-like pauses to simulate reading behavior
//! - Utilities for page navigation and interaction

use chromiumoxide::Page;
use anyhow::Result;
use crate::utils::math::{random_in_range, gaussian};
use crate::utils::timing::human_pause;

pub async fn random_scroll(page: &Page) -> Result<()> {
    // Random scroll with human-like behavior
    let direction = random_in_range(0, 1);
    let amount = gaussian(500.0, 200.0, 100.0, 1500.0) as i32;

    if direction == 0 {
        // Scroll down
        scroll_down(page, amount).await?;
    } else {
        // Scroll up
        scroll_up(page, amount).await?;
    }

    // Pause to simulate reading
    human_pause(2000, 50).await;

    Ok(())
}

#[allow(dead_code)]
pub async fn human_scroll(page: &Page, direction: &str, amount: i32) -> Result<()> {
    match direction {
        "down" => scroll_down(page, amount).await,
        "up" => scroll_up(page, amount).await,
        _ => Err(anyhow::anyhow!("Invalid scroll direction: {direction}")),
    }
}

async fn scroll_down(page: &Page, amount: i32) -> Result<()> {
    // Execute JavaScript to scroll down
    page.evaluate(format!("window.scrollBy({{top: {amount}, behavior: 'smooth'}});")).await?;
    Ok(())
}

async fn scroll_up(page: &Page, amount: i32) -> Result<()> {
    // Execute JavaScript to scroll up
    page.evaluate(format!("window.scrollBy({{top: -{amount}, behavior: 'smooth'}});")).await?;
    Ok(())
}

pub async fn scroll_to_top(page: &Page) -> Result<()> {
    page.evaluate("window.scrollTo({top: 0, behavior: 'smooth'});").await?;
    Ok(())
}

pub async fn scroll_to_bottom(page: &Page) -> Result<()> {
    page.evaluate("window.scrollTo({top: document.body.scrollHeight, behavior: 'smooth'});").await?;
    Ok(())
}