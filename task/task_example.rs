//! Example task template for browser automation.
//!
//! This file demonstrates the structure and patterns for creating new tasks:
//! - Configuration from payload with defaults
//! - Navigation with timeout
//! - Element interaction (click, keyboard, wait)
//! - Verification and assertions
//! - Proper error handling and logging

use anyhow::{bail, Result};
use log::{info, warn};
use serde_json::Value;
use std::time::Duration;
use tokio::time::timeout;

use crate::prelude::TaskContext;

// ============================================================================
// Configuration
// ============================================================================

/// Default target URL for the example task
const DEFAULT_URL: &str = "https://example.com";

/// Enable cursor overlay for debugging/demo purposes
const SHOW_CURSOR_OVERLAY: bool = true;

/// Navigation timeout in milliseconds
const NAVIGATION_TIMEOUT_MS: u64 = 30_000;

/// Element visibility timeout in milliseconds
const VISIBILITY_TIMEOUT_MS: u64 = 15_000;

// ============================================================================
// Task Entry Point
// ============================================================================

/// Main task entry point.
///
/// # Arguments
/// * `api` - TaskContext for browser automation
/// * `payload` - JSON payload with task parameters
///
/// # Returns
/// * `Ok(())` - Task completed successfully
/// * `Err(e)` - Task failed with error
pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    info!("Example task started");

    // Parse configuration from payload (with defaults)
    let config = ExampleConfig::from_payload(&payload)?;

    // Log task parameters
    info!("Target URL: {}", config.url);
    info!("Username: {}", config.username);
    info!("Action: {}", config.action);

    // Enable cursor overlay for visibility (optional)
    if SHOW_CURSOR_OVERLAY {
        crate::capabilities::mouse::set_overlay_enabled(true);
    }

    // Navigate to target URL
    info!("Navigating to: {}", config.url);
    api.navigate(&config.url, NAVIGATION_TIMEOUT_MS).await?;

    // Sync cursor overlay if enabled
    if SHOW_CURSOR_OVERLAY {
        api.sync_cursor_overlay().await?;
    }

    // Wait for page to be ready
    if let Err(e) = api
        .wait_for_visible("#main-content", VISIBILITY_TIMEOUT_MS)
        .await
    {
        warn!("Main content did not become visible: {}", e);
    }

    // Get page title for logging
    let title = api.title().await.unwrap_or_else(|_| "unknown".to_string());
    info!("Page title: {}", title);

    // Perform the configured action
    match config.action.as_str() {
        "click" => {
            info!("Performing click action");
            perform_click_action(api, &config.target_element).await?;
        }
        "type" => {
            info!("Performing type action");
            perform_type_action(api, &config.target_element, &config.input_value).await?;
        }
        "verify" => {
            info!("Performing verification");
            perform_verification(api, &config.target_element, &config.expected_text).await?;
        }
        _ => {
            info!("No specific action configured, demonstrating basic interactions");
            demonstrate_basic_interactions(api).await?;
        }
    }

    // Final pause before completion (for demo purposes)
    api.pause(2000).await;

    info!("Example task completed");
    Ok(())
}

// ============================================================================
// Action Implementations
// ============================================================================

/// Demonstrates clicking an element with proper error handling
async fn perform_click_action(api: &TaskContext, selector: &str) -> Result<()> {
    info!("Clicking element: {}", selector);

    let click_result = match timeout(Duration::from_secs(12), api.click(selector)).await {
        Ok(Ok(outcome)) => outcome,
        Ok(Err(e)) => {
            warn!("Click failed: {}", e);
            return Err(e);
        }
        Err(_) => {
            let e = anyhow::anyhow!("Click timed out after 12s");
            warn!("{}", e);
            return Err(e);
        }
    };

    info!("Click result: {}", click_result.summary());
    Ok(())
}

/// Demonstrates typing text into an input field
async fn perform_type_action(api: &TaskContext, selector: &str, value: &str) -> Result<()> {
    info!("Typing into element: {}", selector);

    // Click to focus
    let focus_result = api.click(selector).await?;
    info!("Focus: {}", focus_result.summary());

    // Clear existing content
    api.clear(selector).await?;

    // Type the value
    api.keyboard(selector, value).await?;

    // Verify the value was entered
    let actual_value = api.value(selector).await?.unwrap_or_default();
    info!("Entered value: {}", actual_value);

    Ok(())
}

/// Demonstrates verification of element content
async fn perform_verification(api: &TaskContext, selector: &str, expected: &str) -> Result<()> {
    info!("Verifying element: {}", selector);

    let text = api.text(selector).await?.unwrap_or_default();
    info!("Element text: {}", text);

    if !text.contains(expected) {
        bail!("Verification failed: expected '{}' in '{}'", expected, text);
    }

    info!("Verification passed: found '{}'", expected);
    Ok(())
}

/// Demonstrates basic browser interactions
async fn demonstrate_basic_interactions(api: &TaskContext) -> Result<()> {
    info!("Demonstrating basic interactions");

    // Example: scroll to top
    api.scroll_to_top().await?;
    info!("Scrolled to top");

    // Example: pause
    api.pause(1000).await;

    // Example: scroll to bottom
    api.scroll_to_bottom().await?;
    info!("Scrolled to bottom");

    // Example: take a screenshot (if supported)
    // api.screenshot("example.png").await?;

    Ok(())
}

// ============================================================================
// Configuration Structure
// ============================================================================

/// Task configuration with defaults
struct ExampleConfig {
    /// Target URL to navigate to
    url: String,
    /// Username for demonstration
    username: String,
    /// Action to perform: "click", "type", "verify", or "auto"
    action: String,
    /// Target element selector for the action
    target_element: String,
    /// Input value for "type" action
    input_value: String,
    /// Expected text for "verify" action
    expected_text: String,
}

impl ExampleConfig {
    /// Parse configuration from JSON payload
    fn from_payload(payload: &Value) -> Result<Self> {
        Ok(Self {
            url: extract_url_from_payload(payload)?,
            username: read_string(payload, "username", "Demo User"),
            action: read_string(payload, "action", "auto"),
            target_element: read_string(payload, "target_element", "#main-content"),
            input_value: read_string(payload, "input_value", "Hello, World!"),
            expected_text: read_string(payload, "expected_text", "Example"),
        })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract URL from payload with fallback to default
fn extract_url_from_payload(payload: &Value) -> Result<String> {
    if let Some(value) = payload.get("url") {
        if let Some(url_str) = value.as_str() {
            return Ok(url_str.to_string());
        }
    }

    if let Some(value) = payload.get("value") {
        if let Some(url_str) = value.as_str() {
            return Ok(url_str.to_string());
        }
    }

    Ok(DEFAULT_URL.to_string())
}

/// Read string from payload with default value
fn read_string(payload: &Value, key: &str, default: &str) -> String {
    payload
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .unwrap_or_else(|| default.to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::{ExampleConfig, DEFAULT_URL};
    use serde_json::json;

    #[test]
    fn example_uses_defaults() {
        let config = ExampleConfig::from_payload(&json!({})).unwrap();
        assert_eq!(config.url, DEFAULT_URL);
        assert_eq!(config.username, "Demo User");
        assert_eq!(config.action, "auto");
    }

    #[test]
    fn example_accepts_custom_config() {
        let config = ExampleConfig::from_payload(&json!({
            "url": "https://example.com/custom",
            "username": "Test User",
            "action": "click",
            "target_element": "#custom-button",
            "input_value": "Custom input",
            "expected_text": "Custom text",
        }))
        .unwrap();

        assert_eq!(config.url, "https://example.com/custom");
        assert_eq!(config.username, "Test User");
        assert_eq!(config.action, "click");
        assert_eq!(config.target_element, "#custom-button");
    }

    #[test]
    fn example_handles_partial_payload() {
        let config = ExampleConfig::from_payload(&json!({
            "username": "Partial User",
        }))
        .unwrap();

        assert_eq!(config.username, "Partial User");
        assert_eq!(config.url, DEFAULT_URL);
        assert_eq!(config.action, "auto");
    }
}
