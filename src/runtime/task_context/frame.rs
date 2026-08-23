//! In-place iframe interaction for `TaskContext`.
//!
//! Cross-origin iframes (e.g. Telegram mini-apps) cannot be reached by CSS
//! selectors from the top document. This module interacts with them directly
//! through CDP: locate the iframe's own execution context via the frame tree,
//! evaluate JS inside the frame to find an element's position, then dispatch a
//! cursor click at the element's absolute viewport coordinates. CDP input
//! events are browser-level, so they work across origins — no navigation and
//! no new tabs are involved.

use anyhow::{anyhow, Result};
use chromiumoxide::cdp::browser_protocol::page::{FrameId, FrameTree, GetFrameTreeParams};
use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
use chromiumoxide::Page;
use std::time::{Duration, Instant};

use crate::capabilities::mouse;
use crate::runtime::task_context::TaskContext;
use crate::utils::{ClickOutcome, ClickStatus};

impl TaskContext {
    /// Click an element INSIDE an iframe, in place — no navigation, no new tab.
    ///
    /// The iframe (e.g. a cross-origin Telegram mini-app) is located in the top
    /// document, its frame execution context is resolved via CDP, and JS is
    /// evaluated inside the frame to find the element's position. A
    /// cursor-simulating click (pointer + mouse events via CDP `Input`) is then
    /// dispatched at the element's absolute viewport coordinates.
    ///
    /// # Arguments
    /// * `iframe_selector` - CSS selector for the iframe element in the top doc
    /// * `element_selector` - CSS selector for the element inside the iframe
    /// * `timeout_ms` - Max wait for the iframe/element to appear
    ///
    /// # Returns
    /// The click outcome with the absolute viewport point that was clicked.
    pub async fn iframe_click(
        &self,
        iframe_selector: &str,
        element_selector: &str,
        timeout_ms: u64,
    ) -> Result<ClickOutcome> {
        if !self.wait_for(iframe_selector, timeout_ms).await? {
            return Err(anyhow!(
                "Iframe '{iframe_selector}' did not appear within {timeout_ms}ms"
            ));
        }

        // Iframe's absolute rect in the top viewport.
        let iframe_rect = self.get_element_rect(iframe_selector).await?;
        if iframe_rect.width <= 0.0 || iframe_rect.height <= 0.0 {
            return Err(anyhow!("Iframe '{iframe_selector}' has zero size"));
        }

        // The `src` attribute is readable cross-origin and lets us locate the
        // matching frame in the CDP frame tree.
        let src = self
            .attr(iframe_selector, "src")
            .await?
            .ok_or_else(|| anyhow!("Iframe '{iframe_selector}' has no src attribute"))?;

        // Poll inside the frame until the element becomes visible.
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut last_error: Option<anyhow::Error> = None;
        let (local_x, local_y) = loop {
            match self.element_point_in_frame(&src, element_selector).await {
                Ok(Some(point)) => break point,
                Ok(None) => {}
                Err(e) => last_error = Some(e),
            }
            if Instant::now() >= deadline {
                return Err(last_error.unwrap_or_else(|| {
                    anyhow!(
                        "Element '{element_selector}' not found inside iframe within {timeout_ms}ms"
                    )
                }));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        };

        // Absolute viewport point = iframe offset + element's local point.
        let x = iframe_rect.x + local_x;
        let y = iframe_rect.y + local_y;

        // Cursor-simulating click at the absolute point (CDP input, cross-origin safe).
        mouse::left_click_at(self.page(), x, y).await?;

        Ok(ClickOutcome {
            click: ClickStatus::Success,
            x,
            y,
            screen_x: None,
            screen_y: None,
        })
    }

    /// Find an element inside a (possibly cross-origin) iframe and return its
    /// center point in the iframe's local viewport coordinates.
    async fn element_point_in_frame(
        &self,
        iframe_src: &str,
        element_selector: &str,
    ) -> Result<Option<(f64, f64)>> {
        let frame_id = find_frame_by_url(self.page(), iframe_src).await?;
        let Some(ctx) = self.page().frame_execution_context(frame_id).await? else {
            return Ok(None);
        };

        let escaped = element_selector.replace('\\', "\\\\").replace('\'', "\\'");
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{escaped}');
                if (!el) return null;
                const r = el.getBoundingClientRect();
                if (r.width <= 0 || r.height <= 0) return null;
                const s = window.getComputedStyle(el);
                if (s.display === 'none' || s.visibility === 'hidden' || Number.parseFloat(s.opacity || '1') === 0) return null;
                return {{ x: r.x + r.width / 2, y: r.y + r.height / 2 }};
            }})()"#
        );

        let mut params = EvaluateParams::new(js);
        params.context_id = Some(ctx);
        let resp = self.page().execute(params).await?;
        match resp.result.result.value {
            Some(serde_json::Value::Object(map)) => {
                let x = map
                    .get("x")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0);
                let y = map
                    .get("y")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0);
                Ok(Some((x, y)))
            }
            _ => Ok(None),
        }
    }
}

/// Recursively search the CDP frame tree for a frame whose URL matches the
/// given iframe `src` (ignoring the URL fragment).
async fn find_frame_by_url(page: &Page, src: &str) -> Result<FrameId> {
    let base = src.split('#').next().unwrap_or(src);
    let resp = page.execute(GetFrameTreeParams {}).await?;

    fn walk(tree: &FrameTree, base: &str) -> Option<FrameId> {
        if frame_url_matches(&tree.frame.url, base) {
            return Some(tree.frame.id.clone());
        }
        if let Some(children) = &tree.child_frames {
            for child in children {
                if let Some(id) = walk(child, base) {
                    return Some(id);
                }
            }
        }
        None
    }

    walk(&resp.result.frame_tree, base)
        .ok_or_else(|| anyhow!("No frame found for iframe src '{base}'"))
}

/// Match a frame document URL against an iframe `src` base (fragment removed).
fn frame_url_matches(frame_url: &str, src_base: &str) -> bool {
    if frame_url == src_base {
        return true;
    }
    // Fallback: same scheme + host with a path-prefix match (covers redirects
    // and trailing-slash differences).
    let mut f = frame_url.splitn(2, "://");
    let mut s = src_base.splitn(2, "://");
    if let (Some(f_scheme), Some(f_rest), Some(s_scheme), Some(s_rest)) =
        (f.next(), f.next(), s.next(), s.next())
    {
        let f_host_path = f_rest.trim_end_matches('/');
        let s_host_path = s_rest.trim_end_matches('/');
        return f_scheme == s_scheme && f_host_path.starts_with(s_host_path);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::frame_url_matches;

    #[test]
    fn frame_url_exact_match() {
        assert!(frame_url_matches(
            "https://atfminers.asloni.online/miner/index.html?v=1",
            "https://atfminers.asloni.online/miner/index.html?v=1"
        ));
    }

    #[test]
    fn frame_url_prefix_match() {
        assert!(frame_url_matches(
            "https://atfminers.asloni.online/miner/index.html?v=1&x=2",
            "https://atfminers.asloni.online/miner"
        ));
    }

    #[test]
    fn frame_url_scheme_host_prefix_match() {
        assert!(frame_url_matches(
            "https://atfminers.asloni.online/other/page",
            "https://atfminers.asloni.online/other"
        ));
    }

    #[test]
    fn frame_url_different_host_no_match() {
        assert!(!frame_url_matches(
            "https://web.telegram.org/k/",
            "https://atfminers.asloni.online/miner"
        ));
    }

    #[test]
    fn frame_url_different_scheme_no_match() {
        assert!(!frame_url_matches(
            "http://atfminers.asloni.online/miner",
            "https://atfminers.asloni.online/miner"
        ));
    }
}
