//! In-place iframe interaction for `TaskContext`.
//!
//! Cross-origin iframes (e.g. Telegram mini-apps) cannot be reached by CSS
//! selectors from the top document, and their frames may not appear in the
//! page's frame tree (OOPIF). This module resolves the iframe's absolute
//! position from the top document, then attaches a raw CDP session to the
//! iframe's own target via the browser debug WebSocket — JS runs *inside* the
//! cross-origin frame (the same mechanism DevTools uses for cross-origin
//! frames). A cursor-simulating click is dispatched at the element's absolute
//! viewport coordinates. No navigation and no new tabs are involved.

use anyhow::{anyhow, Result};
use chromiumoxide::Page;
use std::time::{Duration, Instant};

use crate::capabilities::mouse;
use crate::runtime::task_context::{Rect, TaskContext};
use crate::utils::{ClickOutcome, ClickStatus};
use serde_json::Value;

impl TaskContext {
    /// Click an element INSIDE an iframe, in place — no navigation, no new tab.
    ///
    /// The iframe (e.g. a cross-origin Telegram mini-app) is resolved from the
    /// top document for its absolute position, then a raw CDP session is
    /// attached to the iframe's own target (OOPIF) so JS runs inside the frame
    /// to locate the element (with `scrollIntoView` for scrollable content).
    /// The absolute point = iframe rect + local point; a cursor-simulating
    /// click (CDP `Input`) is dispatched there — works across origins.
    ///
    /// # Arguments
    /// * `iframe_selector` - CSS selector (or XPath starting with `/`) for the iframe
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
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut last_status = String::new();
        let (x, y) = loop {
            // 1. iframe absolute rect + src (top document; works for any iframe)
            let status = match resolve_iframe(self.page(), iframe_selector).await {
                Ok(Some((rect, src))) if rect.width > 0.0 && rect.height > 0.0 => {
                    // 2. element's local center inside the iframe (OOPIF attach)
                    match self.element_local_center(&src, element_selector).await {
                        Ok(Some((lx, ly))) => {
                            let cx = rect.x + lx;
                            let cy = rect.y + ly;
                            log::info!(
                                "[iframe_click] '{element_selector}' local ({lx:.1},{ly:.1}) + iframe ({:.1},{:.1}) -> ({cx:.1},{cy:.1})",
                                rect.x,
                                rect.y
                            );
                            break (cx, cy);
                        }
                        Ok(None) => "element not found inside iframe yet".to_string(),
                        Err(e) => format!("element resolve error: {e}"),
                    }
                }
                Ok(Some((_rect, _src))) => "iframe found but zero-size".to_string(),
                Ok(None) => "iframe not found yet".to_string(),
                Err(e) => format!("iframe resolve error: {e}"),
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

    /// Find an element inside an iframe (OOPIF or same-origin) and return its
    /// LOCAL center within the iframe's own viewport.
    ///
    /// Attaches a raw CDP session to the iframe's target via the browser debug
    /// WebSocket and evaluates JS inside the frame — the same mechanism DevTools
    /// uses for cross-origin frames. The JS scrolls the first visible match into
    /// view (handles scrollable lists and multiple matching elements).
    async fn element_local_center(
        &self,
        iframe_src: &str,
        element_selector: &str,
    ) -> Result<Option<(f64, f64)>> {
        if self.browser_ws_url.is_empty() {
            log::debug!("[iframe] no browser_ws_url available, cannot attach to iframe target");
            return Ok(None);
        }

        let client =
            crate::runtime::task_context::oopif::OopifClient::connect(&self.browser_ws_url).await?;

        // Host from the iframe src identifies the mini-app target.
        let host = scheme_host(iframe_src)
            .map(|(_, h)| h.to_string())
            .unwrap_or_default();
        if host.is_empty() {
            return Ok(None);
        }
        let target = match client.find_iframe_target(&host).await {
            Ok(t) => t,
            Err(e) => {
                log::debug!("[iframe] find_iframe_target failed: {e}");
                return Ok(None);
            }
        };
        let Some(target_id) = target.get("targetId").and_then(Value::as_str) else {
            return Ok(None);
        };
        let session_id = client.attach(target_id).await?;

        let escaped = element_selector.replace('\\', "\\\\").replace('\'', "\\'");
        let js = format!(
            r#"(() => {{
                const els = document.querySelectorAll('{escaped}');
                for (const el of els) {{
                    el.scrollIntoView({{ block: 'center' }});
                    const r = el.getBoundingClientRect();
                    if (r.width > 0 && r.height > 0) {{
                        return {{ x: r.x + r.width / 2, y: r.y + r.height / 2 }};
                    }}
                }}
                return null;
            }})()"#
        );
        let value = client.evaluate(&session_id, &js).await?;
        match value {
            Value::Object(map) => {
                let lx = map.get("x").and_then(Value::as_f64).unwrap_or(0.0);
                let ly = map.get("y").and_then(Value::as_f64).unwrap_or(0.0);
                Ok(Some((lx, ly)))
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

/// Split a URL into `(scheme, host)`.
fn scheme_host(url: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = url.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    Some((scheme, host))
}

#[cfg(test)]
mod tests {
    use super::scheme_host;

    #[test]
    fn scheme_host_extracts_scheme_and_host() {
        assert_eq!(
            scheme_host("https://atfminers.asloni.online/miner/index.html?v=1#x"),
            Some(("https", "atfminers.asloni.online"))
        );
        assert_eq!(
            scheme_host("https://web.telegram.org/k/"),
            Some(("https", "web.telegram.org"))
        );
        assert_eq!(scheme_host("not a url"), None);
    }
}
