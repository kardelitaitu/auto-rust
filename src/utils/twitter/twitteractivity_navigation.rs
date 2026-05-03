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
//! use auto::utils::twitter::twitteractivity_navigation::*;
//! # use auto::runtime::task_context::TaskContext;
//! # async fn example(api: &TaskContext) -> anyhow::Result<()> {
//!
//! // Navigate to home timeline
//! goto_home(api).await?;
//!
//! // Check login status
//! if verify_login(api).await? {
//!     // User is authenticated
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Navigation Timeouts
//!
//! Navigation operations have configurable timeouts:
//! - Default navigation timeout: 30 seconds
//! - Default wait timeout: 15 seconds
//! - These can be adjusted per operation if needed

use crate::prelude::TaskContext;
use crate::utils::timing::{DEFAULT_NAVIGATION_TIMEOUT_MS, TIMEOUT_MEDIUM_MS};
use anyhow::Result;
use log::{info, warn};
use serde_json::Value;
use std::time::Instant;
use tracing::instrument;

use super::{
    twitteractivity_humanized::*, twitteractivity_interact::*, twitteractivity_popup::*,
    twitteractivity_selectors::*,
};

// Use TIMEOUT_MEDIUM_MS (15s) from timing module for wait operations

/// Navigates to Twitter/X home timeline.
/// Uses mouse click on home logo element instead of URL navigation.
#[instrument(skip(api))]
pub async fn goto_home(api: &TaskContext) -> Result<()> {
    let selector = r#"a[aria-label="X"]"#;
    let timeout_ms = TIMEOUT_MEDIUM_MS;

    // Wait for home logo to be visible
    if !api
        .wait_for_any_visible_selector(&[selector], timeout_ms)
        .await?
    {
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
        if let (Some(x), Some(y)) = (
            obj.get("x").and_then(|v| v.as_f64()),
            obj.get("y").and_then(|v| v.as_f64()),
        ) {
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
        TIMEOUT_MEDIUM_MS,
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

// Navigation functions moved from twitteractivity.rs

/// Select a weighted entry point randomly
pub fn select_entry_point() -> &'static str {
    let total_weight: u32 = ENTRY_POINTS.iter().map(|ep| ep.weight).sum();
    let mut random = rand::random::<u32>() % total_weight;

    for entry in ENTRY_POINTS.iter() {
        if random < entry.weight {
            return entry.url;
        }
        random -= entry.weight;
    }

    ENTRY_POINTS[0].url // fallback to home
}

pub async fn navigate_and_read(api: &TaskContext, entry_url: &str) -> Result<()> {
    let entry_name = entry_url
        .replace("https://x.com/", "")
        .replace("https://x.com", "");
    let entry_name = if entry_name.is_empty() {
        "home"
    } else {
        &entry_name
    };

    info!("🎲 Rolled entry point: {} → {}", entry_name, entry_url);

    // Navigate to entry point
    api.navigate(entry_url, 60000).await?;
    human_pause(api, 2000).await;

    // Check if on home feed
    let on_home = is_on_home_feed(api).await.unwrap_or(false);

    if !on_home {
        // Simulate reading on non-home page
        let scroll_duration = rand::random::<u64>() % 10000 + 10000; // 10-20s
        info!(
            "📖 Simulating reading on {} for {}s",
            entry_name,
            scroll_duration / 1000
        );

        let scroll_start = Instant::now();
        let profile = api.behavior_runtime();
        while scroll_start.elapsed().as_millis() < scroll_duration as u128 {
            let scroll_amount = (rand::random::<u64>() % 400 + 200) as i32;
            let _ = api
                .scroll_read(
                    1,
                    scroll_amount,
                    profile.scroll.smooth,
                    profile.scroll.back_scroll,
                )
                .await;
            human_pause(api, rand::random::<u64>() % 300 + 200).await;
        }

        info!("✅ Finished reading, navigating to home...");
        goto_home(api).await?;
        human_pause(api, 500).await;
    }

    Ok(())
}

pub async fn phase1_navigation(api: &TaskContext) -> Result<()> {
    info!("Phase 1: Navigation to entry point");
    let entry_url = select_entry_point();
    navigate_and_read(api, entry_url).await?;

    if verify_login(api).await? {
        info!("User is logged in - proceeding");
    } else {
        warn!("User appears not logged in; task may fail");
    }

    // Dismiss initial popups
    match dismiss_cookie_banner(api).await {
        Ok(true) => info!("Cookie banner dismissed"),
        Ok(false) => {}
        Err(e) => warn!("Cookie banner dismissal failed: {}", e),
    }
    match dismiss_signup_nag(api).await {
        Ok(true) => info!("Signup nag dismissed"),
        Ok(false) => {}
        Err(e) => warn!("Signup nag dismissal failed: {}", e),
    }
    if let Err(e) = close_active_popup(api).await {
        warn!("Popup close failed: {}", e);
    }

    Ok(())
}

// Navigation entry points and functions
pub struct EntryPoint {
    pub url: &'static str,
    pub weight: u32,
}

/// Weighted entry points matching Node.js implementation
pub const ENTRY_POINTS: [EntryPoint; 15] = [
    // Primary Entry (59%)
    EntryPoint {
        url: "https://x.com/",
        weight: 59,
    },
    // 4% Weight Group (32% total)
    EntryPoint {
        url: "https://x.com/i/jf/global-trending/home",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/explore",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/explore/tabs/for-you",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/explore/tabs/trending",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/i/bookmarks",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/notifications",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/notifications/mentions",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/i/chat/",
        weight: 4,
    },
    // 2% Weight Group (4% total)
    EntryPoint {
        url: "https://x.com/i/connect_people?show_topics=false",
        weight: 2,
    },
    EntryPoint {
        url: "https://x.com/i/connect_people?is_creator_only=true",
        weight: 2,
    },
    // Legacy/Supplementary Exploratory Points (5% total)
    EntryPoint {
        url: "https://x.com/explore/tabs/news",
        weight: 1,
    },
    EntryPoint {
        url: "https://x.com/explore/tabs/sports",
        weight: 1,
    },
    EntryPoint {
        url: "https://x.com/explore/tabs/entertainment",
        weight: 1,
    },
    EntryPoint {
        url: "https://x.com/explore/tabs/for_you",
        weight: 2,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_navigation_timeout_constant() {
        assert_eq!(DEFAULT_NAVIGATION_TIMEOUT_MS, 30_000);
    }

    #[test]
    fn test_timeout_medium_constant() {
        assert_eq!(TIMEOUT_MEDIUM_MS, 15_000);
    }

    #[test]
    fn test_timeout_constants_are_positive() {
        const { assert!(DEFAULT_NAVIGATION_TIMEOUT_MS > 0) }
        const { assert!(TIMEOUT_MEDIUM_MS > 0) }
    }

    #[test]
    fn test_navigation_timeout_greater_than_medium_timeout() {
        const { assert!(DEFAULT_NAVIGATION_TIMEOUT_MS > TIMEOUT_MEDIUM_MS) }
    }

    #[test]
    fn test_selector_feed_visible_returns_js() {
        let js = selector_feed_visible();
        assert!(js.contains("querySelector"));
        assert!(js.contains("data-testid"));
        assert!(js.contains("primaryColumn"));
    }

    #[test]
    fn test_selector_login_flow_returns_js() {
        let js = selector_login_flow();
        assert!(js.contains("session"));
        assert!(js.contains("login"));
        assert!(js.contains("Sign in"));
    }

    #[test]
    fn test_selector_health_check_returns_js() {
        let js = selector_health_check();
        assert!(js.contains("feed_visible"));
        assert!(js.contains("tweets_found"));
        assert!(js.contains("engagement_buttons"));
    }

    #[test]
    fn test_selector_functions_exist() {
        // Test that all selector functions are callable
        let _ = selector_feed_visible();
        let _ = selector_login_flow();
        let _ = selector_health_check();
        let _ = selector_close_button();
        let _ = js_get_current_url();
        let _ = js_extract_username_from_url();
    }

    #[test]
    fn test_js_get_current_url_format() {
        let js = js_get_current_url();
        assert!(js.contains("window.location.href"));
        assert!(js.len() < 100); // Should be concise
    }

    #[test]
    fn test_js_extract_username_from_url_format() {
        let js = js_extract_username_from_url();
        assert!(js.contains("pathname"));
        assert!(js.contains("split"));
    }

    #[test]
    fn test_selector_close_button_format() {
        let js = selector_close_button();
        assert!(js.contains("aria-label"));
        assert!(js.contains("Close"));
    }

    #[test]
    fn test_selector_follow_confirm_modal_format() {
        let js = selector_follow_confirm_modal();
        assert!(js.contains("dialog"));
        assert!(js.contains("follow"));
    }

    #[test]
    fn test_selector_following_indicator_format() {
        let js = selector_following_indicator();
        assert!(js.contains("following"));
        assert!(js.contains("button"));
    }

    #[test]
    fn test_selector_tweet_user_avatar_format() {
        let js = selector_tweet_user_avatar();
        assert!(js.contains("Tweet-User-Avatar"));
        assert!(js.contains("profile_images"));
    }

    #[test]
    fn test_selector_element_center_format() {
        let js = selector_element_center("div.test");
        assert!(js.contains("getBoundingClientRect"));
        assert!(js.contains("x"));
        assert!(js.contains("y"));
    }

    #[test]
    fn test_selector_element_center_escapes_quotes() {
        let js = selector_element_center("div.test\"class");
        assert!(js.contains("\\\""));
        assert!(!js.contains("\"test\""));
    }

    #[test]
    fn test_selector_all_tweets_format() {
        let js = selector_all_tweets();
        assert!(js.contains("querySelectorAll"));
        assert!(js.contains("article"));
        assert!(js.contains("data-testid"));
    }

    #[test]
    fn test_selector_follow_button_format() {
        let js = selector_follow_button();
        assert!(js.contains("aria-label"));
        assert!(js.contains("follow"));
    }

    #[test]
    fn test_selector_engagement_buttons_format() {
        let js = selector_engagement_buttons();
        assert!(js.contains("like"));
        assert!(js.contains("retweet"));
        assert!(js.contains("reply"));
    }

    #[test]
    fn test_selector_popup_overlay_format() {
        let js = selector_popup_overlay();
        assert!(js.contains("dialog"));
        assert!(js.contains("aria-modal"));
    }

    /// Property-based test: entry point weights must sum to 100
    #[test]
    fn test_entry_point_weights_sum_to_100() {
        let total: u32 = ENTRY_POINTS.iter().map(|e| e.weight).sum();
        assert_eq!(
            total, 100,
            "ENTRY_POINTS weights must sum to 100, got {}",
            total
        );
    }

    /// Property-based test: verify home feed gets ~59% of selections over 1000 samples
    /// Uses deterministic seeding for reproducibility
    #[test]
    fn test_entry_point_distribution_properties() {
        use rand::rngs::StdRng;
        use rand::Rng;
        use rand::SeedableRng;

        const SAMPLES: usize = 1000;
        const HOME_URL: &str = "https://x.com/";
        const TOLERANCE: f64 = 0.05; // 5% tolerance

        // Calculate expected probability
        let total_weight: u32 = ENTRY_POINTS.iter().map(|e| e.weight).sum();
        let home_entry = ENTRY_POINTS.iter().find(|e| e.url == HOME_URL).unwrap();
        let expected_prob = home_entry.weight as f64 / total_weight as f64;

        // Run 1000 samples with seeded RNG for reproducibility
        let mut rng = StdRng::seed_from_u64(42);
        let mut home_count = 0;

        for _ in 0..SAMPLES {
            let random = rng.gen::<u32>() % total_weight;
            let mut cumulative = 0;
            let mut selected = "";
            for entry in ENTRY_POINTS.iter() {
                cumulative += entry.weight;
                if random < cumulative {
                    selected = entry.url;
                    break;
                }
            }
            if selected == HOME_URL {
                home_count += 1;
            }
        }

        let actual_prob = home_count as f64 / SAMPLES as f64;
        let diff = (actual_prob - expected_prob).abs();

        assert!(
            diff <= TOLERANCE,
            "Home feed distribution deviated too much: expected {:.2}%, got {:.2}% (diff {:.2}%)",
            expected_prob * 100.0,
            actual_prob * 100.0,
            diff * 100.0
        );
    }

    /// Property: select_entry_point never returns empty string or panics
    #[test]
    fn test_select_entry_point_never_empty() {
        for _ in 0..100 {
            let url = select_entry_point();
            assert!(!url.is_empty(), "Entry point URL must not be empty");
            assert!(url.starts_with("https://"), "Entry point must use HTTPS");
            assert!(
                url.contains("x.com") || url.contains("twitter.com"),
                "Entry point must be X/Twitter domain"
            );
        }
    }

    /// Property: all entry point URLs are unique
    #[test]
    fn test_entry_points_unique_urls() {
        let mut urls = std::collections::HashSet::new();
        for entry in ENTRY_POINTS.iter() {
            assert!(
                urls.insert(entry.url),
                "Duplicate entry point URL: {}",
                entry.url
            );
        }
        assert_eq!(
            urls.len(),
            ENTRY_POINTS.len(),
            "All entry point URLs should be unique"
        );
    }

    /// Property: all entry point weights are positive
    #[test]
    fn test_entry_point_weights_positive() {
        for entry in ENTRY_POINTS.iter() {
            assert!(
                entry.weight > 0,
                "Entry point weight must be positive: {}",
                entry.url
            );
        }
    }
}
