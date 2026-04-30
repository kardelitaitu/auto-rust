//! DOM query and inspection methods for TaskContext.
//!
//! Provides methods for checking element existence, visibility,
//! extracting content, and waiting for elements.

use anyhow::Result;
use chromiumoxide::Page;

/// Check if selector exists in DOM (may be hidden).
pub async fn exists(page: &Page, selector: &str) -> Result<bool> {
    crate::capabilities::navigation::selector_exists(page, selector).await
}

/// Check if selector is visible (displayed and not hidden).
pub async fn visible(page: &Page, selector: &str) -> Result<bool> {
    crate::capabilities::navigation::selector_is_visible(page, selector).await
}

/// Get text content of selector. Returns None if not found.
pub async fn text(page: &Page, selector: &str) -> Result<Option<String>> {
    crate::capabilities::navigation::selector_text(page, selector).await
}

/// Get inner HTML of selector. Returns None if not found.
pub async fn html(page: &Page, selector: &str) -> Result<Option<String>> {
    crate::capabilities::navigation::selector_html(page, selector).await
}

/// Get element attribute by name. Returns None if not found.
pub async fn attr(page: &Page, selector: &str, name: &str) -> Result<Option<String>> {
    crate::capabilities::navigation::selector_attr(page, selector, name).await
}

/// Get input/textarea value attribute. Returns None if not found.
pub async fn value(page: &Page, selector: &str) -> Result<Option<String>> {
    crate::capabilities::navigation::selector_value(page, selector).await
}

/// Wait for selector to exist in DOM. Returns true if found within timeout.
pub async fn wait_for(page: &Page, selector: &str, timeout_ms: u64) -> Result<bool> {
    crate::capabilities::navigation::wait_for_selector(page, selector, timeout_ms).await
}

/// Wait for selector to be visible. Returns true if visible within timeout.
pub async fn wait_for_visible(page: &Page, selector: &str, timeout_ms: u64) -> Result<bool> {
    crate::capabilities::navigation::wait_for_visible_selector(page, selector, timeout_ms).await
}

/// Wait until any of the given selectors becomes visible. Returns first match or false.
pub async fn wait_for_any_visible(
    page: &Page,
    selectors: &[&str],
    timeout_ms: u64,
) -> Result<bool> {
    crate::capabilities::navigation::wait_for_any_visible_selector(page, selectors, timeout_ms)
        .await
}

/// Get current page URL.
pub async fn url(page: &Page) -> Result<String> {
    crate::capabilities::navigation::page_url(page).await
}

/// Get page title from DOM.
pub async fn title(page: &Page) -> Result<String> {
    crate::capabilities::navigation::page_title(page).await
}

/// Get viewport dimensions.
pub async fn viewport(page: &Page) -> Result<crate::internal::page_size::Viewport> {
    crate::internal::page_size::get_viewport(page).await
}
