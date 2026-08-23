//! In-place iframe interaction for `TaskContext`.
//!
//! Cross-origin iframes (e.g. Telegram mini-apps) cannot be reached by CSS
//! selectors from the top document, and their frames may not appear in the
//! page's frame tree (OOPIF). This module interacts with them through the CDP
//! **DOM domain**, which pierces iframes at the DevTools level (not subject to
//! web CORS): find the iframe node, pierce into its content document, query the
//! target element, and read its box model in absolute coordinates. A
//! cursor-simulating click is then dispatched at that point. No navigation and
//! no new tabs are involved.

use anyhow::{anyhow, Result};
use chromiumoxide::cdp::browser_protocol::dom::{
    DescribeNodeParams, EnableParams, GetBoxModelParams, GetDocumentParams, QuerySelectorParams,
};
use chromiumoxide::cdp::browser_protocol::page::{FrameId, FrameTree, GetFrameTreeParams};
use chromiumoxide::cdp::js_protocol::runtime::EvaluateParams;
use chromiumoxide::Page;
use std::time::{Duration, Instant};

use crate::capabilities::mouse;
use crate::runtime::task_context::{Rect, TaskContext};
use crate::utils::{ClickOutcome, ClickStatus};
use serde_json::Value;

impl TaskContext {
    /// Click an element INSIDE an iframe, in place — no navigation, no new tab.
    ///
    /// The iframe (e.g. a cross-origin Telegram mini-app) is located with the
    /// CDP DOM domain (`DOM.querySelector` + `DOM.describeNode` with pierce),
    /// the target element is queried inside the iframe's content document, and
    /// its box model gives the absolute viewport point. A cursor-simulating
    /// click (pointer + mouse events via CDP `Input`) is dispatched there.
    ///
    /// # Arguments
    /// * `iframe_selector` - CSS selector for the iframe element in the top doc
    ///   (or an XPath starting with `/`)
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
        // DOM domain enables the commands we need (idempotent).
        let _ = self
            .page()
            .execute(EnableParams {
                include_whitespace: None,
            })
            .await;

        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut last_status = String::new();
        let (x, y) = loop {
            let status = match self
                .element_center_in_iframe(iframe_selector, element_selector)
                .await
            {
                Ok(Some((x, y))) => {
                    log::info!(
                        "[iframe_click] '{element_selector}' inside '{iframe_selector}' at ({x:.1},{y:.1})"
                    );
                    break (x, y);
                }
                Ok(None) => "element not found in iframe yet".to_string(),
                Err(e) => format!("resolve error: {e}"),
            };
            if status != last_status {
                log::info!("[iframe_click] {status}");
                last_status = status;
            }
            if Instant::now() >= deadline {
                return Err(anyhow!(
                    "Element '{element_selector}' inside iframe '{iframe_selector}' not found within {timeout_ms}ms. Last status: {last_status}"
                ));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        };

        log::info!("[iframe_click] clicking at viewport ({x:.1},{y:.1})");
        mouse::left_click_at(self.page(), x, y).await?;

        Ok(ClickOutcome {
            click: ClickStatus::Success,
            x,
            y,
            screen_x: None,
            screen_y: None,
        })
    }

    /// Click at a coordinate INSIDE an iframe, in place — no navigation, no new tab.
    ///
    /// Reliable for cross-origin iframes (e.g. Telegram mini-apps) whose content
    /// is not reachable through the CDP frame tree (OOPIF) or DOM piercing. The
    /// iframe's absolute viewport position is resolved from the top document and
    /// a cursor-simulating click is dispatched at `iframe + (local_x, local_y)`.
    ///
    /// # Arguments
    /// * `iframe_selector` - CSS selector or XPath for the iframe element
    /// * `local_x`, `local_y` - offset within the iframe's own viewport
    /// * `timeout_ms` - max wait for the iframe to become usable
    pub async fn iframe_click_at(
        &self,
        iframe_selector: &str,
        local_x: f64,
        local_y: f64,
        timeout_ms: u64,
    ) -> Result<ClickOutcome> {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let (x, y) = loop {
            match resolve_iframe(self.page(), iframe_selector).await {
                Ok(Some((rect, _))) if rect.width > 0.0 && rect.height > 0.0 => {
                    let cx = rect.x + local_x;
                    let cy = rect.y + local_y;
                    log::info!(
                        "[iframe_click_at] iframe at ({:.1},{:.1}) + local ({local_x:.1},{local_y:.1}) -> viewport ({cx:.1},{cy:.1})",
                        rect.x,
                        rect.y
                    );
                    break (cx, cy);
                }
                Ok(_) => log::debug!("[iframe_click_at] iframe not usable yet"),
                Err(e) => log::debug!("[iframe_click_at] resolve error: {e}"),
            }
            if Instant::now() >= deadline {
                return Err(anyhow!(
                    "Iframe '{iframe_selector}' did not become usable within {timeout_ms}ms"
                ));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        };

        log::info!("[iframe_click_at] clicking at viewport ({x:.1},{y:.1})");
        mouse::left_click_at(self.page(), x, y).await?;

        Ok(ClickOutcome {
            click: ClickStatus::Success,
            x,
            y,
            screen_x: None,
            screen_y: None,
        })
    }

    /// Find an element inside an iframe and return its absolute viewport center.
    ///
    /// Uses the CDP DOM domain which pierces cross-origin iframes. For CSS
    /// iframe selectors this is fully DOM-based. For XPath iframe selectors it
    /// falls back to resolving the frame context and evaluating inside the frame.
    async fn element_center_in_iframe(
        &self,
        iframe_selector: &str,
        element_selector: &str,
    ) -> Result<Option<(f64, f64)>> {
        if iframe_selector.starts_with('/') {
            return self
                .element_center_in_iframe_xpath(iframe_selector, element_selector)
                .await;
        }

        let page = self.page();
        let doc = page.execute(GetDocumentParams::builder().build()).await?;
        let root_id = doc.result.root.node_id;

        // iframe node
        let iframe = page
            .execute(QuerySelectorParams::new(root_id, iframe_selector))
            .await?;
        let iframe_id = iframe.result.node_id;

        // pierce into the iframe to find its content document node
        let desc = page
            .execute(
                DescribeNodeParams::builder()
                    .node_id(iframe_id)
                    .depth(1)
                    .pierce(true)
                    .build(),
            )
            .await?;
        let iframe_node = desc.result.node;
        let doc_id = iframe_node
            .children
            .as_ref()
            .and_then(|cs| cs.iter().find(|c| c.node_name == "#document"))
            .map(|c| c.node_id);
        let Some(doc_id) = doc_id else {
            return Ok(None);
        };

        // element inside the iframe's document
        let el = page
            .execute(QuerySelectorParams::new(doc_id, element_selector))
            .await?;
        let el_id = el.result.node_id;

        // absolute box model → center
        let boxm = page
            .execute(GetBoxModelParams::builder().node_id(el_id).build())
            .await?;
        let quad = boxm.result.model.content.inner();
        if quad.len() >= 8 {
            let cx = (quad[0] + quad[2] + quad[4] + quad[6]) / 4.0;
            let cy = (quad[1] + quad[3] + quad[5] + quad[7]) / 4.0;
            return Ok(Some((cx, cy)));
        }
        Ok(None)
    }

    /// Fallback for XPath iframe selectors: resolve the iframe rect + src via
    /// JS, find the frame context via the frame tree, evaluate inside the frame.
    async fn element_center_in_iframe_xpath(
        &self,
        iframe_selector: &str,
        element_selector: &str,
    ) -> Result<Option<(f64, f64)>> {
        let Some((rect, src)) = resolve_iframe(self.page(), iframe_selector).await? else {
            return Ok(None);
        };
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return Ok(None);
        }
        let frame_id = find_frame_by_url(self.page(), &src).await?;
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
                return {{ x: r.x + r.width / 2, y: r.y + r.height / 2 }};
            }})()"#
        );
        let mut params = EvaluateParams::new(js);
        params.context_id = Some(ctx);
        let resp = self.page().execute(params).await?;
        if let Some(exc) = &resp.result.exception_details {
            log::warn!("[iframe] JS exception: {}", exc.text);
            return Ok(None);
        }
        match resp.result.result.value {
            Some(serde_json::Value::Object(map)) => {
                let lx = map.get("x").and_then(Value::as_f64).unwrap_or(0.0);
                let ly = map.get("y").and_then(Value::as_f64).unwrap_or(0.0);
                Ok(Some((rect.x + lx, rect.y + ly)))
            }
            _ => Ok(None),
        }
    }
}

/// Resolve an iframe element by CSS selector or XPath, returning its viewport
/// rect and `src` attribute.
async fn resolve_iframe(page: &Page, selector: &str) -> Result<Option<(Rect, String)>> {
    let escaped = selector.replace('\\', "\\\\").replace('\'', "\\'");
    let js = format!(
        r#"(() => {{
            try {{
                const q = '{escaped}';
                let el = null;
                if (q.startsWith('/')) {{
                    const xr = document.evaluate(q, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                    el = xr.singleNodeValue;
                }} else {{
                    el = document.querySelector(q);
                }}
                if (!el || el.tagName !== 'IFRAME') return null;
                const r = el.getBoundingClientRect();
                return {{
                    x: r.x, y: r.y, width: r.width, height: r.height,
                    src: el.getAttribute('src') || ''
                }};
            }} catch (e) {{
                return {{ error: String(e && e.message ? e.message : e) }};
            }}
        }})()"#
    );
    let resp = page.evaluate(js).await?;
    match resp.value() {
        Some(serde_json::Value::Object(map)) => {
            if let Some(err) = map.get("error").and_then(Value::as_str) {
                log::warn!("[iframe] resolve_iframe JS error: {err}");
                return Err(anyhow!("iframe selector JS error: {err}"));
            }
            let x = map.get("x").and_then(Value::as_f64).unwrap_or(0.0);
            let y = map.get("y").and_then(Value::as_f64).unwrap_or(0.0);
            let w = map.get("width").and_then(Value::as_f64).unwrap_or(0.0);
            let h = map.get("height").and_then(Value::as_f64).unwrap_or(0.0);
            let src = map
                .get("src")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            Ok(Some((
                Rect {
                    x,
                    y,
                    width: w,
                    height: h,
                },
                src,
            )))
        }
        _ => Ok(None),
    }
}

/// Resolve the CDP frame for an iframe by matching its `src` (fallback path).
async fn find_frame_by_url(page: &Page, src: &str) -> Result<FrameId> {
    let base = src.split('#').next().unwrap_or(src);
    let resp = page.execute(GetFrameTreeParams {}).await?;

    let mut frames: Vec<(FrameId, String)> = Vec::new();
    fn collect(tree: &FrameTree, out: &mut Vec<(FrameId, String)>) {
        out.push((tree.frame.id.clone(), tree.frame.url.clone()));
        if let Some(children) = &tree.child_frames {
            for child in children {
                collect(child, out);
            }
        }
    }
    collect(&resp.result.frame_tree, &mut frames);

    for (id, url) in &frames {
        if frame_url_matches(url, base) {
            return Ok(id.clone());
        }
    }
    let urls: Vec<&str> = frames.iter().map(|(_, u)| u.as_str()).collect();
    Err(anyhow!(
        "No frame found for iframe src '{base}'. Frames in tree: [{}]",
        urls.join(", ")
    ))
}

/// Match a frame document URL against an iframe `src` base (fragment removed).
fn frame_url_matches(frame_url: &str, src_base: &str) -> bool {
    if frame_url == src_base {
        return true;
    }
    if let (Some((f_scheme, f_host)), Some((s_scheme, s_host))) =
        (scheme_host(frame_url), scheme_host(src_base))
    {
        if f_scheme == s_scheme && f_host == s_host {
            return true;
        }
    }
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
    fn frame_url_spa_navigation_same_host_matches() {
        assert!(frame_url_matches(
            "https://atfminers.asloni.online/miner/tasks/view",
            "https://atfminers.asloni.online/miner/index.html?v=1786287551"
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
