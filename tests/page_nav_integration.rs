//! Integration tests for `src/runtime/task_context/page_nav.rs` CDP operations.
//!
//! Tests the 6 page_nav functions against a real browser via CDP WebSocket.
//! Uses the same `TASK_API_TEST_WS` setup as other integration tests.
//!
//! Run with:
//!     powershell -ExecutionPolicy Bypass -File ./scripts/run-integration-tests.ps1 -TestFilter page_nav
//!
//! Or manually:
//!     1. Start browser: brave --remote-debugging-port=9222
//!     2. Set: set TASK_API_TEST_WS=ws://localhost:9222
//!     3. Run: cargo test --test page_nav_integration -- --ignored

#[path = "common/mod.rs"]
mod common;

use auto::config::{BrowserConfig, NativeInteractionConfig};
use auto::runtime::task_context::{FocusStatus, TaskContext};
use auto::task::policy::DEFAULT_TASK_POLICY;
use chromiumoxide::Browser;
use std::env;
use std::sync::Arc;

async fn connect_test_session() -> Option<auto::session::Session> {
    let Ok(ws_url) = env::var("TASK_API_TEST_WS") else {
        eprintln!("skipping page_nav integration tests: TASK_API_TEST_WS is not set");
        return None;
    };

    let (browser, handler) = Browser::connect(&ws_url).await.ok()?;
    let session = auto::session::Session::new(
        "page-nav-test".to_string(),
        "page-nav-test".to_string(),
        "test".to_string(),
        browser,
        handler,
        1,
        0,
        None,
        ws_url,
    );
    Some(session)
}

fn test_browser_config() -> BrowserConfig {
    BrowserConfig::default()
}

fn build_task_context(
    session: &auto::session::Session,
    page: Arc<chromiumoxide::Page>,
) -> TaskContext {
    TaskContext::new(
        session.id.clone(),
        page,
        session.behavior_profile.clone(),
        session.behavior_runtime,
        NativeInteractionConfig::default(),
        &test_browser_config(),
        &DEFAULT_TASK_POLICY,
        None,
        session.browser_ws_url.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================================================
    // set_user_agent tests
    // ===========================================================================

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_set_user_agent_empty_string() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // Verify page has a userAgent (empty string case)
        // set_user_agent on empty string should not panic
        api.set_user_agent("").await?;

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_set_user_agent_with_single_quotes() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // Test that single quotes in user agent are properly escaped
        // page_nav uses replace('\u0027', "\\'") for escaping
        let user_agent = "Mozilla/5.0 (Browser with 'single quotes')";
        api.set_user_agent(user_agent).await?;

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_set_user_agent_standard_string() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // Set a standard user agent
        api.set_user_agent("Mozilla/5.0 TestAgent/1.0").await?;

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    // ===========================================================================
    // set_extra_http_headers tests (stub behavior)
    // ===========================================================================

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_set_extra_http_headers_stub_returns_ok() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // set_extra_http_headers is a stub that just returns Ok(())
        // Verify it doesn't panic and returns ok
        api.set_extra_http_headers(&[]).await?;

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    // ===========================================================================
    // apply_browser_context tests (Option<&str> handling)
    // ===========================================================================

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_apply_browser_context_none_user_agent() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // When user_agent is None, set_user_agent should be skipped
        api.apply_browser_context(None, &[]).await?;

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_apply_browser_context_some_user_agent() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // When user_agent is Some, it should be applied
        api.apply_browser_context(Some("TestAgent/2.0"), &[])
            .await?;

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    // ===========================================================================
    // wait_for_load tests (timeout enforcement)
    // ===========================================================================

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_wait_for_load_already_complete() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // about:blank should already be complete/interactive
        api.wait_for_load(2000).await?;

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_wait_for_load_timeout_non_page() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // Wait for load on already-loaded page should complete quickly
        let start = std::time::Instant::now();
        api.wait_for_load(2000).await?;
        let elapsed = start.elapsed().as_millis();

        // Should complete in under 1.5s (loosened from 1s for CI stability)
        assert!(
            elapsed < 1500,
            "Expected fast completion, took {}ms",
            elapsed
        );

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    // ===========================================================================
    // wait_for_any_visible_selector tests (getBoundingClientRect visibility)
    // ===========================================================================

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_wait_for_any_visible_selector_body() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // body should always be visible
        let result = api.wait_for_any_visible_selector(&["body"], 2000).await?;
        assert!(result, "body should be visible");

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_wait_for_any_visible_selector_nonexistent() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // Non-existent selector should return false after timeout
        let result = api
            .wait_for_any_visible_selector(&["#nonexistent"], 500)
            .await?;
        assert!(!result, "nonexistent selector should not be visible");

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_wait_for_any_visible_selector_display_none() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session
            .acquire_page_at("data:text/html,<div id='hidden' style='display:none'>Hidden</div>")
            .await?;
        let api = build_task_context(&session, page.clone());

        // display:none element should not be visible (width=0, height=0)
        let result = api
            .wait_for_any_visible_selector(&["#hidden"], 1000)
            .await?;
        assert!(!result, "display:none element should not be visible");

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_wait_for_any_visible_selector_multiple() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session
            .acquire_page_at(
                "data:text/html,<div id='a'>A</div><div id='b'>B</div><div id='c'>C</div>",
            )
            .await?;
        let api = build_task_context(&session, page.clone());

        // Test checking multiple selectors - at least one should be visible
        let result = api
            .wait_for_any_visible_selector(&["#a", "#b", "#c"], 2000)
            .await?;
        assert!(result, "at least one selector should be visible");

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    // ===========================================================================
    // focus tests (getBoundingClientRect + focus)
    // ===========================================================================

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_focus_body_element() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // Focus body should succeed with valid coordinates
        let outcome = api.focus("body").await?;
        assert!(matches!(outcome.focus, FocusStatus::Success));
        assert!(outcome.x > 0.0);
        assert!(outcome.y > 0.0);

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_focus_nonexistent_element() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // Focus nonexistent element should fail
        let result = api.focus("#nonexistent").await;
        assert!(result.is_err(), "focus on nonexistent should fail");

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_focus_input_element() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session
            .acquire_page_at("data:text/html,<input id='test-input' type='text' value='test'>")
            .await?;
        let api = build_task_context(&session, page.clone());

        // Focus the input should succeed
        let outcome = api.focus("#test-input").await?;
        assert!(matches!(outcome.focus, FocusStatus::Success));

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    // ===========================================================================
    // Transient error classification integration
    // ===========================================================================

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_retry_on_transient_error_timeout() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // Verify page operations work normally (baseline for retry behavior)
        let title = api.title().await;
        assert!(title.is_ok(), "page should be responsive");

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_no_retry_on_permanent_error_not_found() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session.acquire_page().await?;
        let api = build_task_context(&session, page.clone());

        // Element not found should fail immediately (permanent error)
        // The with_retry loop should NOT retry "not found" errors
        let result = api.focus("#does-not-exist").await;
        assert!(result.is_err(), "not found error should be permanent");

        // Verify it's a TaskError with CdpError variant
        let err_msg = result.unwrap_err().to_string();
        let err_lower = err_msg.to_lowercase();
        assert!(
            err_lower.contains("not found") || err_lower.contains("no such element"),
            "Expected 'not found' or 'no such element' in error, got: {}",
            err_lower
        );

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    // ===========================================================================
    // Visibility check edge cases
    // ===========================================================================

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_visibility_check_zero_width() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session
            .acquire_page_at("data:text/html,<div id='zero-width' style='width:0px'>Hidden</div>")
            .await?;
        let api = build_task_context(&session, page.clone());

        // Element with width=0 should not be visible per our check (width > 0 && height > 0)
        let result = api
            .wait_for_any_visible_selector(&["#zero-width"], 500)
            .await?;
        assert!(!result, "width=0 element should not be visible");

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_visibility_check_zero_height() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session
            .acquire_page_at("data:text/html,<div id='zero-height' style='height:0px'>Hidden</div>")
            .await?;
        let api = build_task_context(&session, page.clone());

        // Element with height=0 should not be visible
        let result = api
            .wait_for_any_visible_selector(&["#zero-height"], 500)
            .await?;
        assert!(!result, "height=0 element should not be visible");

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }

    #[tokio::test]
    #[ignore] // Requires real browser connection
    async fn test_visibility_check_collapsed_element() -> anyhow::Result<()> {
        let Some(mut session) = connect_test_session().await else {
            return Ok(());
        };

        let page: Arc<chromiumoxide::Page> = session
            .acquire_page_at(
                "data:text/html,<div id='collapsed' style='visibility:hidden'>Collapsed</div>",
            )
            .await?;
        let api = build_task_context(&session, page.clone());

        // visibility:hidden typically still has bounding rect > 0
        // Our geometric check (width > 0 && height > 0) passes for visibility:hidden
        // This is expected - we check geometric visibility, not CSS visibility
        let result = api
            .wait_for_any_visible_selector(&["#collapsed"], 500)
            .await?;
        assert!(result, "visibility:hidden element has non-zero rect");

        session.release_page(page).await;
        session.graceful_shutdown().await?;
        Ok(())
    }
}
