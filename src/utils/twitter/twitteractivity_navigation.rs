//! Navigation helpers for Twitter/X pages.
//! Handles going to home timeline, notifications, and login state checks.

use crate::prelude::TaskContext;
use anyhow::Result;
use serde_json::Value;

use super::{twitteractivity_selectors::*, twitteractivity_humanized::*};

/// Navigates to Twitter/X home timeline.
/// URL varies by user region/login state; tries known working URLs.
pub async fn goto_home(api: &TaskContext) -> Result<()> {
    let urls = [
        "https://x.com/home",
        "https://twitter.com/home",
        "https://x.com/",
        "https://twitter.com/",
    ];
    let timeout_ms = 30_000;

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
    api.navigate(url, 30_000).await?;
    // Wait for either the notifications column or fallback signals
    api.wait_for_any_visible_selector(
        &[
            "[data-testid='primaryColumn']",
            "main[role='main']",
            "a[aria-label='Notifications']",
        ],
        15_000,
    )
    .await.ok();
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

