// Integration tests for navigation.rs functions
// Requires TASK_API_TEST_WS environment variable with browser WebSocket URL
//
// NOTE: This project supports Brave, Chrome, and Roxybrowser for browser automation.
// To run these tests and achieve 90%+ coverage:
//
// 1. Start a browser with remote debugging:
//    Brave: brave --remote-debugging-port=9222
//    Chrome: "C:\Program Files\Google\Chrome\Application\chrome.exe" --remote-debugging-port=9222
//    Roxybrowser: roxybrowser --remote-debugging-port=9222
//
// 2. Set the environment variable:
//    set TASK_API_TEST_WS=ws://localhost:9222
//
// 3. Run the tests without --ignored flag:
//    cargo test --test navigation_integration -- --ignored

use anyhow::Result;
use chromiumoxide::Browser;
use futures::StreamExt;
use std::env;

async fn connect_test_browser() -> Result<Browser> {
    let cdp_url = env::var("TASK_API_TEST_WS")
        .map_err(|_| anyhow::anyhow!("TASK_API_TEST_WS environment variable not set"))?;

    // Fetch the actual WebSocket URL from the CDP version endpoint
    let client = reqwest::Client::new();
    let response = client
        .get(format!(
            "{}/json/version",
            cdp_url.replace("ws://", "http://")
        ))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query CDP endpoint: {}", e))?;

    let version_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse CDP response: {}", e))?;

    let ws_url = version_data
        .get("webSocketDebuggerUrl")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No webSocketDebuggerUrl in CDP response"))?;

    let (browser, mut handler) = Browser::connect(ws_url).await?;

    // Spawn the handler task to keep it running
    tokio::task::spawn(async move { while let Some(_event) = handler.next().await {} });

    Ok(browser)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_goto_raw_timeout() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        // Test timeout behavior with invalid URL
        let result = auto::utils::navigation::goto_raw(&page, "about:blank", 100).await;
        assert!(result.is_ok());

        // Don't close browser - just close page
        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_wait_for_selector_timeout() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        // Test timeout with non-existent selector
        let result = auto::utils::navigation::wait_for_selector(&page, "#nonexistent", 100).await;
        if let Err(ref e) = result {
            eprintln!("wait_for_selector error: {}", e);
        }
        assert!(
            result.is_ok(),
            "wait_for_selector returned error: {:?}",
            result
        );
        assert!(!result.unwrap());

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_wait_for_visible_selector_timeout() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        // Test timeout with non-existent selector
        let result =
            auto::utils::navigation::wait_for_visible_selector(&page, "#nonexistent", 100).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_wait_for_any_visible_selector_timeout() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        // Test timeout with non-existent selectors
        let selectors = ["#a", "#b", "#c"];
        let result =
            auto::utils::navigation::wait_for_any_visible_selector(&page, &selectors, 100).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_selector_exists_css() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        // Test with body selector (always exists)
        let result = auto::utils::navigation::selector_exists(&page, "body").await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test with non-existent selector
        let result = auto::utils::navigation::selector_exists(&page, "#nonexistent").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_selector_is_visible_css() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        // Test with body selector (always visible)
        let result = auto::utils::navigation::selector_is_visible(&page, "body").await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test with non-existent selector
        let result = auto::utils::navigation::selector_is_visible(&page, "#nonexistent").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_page_url() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        let url = auto::utils::navigation::page_url(&page).await;
        assert!(url.is_ok());
        assert!(url.unwrap().contains("about:blank"));

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_page_title() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        let title = auto::utils::navigation::page_title(&page).await;
        assert!(title.is_ok());

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_focus() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        // Test focus on body (should succeed)
        let result = auto::utils::navigation::focus(&page, "body").await;
        assert!(result.is_ok());

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_go_back() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        // Test go_back (should succeed even if no history)
        let result = auto::utils::navigation::go_back(&page).await;
        assert!(result.is_ok());

        let _ = page.close().await;
    }
}
