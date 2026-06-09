//! Mouse click operations — click dispatch with pointer/mouse event lifecycle.
//!
//! Handles click_at, left_click_at, right_click_at, middle_click_at and the
//! underlying dispatch_click pipeline.

use super::overlay::cursor_move_to;
use super::types::MouseButton;
use crate::utils::timing::human_pause;
use anyhow::Result;
use chromiumoxide::Page;

pub async fn click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    left_click_at(page, x, y).await
}

pub async fn left_click_at_without_move(page: &Page, x: f64, y: f64) -> Result<()> {
    dispatch_click(page, x, y, MouseButton::Left).await
}

pub async fn right_click_at_without_move(page: &Page, x: f64, y: f64) -> Result<()> {
    dispatch_click(page, x, y, MouseButton::Right).await
}

pub async fn left_click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    cursor_move_to(page, x, y).await?;
    human_pause(50, 50).await;
    dispatch_click(page, x, y, MouseButton::Left).await
}

pub async fn middle_click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    cursor_move_to(page, x, y).await?;
    human_pause(50, 50).await;
    dispatch_click(page, x, y, MouseButton::Middle).await
}

pub async fn right_click_at(page: &Page, x: f64, y: f64) -> Result<()> {
    cursor_move_to(page, x, y).await?;
    human_pause(50, 50).await;
    dispatch_click(page, x, y, MouseButton::Right).await
}

pub async fn dispatch_click(page: &Page, x: f64, y: f64, button: MouseButton) -> Result<()> {
    // Fire pointer events around the click for better browser compatibility
    // Real browsers fire these, but most automation tools skip them
    let _ = super::cdp::dispatch_pointer_event(page, "pointerover", x, y, button).await;
    let _ = super::cdp::dispatch_pointer_event(page, "pointerenter", x, y, button).await;

    // Small delay to simulate pointer capture
    crate::utils::timing::human_pause(15, 30).await;

    // Fire pointermove at final position
    let _ = super::cdp::dispatch_pointer_event(page, "pointermove", x, y, button).await;

    // Mouse events (the actual click)
    let button_idx = button.as_button_index();
    super::cdp::dispatch_mouse_action(page, x, y, button_idx, "mousedown").await?;

    // Brief press duration - real humans don't release immediately
    crate::utils::timing::human_pause(80, 25).await;

    super::cdp::dispatch_mouse_action(page, x, y, button_idx, "mouseup").await?;

    // Fire pointerout after click (cleanup)
    let _ = super::cdp::dispatch_pointer_event(page, "pointerout", x, y, button).await;

    Ok(())
}
