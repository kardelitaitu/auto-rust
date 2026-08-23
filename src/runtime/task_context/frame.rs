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
        log::debug!(
            "[iframe_click] waiting for iframe '{iframe_selector}' (timeout {timeout_ms}ms)"
        );
        if !self.wait_for(iframe_selector, timeout_ms).await? {
            return Err(anyhow!(
                "Iframe '{iframe_selector}' did not appear within {timeout_ms}ms"
            ));
        }

        // Iframe's absolute rect in the top viewport.
        let iframe_rect = self.get_element_rect(iframe_selector).await?;
        log::debug!(
            "[iframe_click] iframe rect: x={:.1} y={:.1} w={:.1} h={:.1}",
            iframe_rect.x,
            iframe_rect.y,
            iframe_rect.width,
            iframe_rect.height
        );
        if iframe_rect.width <= 0.0 || iframe_rect.height <= 0.0 {
            return Err(anyhow!("Iframe '{iframe_selector}' has zero size"));
        }

        // The `src` attribute is readable cross-origin and lets us locate the
        // matching frame in the CDP frame tree.
        let src = self
            .attr(iframe_selector, "src")
            .await?
            .ok_or_else(|| anyhow!("Iframe '{iframe_selector}' has no src attribute"))?;
        log::info!("[iframe_click] iframe src: {}", &src[..src.len().min(160)]);

        // Poll inside the frame until the element becomes visible.
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut last_error: Option<anyhow::Error> = None;
        let mut attempt = 0u32;
        let (local_x, local_y) = loop {
            attempt += 1;
            match self.element_point_in_frame(&src, element_selector).await {
                Ok(Some(point)) => {
                    log::debug!(
                        "[iframe_click] attempt {attempt}: element found at local ({:.1},{:.1})",
                        point.0,
                        point.1
                    );
                    break point;
                }
                Ok(None) => log::debug!("[iframe_click] attempt {attempt}: element not usable yet"),
                Err(e) => {
                    log::debug!("[iframe_click] attempt {attempt}: error: {e}");
                    last_error = Some(e);
                }
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
        log::info!("[iframe_click] clicking at viewport ({x:.1},{y:.1})");

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
    ///
    /// The evaluation reports a status so callers can distinguish "missing",
    /// "zero-size", "hidden", or a genuine JS exception from success.
    async fn element_point_in_frame(
        &self,
        iframe_src: &str,
        element_selector: &str,
    ) -> Result<Option<(f64, f64)>> {
        let frame_id = match find_frame_by_url(self.page(), iframe_src).await {
            Ok(id) => {
                log::info!("[iframe] matched frame id: {id:?}");
                id
            }
            Err(e) => {
                log::warn!("[iframe] frame lookup failed: {e}");
                return Err(e);
            }
        };
        let Some(ctx) = self
            .page()
            .frame_execution_context(frame_id.clone())
            .await?
        else {
            log::warn!("[iframe] frame {frame_id:?} has no execution context (is it loaded?)");
            return Ok(None);
        };
        log::info!("[iframe] execution context: {ctx:?}");

        let escaped = element_selector.replace('\\', "\\\\").replace('\'', "\\'");
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{escaped}');
                if (!el) return {{ status: 'missing' }};
                const r = el.getBoundingClientRect();
                if (r.width <= 0 || r.height <= 0) return {{ status: 'zero_size', found: true }};
                const s = window.getComputedStyle(el);
                if (s.display === 'none' || s.visibility === 'hidden' || Number.parseFloat(s.opacity || '1') === 0) return {{ status: 'hidden', found: true }};
                return {{ status: 'ok', x: r.x + r.width / 2, y: r.y + r.height / 2 }};
            }})()"#
        );

        let mut params = EvaluateParams::new(js);
        params.context_id = Some(ctx);
        let resp = self.page().execute(params).await?;

        if let Some(exc) = &resp.result.exception_details {
            log::warn!(
                "[iframe] JS exception while querying '{element_selector}': {}",
                exc.text
            );
            return Ok(None);
        }

        match resp.result.result.value {
            Some(serde_json::Value::Object(map)) => {
                let status = map
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown");
                match status {
                    "ok" => {
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
                    other => {
                        log::info!("[iframe] element '{element_selector}' status: {other}");
                        Ok(None)
                    }
                }
            }
            _ => {
                log::debug!("[iframe] unexpected evaluate result for '{element_selector}'");
                Ok(None)
            }
        }
    }
}

/// Resolve the CDP frame that corresponds to an iframe element.
///
/// Strategy (in order):
/// 1. Exact URL match against the iframe's `src` (fragment removed)
/// 2. Same scheme + host (handles SPA navigation inside the frame)
/// 3. If there is exactly one http(s) child frame, assume it is the target
///    (a mini-app iframe is usually the only embedded frame on the page)
///
/// The error message always lists every frame URL seen, so the failure is
/// self-diagnosing regardless of the log level.
async fn find_frame_by_url(page: &Page, src: &str) -> Result<FrameId> {
    let base = src.split('#').next().unwrap_or(src);
    let resp = page.execute(GetFrameTreeParams {}).await?;

    // Flatten the tree once, logging each frame for diagnostics.
    let mut frames: Vec<(FrameId, String)> = Vec::new();
    fn collect(tree: &FrameTree, out: &mut Vec<(FrameId, String)>, depth: usize) {
        let indent = "  ".repeat(depth);
        log::info!(
            "[iframe] {indent}frame id={:?} url={}",
            tree.frame.id,
            tree.frame.url
        );
        out.push((tree.frame.id.clone(), tree.frame.url.clone()));
        if let Some(children) = &tree.child_frames {
            for child in children {
                collect(child, out, depth + 1);
            }
        }
    }
    collect(&resp.result.frame_tree, &mut frames, 0);

    // 1. exact, 2. host match (skip the main frame itself for strategy 2+)
    for (id, url) in &frames {
        if frame_url_matches(url, base) {
            log::info!("[iframe] matched frame {id:?} by URL");
            return Ok(id.clone());
        }
    }

    // 3. exactly one http(s) child frame → assume it is the iframe.
    let http_frames: Vec<&(FrameId, String)> = frames
        .iter()
        .filter(|(_, url)| url.starts_with("http://") || url.starts_with("https://"))
        .collect();
    if http_frames.len() == 1 {
        let (id, url) = http_frames[0];
        log::info!("[iframe] matched only http(s) frame {id:?} by url={url}");
        return Ok(id.clone());
    }

    let urls: Vec<&str> = frames.iter().map(|(_, u)| u.as_str()).collect();
    Err(anyhow!(
        "No frame found for iframe src '{base}'. Frames in tree: [{}]",
        urls.join(", ")
    ))
}

/// Match a frame document URL against an iframe `src` base (fragment removed).
///
/// Matching order: exact URL → same scheme+host (SPA navigation inside the
/// frame changes the path/query, so host equality is the reliable signal) →
/// path-prefix fallback.
fn frame_url_matches(frame_url: &str, src_base: &str) -> bool {
    if frame_url == src_base {
        return true;
    }

    if let (Some((f_scheme, f_host)), Some((s_scheme, s_host))) =
        (scheme_host(frame_url), scheme_host(src_base))
    {
        // Same scheme + host: the frame is the target even if it navigated
        // to a different route inside the app.
        if f_scheme == s_scheme && f_host == s_host {
            return true;
        }
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
        if f_scheme == s_scheme && f_host_path.starts_with(s_host_path) {
            return true;
        }
    }
    false
}

/// Split a URL into `(scheme, host)`.
fn scheme_host(url: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = url.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    Some((scheme, host))
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

    #[test]
    fn frame_url_spa_navigation_same_host_matches() {
        // The mini-app navigated to a different route (SPA) — same host must
        // still resolve to the iframe.
        assert!(frame_url_matches(
            "https://atfminers.asloni.online/miner/tasks/view",
            "https://atfminers.asloni.online/miner/index.html?v=1786287551"
        ));
    }
}
