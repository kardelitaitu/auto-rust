// Integration tests for TaskContext locator methods
// Requires TASK_API_TEST_WS environment variable with browser WebSocket URL
//
// 1. Start a browser with remote debugging:
//    Brave: brave --remote-debugging-port=9002
//    Chrome: "C:\Program Files\Google\Chrome\Application\chrome.exe" --remote-debugging-port=9222
//
// 2. Set the environment variable:
//    set TASK_API_TEST_WS=ws://localhost:9002
//
// 3. Run the tests:
//    cargo test --test task_context_integration -- --ignored --test-threads=1

use anyhow::Result;
use chromiumoxide::Browser;
use futures::StreamExt;
use std::env;

async fn connect_test_browser() -> Result<Browser> {
    let cdp_url = env::var("TASK_API_TEST_WS")
        .map_err(|_| anyhow::anyhow!("TASK_API_TEST_WS environment variable not set"))?;

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

    tokio::task::spawn(async move { while let Some(_event) = handler.next().await {} });

    Ok(browser)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_query_exists_with_body() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        // Test body exists
        let result = auto::runtime::task_context::query::exists(&page, "body").await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test non-existent selector
        let result = auto::runtime::task_context::query::exists(&page, "#nonexistent").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_query_visible_with_body() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        // Test body is visible
        let result = auto::runtime::task_context::query::visible(&page, "body").await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test non-existent selector
        let result = auto::runtime::task_context::query::visible(&page, "#nonexistent").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_query_text() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser
            .new_page("data:text/html,<div id='test'>Hello World</div>")
            .await
            .unwrap();

        // Test text extraction
        let result = auto::runtime::task_context::query::text(&page, "#test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("Hello World".to_string()));

        // Test non-existent selector
        let result = auto::runtime::task_context::query::text(&page, "#nonexistent").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_query_html() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser
            .new_page("data:text/html,<div id='test'><span>Content</span></div>")
            .await
            .unwrap();

        // Test HTML extraction
        let result = auto::runtime::task_context::query::html(&page, "#test").await;
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.is_some());
        assert!(html.unwrap().contains("span"));

        // Test non-existent selector
        let result = auto::runtime::task_context::query::html(&page, "#nonexistent").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_query_attr() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser
            .new_page("data:text/html,<div id='test' class='my-class' data-value='123'></div>")
            .await
            .unwrap();

        // Test attribute extraction
        let result = auto::runtime::task_context::query::attr(&page, "#test", "class").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("my-class".to_string()));

        let result = auto::runtime::task_context::query::attr(&page, "#test", "data-value").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("123".to_string()));

        // Test non-existent attribute
        let result = auto::runtime::task_context::query::attr(&page, "#test", "nonexistent").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        // Test non-existent selector
        let result = auto::runtime::task_context::query::attr(&page, "#nonexistent", "class").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_query_value() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser
            .new_page("data:text/html,<input id='test' value='input-value'>")
            .await
            .unwrap();

        // Test value extraction
        let result = auto::runtime::task_context::query::value(&page, "#test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some("input-value".to_string()));

        // Test non-existent selector
        let result = auto::runtime::task_context::query::value(&page, "#nonexistent").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_query_wait_for() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        // Test wait for body (should exist immediately)
        let result = auto::runtime::task_context::query::wait_for(&page, "body", 1000).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test wait for non-existent (should timeout and return false)
        let result = auto::runtime::task_context::query::wait_for(&page, "#nonexistent", 100).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_query_wait_for_visible() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        // Test wait for visible body
        let result =
            auto::runtime::task_context::query::wait_for_visible(&page, "body", 1000).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Test wait for non-existent (should timeout and return false)
        let result =
            auto::runtime::task_context::query::wait_for_visible(&page, "#nonexistent", 100).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_query_url() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser.new_page("about:blank").await.unwrap();

        let result = auto::runtime::task_context::query::url(&page).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("about:blank"));

        let _ = page.close().await;
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_query_title() {
        let browser = connect_test_browser().await.unwrap();
        let page = browser
            .new_page("data:text/html,<title>Test Page</title><body></body>")
            .await
            .unwrap();

        let result = auto::runtime::task_context::query::title(&page).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Test Page");

        let _ = page.close().await;
    }
}
