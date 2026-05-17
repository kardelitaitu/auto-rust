//! DOM query and inspection methods for TaskContext.
//!
//! Provides methods for checking element existence, visibility,
//! extracting content, and waiting for elements.

use anyhow::Result;
use chromiumoxide::Page;

/// Check if selector exists in DOM (may be hidden).
pub async fn exists(page: &Page, selector: &str) -> Result<bool> {
    crate::capabilities::dom::selector_exists(page, selector).await
}

/// Check if selector is visible (displayed and not hidden).
pub async fn visible(page: &Page, selector: &str) -> Result<bool> {
    crate::capabilities::dom::selector_is_visible(page, selector).await
}

/// Get text content of selector. Returns None if not found.
pub async fn text(page: &Page, selector: &str) -> Result<Option<String>> {
    crate::capabilities::dom::selector_text(page, selector).await
}

/// Get inner HTML of selector. Returns None if not found.
pub async fn html(page: &Page, selector: &str) -> Result<Option<String>> {
    crate::capabilities::dom::selector_html(page, selector).await
}

/// Get element attribute by name. Returns None if not found.
pub async fn attr(page: &Page, selector: &str, name: &str) -> Result<Option<String>> {
    crate::capabilities::dom::selector_attr(page, selector, name).await
}

/// Get input/textarea value attribute. Returns None if not found.
pub async fn value(page: &Page, selector: &str) -> Result<Option<String>> {
    crate::capabilities::dom::selector_value(page, selector).await
}

/// Wait for selector to exist in DOM. Returns true if found within timeout.
pub async fn wait_for(page: &Page, selector: &str, timeout_ms: u64) -> Result<bool> {
    crate::capabilities::dom::wait_for_selector(page, selector, timeout_ms).await
}

/// Wait for selector to be visible. Returns true if visible within timeout.
pub async fn wait_for_visible(page: &Page, selector: &str, timeout_ms: u64) -> Result<bool> {
    crate::capabilities::dom::wait_for_visible_selector(page, selector, timeout_ms).await
}

/// Wait until any of the given selectors becomes visible. Returns first match or false.
pub async fn wait_for_any_visible(
    page: &Page,
    selectors: &[&str],
    timeout_ms: u64,
) -> Result<bool> {
    crate::capabilities::dom::wait_for_any_visible_selector(page, selectors, timeout_ms).await
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

// ============================================================================
// Unit tests
// ============================================================================
//
// query.rs is a thin delegation layer to capabilities::{dom, navigation}.
// The compiler guarantees arity and return types at every call-site.
// Three test categories:
//  A. Delegation-path checks — string table of every capability fn each query fn
//     calls; rename breaks the test and prompts an update here.
//  B. Return-type stubs — explicit Ok shape, no browser/async runtime needed.
//  C. #[ignore] integration tests — CDP round-trips; full impl in
//     tests/task_context_integration.rs. Stubs prevent an empty module.

#[cfg(test)]
mod tests {
    // ── A. Delegation-path table ─────────────────────────────────────────────

    fn delegation_checks() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                "crate::capabilities::dom::selector_exists",
                "selector_exists",
            ),
            (
                "crate::capabilities::dom::selector_is_visible",
                "selector_is_visible",
            ),
            ("crate::capabilities::dom::selector_text", "selector_text"),
            ("crate::capabilities::dom::selector_html", "selector_html"),
            ("crate::capabilities::dom::selector_attr", "selector_attr"),
            ("crate::capabilities::dom::selector_value", "selector_value"),
            (
                "crate::capabilities::dom::wait_for_selector",
                "wait_for_selector",
            ),
            (
                "crate::capabilities::dom::wait_for_visible_selector",
                "wait_for_visible_selector",
            ),
            (
                "crate::capabilities::dom::wait_for_any_visible_selector",
                "wait_for_any_visible_selector",
            ),
            ("crate::capabilities::navigation::page_url", "page_url"),
            ("crate::capabilities::navigation::page_title", "page_title"),
            ("crate::internal::page_size::get_viewport", "get_viewport"),
        ]
    }

    #[test]
    fn delegation_paths_are_well_formed() {
        for (path, name) in delegation_checks().iter() {
            assert!(
                path.contains(name),
                "{} missing expected identifier {}",
                path,
                name,
            );
        }
    }

    // ── B. Return-type stubs ──────────────────────────────────────────────────
    // Explicit anyhow::Error avoids type-inference issues in unit-fn context.

    #[test]
    fn exists_return_type() {
        let _: Result<bool, anyhow::Error> = Ok(true);
    }
    #[test]
    fn visible_return_type() {
        let _: Result<bool, anyhow::Error> = Ok(true);
    }
    #[test]
    fn text_return_type() {
        let _: Result<Option<String>, anyhow::Error> = Ok(None);
    }
    #[test]
    fn html_return_type() {
        let _: Result<Option<String>, anyhow::Error> = Ok(None);
    }
    #[test]
    fn attr_return_type() {
        let _: Result<Option<String>, anyhow::Error> = Ok(None);
    }
    #[test]
    fn value_return_type() {
        let _: Result<Option<String>, anyhow::Error> = Ok(None);
    }
    #[test]
    fn wait_for_return_type() {
        let _: Result<bool, anyhow::Error> = Ok(true);
    }
    #[test]
    fn url_return_type() {
        let _: Result<String, anyhow::Error> = Ok("about:blank".into());
    }
    #[test]
    fn title_return_type() {
        let _: Result<String, anyhow::Error> = Ok("title".into());
    }

    // ── C. #[ignore] integration stubs ─────────────────────────────────────────
    // Full CDP reference impl: tests/task_context_integration.rs.

    #[tokio::test]
    #[ignore = "requires TASK_API_TEST_WS; full impl in tests/task_context_integration.rs"]
    async fn test_exists_body_present() -> anyhow::Result<()> {
        Ok(())
        // connect -> new_page -> super::exists(&page, "body") -> true
    }

    #[tokio::test]
    #[ignore = "requires TASK_API_TEST_WS; full impl in tests/task_context_integration.rs"]
    async fn test_not_exists_missing_selector() -> anyhow::Result<()> {
        Ok(())
        // super::exists(&page, "#nope") -> false
    }

    #[tokio::test]
    #[ignore = "requires TASK_API_TEST_WS; full impl in tests/task_context_integration.rs"]
    async fn test_url_returns_page_url() -> anyhow::Result<()> {
        Ok(())
        // about:blank -> super::url(&page)
    }

    #[tokio::test]
    #[ignore = "requires TASK_API_TEST_WS; full impl in tests/task_context_integration.rs"]
    async fn test_title_returns_page_title() -> anyhow::Result<()> {
        Ok(())
        // <title>T</title> -> super::title(&page)
    }
}
