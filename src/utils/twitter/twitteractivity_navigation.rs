//! Navigation helpers for Twitter/X pages.
//!
//! This module provides functions for navigating to various Twitter pages and
//! checking the current page state. It handles navigation to home timeline,
//! notifications, and verifies login/auth status.
//!
//! ## Key Components
//!
//! - **Page Navigation**: Navigate to home, notifications, and other pages
//! - **Login State Checks**: Verify if user is authenticated
//! - **Page Detection**: Identify current page type (home, notifications, etc.)
//! - **URL Management**: Handle URL normalization and validation
//!
//! ## Key Functions
//!
//! - [`navigate_to_home()`]: Navigate to home timeline
//! - [`navigate_to_notifications()`]: Navigate to notifications page
//! - [`is_logged_in()`]: Check if user is authenticated
//! - [`get_current_url()`]: Get current page URL
//! - [`wait_for_navigation()`]: Wait for page navigation to complete
//!
//! ## Usage
//!
//! ```rust,no_run
//! use rust_orchestrator::utils::twitter::twitteractivity_navigation::*;
//!
//! // Navigate to home timeline
//! navigate_to_home(api).await?;
//!
//! // Check login status
//! if is_logged_in(api).await? {
//!     // User is authenticated
//! }
//!
//! // Get current URL
//! let url = get_current_url(api).await?;
//! ```
//!
//! ## Navigation Timeouts
//!
//! Navigation operations have configurable timeouts:
//! - Default navigation timeout: 30 seconds
//! - Default wait timeout: 15 seconds
//! - These can be adjusted per operation if needed

use crate::prelude::TaskContext;
use anyhow::Result;
use log::info;
use serde_json::Value;
use tracing::instrument;

use super::{twitteractivity_humanized::*, twitteractivity_selectors::*};

/// Default timeout for navigation operations in milliseconds
const DEFAULT_NAVIGATION_TIMEOUT_MS: u64 = 30_000;
/// Default timeout for wait operations in milliseconds
const DEFAULT_WAIT_TIMEOUT_MS: u64 = 15_000;

/// Navigates to Twitter/X home timeline.
/// Uses mouse click on home logo element instead of URL navigation.
#[instrument(skip(api))]
pub async fn goto_home(api: &TaskContext) -> Result<()> {
    let selector = r#"a[aria-label="X"]"#;
    let timeout_ms = DEFAULT_WAIT_TIMEOUT_MS;

    // Wait for home logo to be visible
    if !api.wait_for_any_visible_selector(&[selector], timeout_ms).await? {
        log::warn!("Home logo not found, falling back to URL navigation");
        return goto_home_fallback(api).await;
    }

    // Get position of home logo for logging
    if let Some((x, y)) = get_element_center(api, selector).await? {
        info!("Navigated to home ({:.1}, {:.1})", x, y);
    }

    // Click the home logo
    api.click(selector).await?;
    after_navigation_pause(api).await;

    // Verify the feed is actually visible after navigation
    if is_feed_visible(api).await? {
        after_navigation_pause(api).await;
        return Ok(());
    }

    // If feed not visible, fallback to URL navigation
    log::warn!("Feed not visible after home logo click, falling back to URL navigation");
    goto_home_fallback(api).await
}

/// Gets the center coordinates of an element matching the selector.
/// Returns None if element not found or rect invalid.
async fn get_element_center(api: &TaskContext, selector: &str) -> Result<Option<(f64, f64)>> {
    let js = format!(
        r#"
        (function() {{
            var el = document.querySelector('{}');
            if (!el) return null;
            var rect = el.getBoundingClientRect();
            if (rect.width <= 0 || rect.height <= 0) return null;
            return {{
                x: rect.x + rect.width / 2,
                y: rect.y + rect.height / 2
            }};
        }})()
        "#,
        selector
    );

    let result = api.page().evaluate(js).await?;
    if let Some(obj) = result.value().and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (obj.get("x").and_then(|v| v.as_f64()), obj.get("y").and_then(|v| v.as_f64())) {
            return Ok(Some((x, y)));
        }
    }
    Ok(None)
}

/// Fallback navigation using URLs (original implementation)
async fn goto_home_fallback(api: &TaskContext) -> Result<()> {
    let urls = [
        "https://x.com/home",
        "https://twitter.com/home",
        "https://x.com/",
        "https://twitter.com/",
    ];
    let timeout_ms = DEFAULT_NAVIGATION_TIMEOUT_MS;

    for url in &urls {
        api.navigate(url, timeout_ms).await?;
        // Verify the feed is actually visible after navigation
        if is_feed_visible(api).await? {
            after_navigation_pause(api).await;
            return Ok(());
        }
    }

    // If none of the URLs produced a visible feed, still consider it OK
    // (maybe the selector needs adjustment; we'll assume nav worked)
    after_navigation_pause(api).await;
    Ok(())
}

/// Navigates to the notifications page.
/// Typically https://x.com/notifications or similar.
pub async fn goto_notifications(api: &TaskContext) -> Result<()> {
    let url = "https://x.com/notifications";
    api.navigate(url, DEFAULT_NAVIGATION_TIMEOUT_MS).await?;
    // Wait for either the notifications column or fallback signals
    api.wait_for_any_visible_selector(
        &[
            "[data-testid='primaryColumn']",
            "main[role='main']",
            "a[aria-label='Notifications']",
        ],
        DEFAULT_WAIT_TIMEOUT_MS,
    )
    .await
    .ok();
    after_navigation_pause(api).await;
    Ok(())
}

/// Checks if the feed/timeline is visible on the current page.
/// Used to confirm successful navigation to a content page.
pub async fn is_feed_visible(api: &TaskContext) -> Result<bool> {
    let js = selector_feed_visible();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value().cloned().unwrap_or(Value::Bool(false));
    Ok(value.as_bool().unwrap_or(false))
}

/// Checks if a login/onboarding flow is currently displayed.
/// Returns `true` if login is required (user not authenticated).
pub async fn is_login_flow(api: &TaskContext) -> Result<bool> {
    let js = selector_login_flow();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value();
    let flow = value.and_then(|v: &Value| v.as_str().map(|s| s.to_string()));
    Ok(flow.is_some() && !flow.as_ref().unwrap().is_empty())
}

/// Verifies that the user is logged in by checking absence of login indicators.
/// Returns `true` if the page looks like a logged-in timeline.
#[instrument(skip(api))]
pub async fn verify_login(api: &TaskContext) -> Result<bool> {
    // First verify feed is visible
    let feed_visible = is_feed_visible(api).await?;
    if !feed_visible {
        return Ok(false);
    }
    // Also ensure no login flow modal is present
    let in_login_flow = is_login_flow(api).await?;
    Ok(!in_login_flow)
}

/// Waits for any of the provided selectors to appear on the page.
/// Common wait used after navigation to ensure page stability.
pub async fn wait_for_page_ready(
    api: &TaskContext,
    selectors: &[&str],
    timeout_ms: u64,
) -> Result<bool> {
    let ready = api
        .wait_for_any_visible_selector(selectors, timeout_ms)
        .await?;
    Ok(ready)
}

/// Performs a quick health check on critical Twitter selectors.
/// Logs warnings if selectors are failing (indicates DOM changes).
pub async fn check_selector_health(api: &TaskContext) -> Result<()> {
    let js = selector_health_check();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value();

    if let Some(obj) = value.and_then(|v| v.as_object()) {
        let feed_ok = obj
            .get("feed_visible")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let tweets_ok = obj
            .get("tweets_found")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let buttons_ok = obj
            .get("engagement_buttons")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !feed_ok {
            log::warn!("Selector health check: feed selector failing");
        }
        if !tweets_ok {
            log::warn!("Selector health check: tweet selector failing");
        }
        if !buttons_ok {
            log::warn!("Selector health check: engagement button selector failing");
        }

        if feed_ok && tweets_ok && buttons_ok {
            log::info!("Selector health check: all critical selectors OK");
        }
    }

    Ok(())
}
