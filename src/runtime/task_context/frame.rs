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
        self.iframe_click_inner(iframe_selector, element_selector, None, None, timeout_ms)
            .await
    }

    /// Like [`Self::iframe_click`], but skips the element whose local center is
    /// within 5px of `(skip_x, skip_y)`. Useful for clicking each item in a
    /// scrolling list one at a time (the previously clicked item stays visible).
    pub async fn iframe_click_skip(
        &self,
        iframe_selector: &str,
        element_selector: &str,
        skip_x: f64,
        skip_y: f64,
        timeout_ms: u64,
    ) -> Result<ClickOutcome> {
        self.iframe_click_inner(
            iframe_selector,
            element_selector,
            Some((skip_x, skip_y)),
            None,
            timeout_ms,
        )
        .await
    }

    /// Like [`Self::iframe_click`], but also requires the element's exact trimmed
    /// text content to match `text_filter`. Useful when a CSS selector alone may
    /// match multiple elements (e.g. `.btn-small#btn-youtube_like_comment` +
    /// text "Go").
    pub async fn iframe_click_text(
        &self,
        iframe_selector: &str,
        element_selector: &str,
        text_filter: &str,
        timeout_ms: u64,
    ) -> Result<ClickOutcome> {
        self.iframe_click_inner(
            iframe_selector,
            element_selector,
            None,
            Some(text_filter),
            timeout_ms,
        )
        .await
    }

    /// Quick single-shot probe: check whether an element matching `element_selector`
    /// with text `text_filter` exists inside the iframe. Returns immediately with
    /// `true`/`false` (no poll loop). Useful as a fast "skip" check before
    /// committing to a full `iframe_click_text` (which would otherwise poll for
    /// the whole timeout just to discover the element is gone).
    pub async fn iframe_has_text(
        &self,
        iframe_selector: &str,
        element_selector: &str,
        text_filter: &str,
    ) -> Result<bool> {
        // The mini-app can re-create its iframe (blank doc) after a tab click,
        // so a single shot can miss. Retry briefly (up to ~4s) while the
        // document looks blank; a definite answer (element found, or a populated
        // doc without the element) returns immediately.
        for _ in 0..8 {
            match self
                .iframe_has_text_once(iframe_selector, element_selector, text_filter)
                .await?
            {
                Some(answer) => return Ok(answer),
                None => tokio::time::sleep(Duration::from_millis(500)).await,
            }
        }
        Ok(false)
    }

    /// One-shot probe. Returns:
    /// - `Some(true)` — element found with matching text (and not disabled)
    /// - `Some(false)` — definite miss (populated doc without the element, or
    ///   element found with different text / disabled)
    /// - `None` — ambiguous (iframe/session not ready, or document is blank) —
    ///   caller should retry.
    async fn iframe_has_text_once(
        &self,
        iframe_selector: &str,
        element_selector: &str,
        text_filter: &str,
    ) -> Result<Option<bool>> {
        if self.browser_ws_url.is_empty() {
            return Ok(Some(false));
        }
        let client =
            match crate::runtime::task_context::oopif::OopifClient::connect(&self.browser_ws_url)
                .await
            {
                Ok(c) => Some(c),
                Err(e) => {
                    log::warn!("[iframe_has_text] connect failed: {e}");
                    return Ok(Some(false));
                }
            };
        let Some((rect, src)) = resolve_iframe(self.page(), iframe_selector).await? else {
            return Ok(None);
        };
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return Ok(None);
        }
        // Diagnose which iframe element we resolved: count all matches on the
        // main page and their srcs — Telegram may keep several mini-app iframes
        // in the DOM, and querySelector picks the first (possibly a stale one).
        {
            let sel = escape_js_string(iframe_selector);
            let js = format!(
                r#"(() => {{
                    const els = document.querySelectorAll('{sel}');
                    return JSON.stringify([...els].map(e => ({{
                        src: (e.getAttribute('src') || '').slice(0, 60),
                        w: e.getBoundingClientRect().width
                    }})));
                }})()"#
            );
            if let Ok(resp) = self.page().evaluate(js).await {
                if let Some(list) = resp.value().and_then(serde_json::Value::as_str) {
                    log::info!(
                        "[iframe_has_text] resolved '{iframe_selector}' -> {list} (using src='{src}')"
                    );
                }
            }
        }
        let mut session_cache: Option<(String, String)> = None;
        let Some(session_id) = self
            .iframe_session_id(client.as_ref(), &src, &mut session_cache)
            .await?
        else {
            return Ok(None);
        };
        let Some(client) = client.as_ref() else {
            return Ok(Some(false));
        };
        let escaped = escape_js_string(element_selector);
        let js = format!(
            r#"(() => {{
                const el = document.querySelector('{escaped}');
                const btnCount = document.querySelectorAll('.btn-small').length;
                const sample = [...document.querySelectorAll('[id*="btn-"]')]
                    .slice(0, 8).map(b => b.id).join(',');
                const tabCount = document.querySelectorAll('[onclick^="switchTab("]').length;
                const body = (document.body ? document.body.innerText : '').replace(/\\s+/g, ' ').trim().slice(0, 200);
                const docUrl = location.href.slice(0, 80);
                const ready = document.readyState;
                // Busy detection: a visible modal/overlay, or processing text
                // anywhere in the document. The mini-app shows "Processing"
                // in an overlay while the task button's text stays "Go".
                const busyEl = [...document.querySelectorAll('[class*="modal"],[id*="modal"],[class*="overlay"],[id*="overlay"]')]
                    .find(n => {{
                        const r = n.getBoundingClientRect();
                        return r.width > 0 && r.height > 0;
                    }});
                const busy = !!busyEl || /Processing|Verifying|Connecting|Please wait|Claiming|In progress/i.test(body);
                if (!el) return JSON.stringify({{ found: false, text: null, disabled: false, btnCount, sample, tabCount, body, docUrl, ready, busy }});
                return JSON.stringify({{
                    found: true,
                    text: (el.textContent || '').trim(),
                    disabled: !!el.disabled,
                    btnCount,
                    sample,
                    tabCount,
                    body,
                    docUrl,
                    ready,
                    busy
                }});
            }})()"#
        );
        let value = client.evaluate(&session_id, &js).await?;
        // Probe returns found + text + disabled + btnCount + button-id sample +
        // tab count + on-screen text + the attached document's URL/readyState so
        // we can see what view (or which iframe) the probe is actually hitting.
        // The JS returns JSON.stringify(...) — parse the string back into an object.
        let value = match value {
            Value::String(s) => serde_json::from_str::<Value>(&s).unwrap_or(Value::Null),
            other => other,
        };
        let text = value.get("text").and_then(Value::as_str).unwrap_or("");
        let found = value.get("found").and_then(Value::as_bool).unwrap_or(false);
        let disabled = value
            .get("disabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let busy = value.get("busy").and_then(Value::as_bool).unwrap_or(false);
        let btn_count = value.get("btnCount").and_then(Value::as_u64).unwrap_or(0);
        let sample = value.get("sample").and_then(Value::as_str).unwrap_or("");
        let tab_count = value.get("tabCount").and_then(Value::as_u64).unwrap_or(0);
        let body = value.get("body").and_then(Value::as_str).unwrap_or("");
        let doc_url = value.get("docUrl").and_then(Value::as_str).unwrap_or("");
        let ready = value.get("ready").and_then(Value::as_str).unwrap_or("");
        log::info!(
            "[iframe_has_text] '{element_selector}' found={found} disabled={disabled} busy={busy} btnCount={btn_count} tabCount={tab_count} sample=[{sample}] docUrl='{doc_url}' ready={ready} body='{body}' text='{text}' (filter='{text_filter}')"
        );
        if found {
            // Definite: element present. Stop when it's disabled, its text no
            // longer matches, or the mini-app shows a busy state (modal /
            // "Processing") — the button text may stay "Go" while a task runs.
            return Ok(Some(found && !disabled && !busy && text == text_filter));
        }
        // Element absent. If the document has real content, this is a definite
        // miss; a blank document means the iframe may still be loading — retry.
        if body.is_empty() && btn_count == 0 && tab_count == 0 {
            Ok(None)
        } else {
            Ok(Some(false))
        }
    }

    /// Shared implementation for [`Self::iframe_click`] / [`Self::iframe_click_skip`]
    /// / [`Self::iframe_click_text`].
    async fn iframe_click_inner(
        &self,
        iframe_selector: &str,
        element_selector: &str,
        skip: Option<(f64, f64)>,
        text_filter: Option<&str>,
        timeout_ms: u64,
    ) -> Result<ClickOutcome> {
        // Open ONE raw CDP session to the browser (reused across the poll loop).
        // Fail fast when the URL is configured but the browser is unreachable —
        // don't silently degrade into a misleading "element not found".
        let client = if self.browser_ws_url.is_empty() {
            None
        } else {
            Some(
                crate::runtime::task_context::oopif::OopifClient::connect(&self.browser_ws_url)
                    .await
                    .map_err(|e| {
                        anyhow!(
                            "OOPIF client connect to '{}' failed: {e}",
                            self.browser_ws_url
                        )
                    })?,
            )
        };

        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut last_status = String::new();
        // Cache the attached (target_id, session_id); re-attach only when the
        // iframe target changes (e.g. the frame reloaded).
        let mut session_cache: Option<(String, String)> = None;
        let (x, y) = loop {
            // 1. iframe absolute rect + src (top document; works for any iframe)
            let status = match resolve_iframe(self.page(), iframe_selector).await {
                Ok(Some((rect, src))) if rect.width > 0.0 && rect.height > 0.0 => {
                    // 2. element's local center inside the iframe (OOPIF attach)
                    match self
                        .element_local_center(
                            client.as_ref(),
                            &src,
                            element_selector,
                            skip,
                            &mut session_cache,
                            text_filter,
                        )
                        .await
                    {
                        Ok(Some((lx, ly))) => {
                            // Re-read the iframe rect: `scrollIntoView` inside the
                            // frame can scroll the parent page, moving the iframe.
                            match resolve_iframe(self.page(), iframe_selector).await {
                                Ok(Some((rect2, _))) if rect2.width > 0.0 && rect2.height > 0.0 => {
                                    let cx = rect2.x + lx;
                                    let cy = rect2.y + ly;
                                    // The local center must be INSIDE the iframe —
                                    // otherwise the click lands on the page below
                                    // the mini-app and nothing happens.
                                    if lx < 0.0 || ly < 0.0 || lx > rect2.width || ly > rect2.height
                                    {
                                        "element outside iframe viewport (not scrolled into view)"
                                            .to_string()
                                    } else {
                                        log::info!(
                                            "[iframe_click] '{element_selector}' local ({lx:.1},{ly:.1}) + iframe ({:.1},{:.1}) -> ({cx:.1},{cy:.1})",
                                            rect2.x,
                                            rect2.y
                                        );
                                        break (cx, cy);
                                    }
                                }
                                _ => "iframe moved or became unusable after element resolve"
                                    .to_string(),
                            }
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

    /// Count elements matching `element_selector` inside the iframe (OOPIF-safe).
    ///
    /// Returns the number of visible (non-zero-size) matches. Useful for knowing
    /// how many items exist (e.g. "Go" buttons) instead of polling until timeout.
    ///
    /// # Arguments
    /// * `iframe_selector` - CSS selector (or XPath starting with `/`) for the iframe
    /// * `element_selector` - CSS selector for the elements inside the iframe
    /// * `timeout_ms` - Max wait for the iframe to become usable
    pub async fn iframe_count(
        &self,
        iframe_selector: &str,
        element_selector: &str,
        timeout_ms: u64,
    ) -> Result<usize> {
        let client = if self.browser_ws_url.is_empty() {
            None
        } else {
            Some(
                crate::runtime::task_context::oopif::OopifClient::connect(&self.browser_ws_url)
                    .await
                    .map_err(|e| {
                        anyhow!(
                            "OOPIF client connect to '{}' failed: {e}",
                            self.browser_ws_url
                        )
                    })?,
            )
        };

        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut last_status = String::new();
        let mut session_cache: Option<(String, String)> = None;
        loop {
            let status = match resolve_iframe(self.page(), iframe_selector).await {
                Ok(Some((rect, src))) if rect.width > 0.0 && rect.height > 0.0 => {
                    match self
                        .element_count_in_iframe(
                            client.as_ref(),
                            &src,
                            element_selector,
                            &mut session_cache,
                        )
                        .await
                    {
                        Ok(Some(n)) => {
                            log::info!(
                                "[iframe_count] '{element_selector}' inside '{iframe_selector}': {n} visible"
                            );
                            return Ok(n);
                        }
                        Ok(None) => "iframe target not ready yet".to_string(),
                        Err(e) => format!("count resolve error: {e}"),
                    }
                }
                Ok(Some((_rect, _src))) => "iframe found but zero-size".to_string(),
                Ok(None) => "iframe not found yet".to_string(),
                Err(e) => format!("iframe resolve error: {e}"),
            };
            if status != last_status {
                log::info!("[iframe_count] {status}");
                last_status = status;
            }
            if Instant::now() >= deadline {
                return Err(anyhow!(
                    "Iframe '{iframe_selector}' did not become usable within {timeout_ms}ms. Last status: {last_status}"
                ));
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
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
    /// uses for cross-origin frames. The JS:
    /// - iterates all matching elements, scrolling each into view
    /// - skips elements whose local center is within 5px of `skip` (if set)
    /// - verifies the element is the topmost at the click point (`elementFromPoint`)
    ///   so a covered element is not clicked
    /// - returns the center of the first passing element
    async fn element_local_center(
        &self,
        client: Option<&crate::runtime::task_context::oopif::OopifClient>,
        iframe_src: &str,
        element_selector: &str,
        skip: Option<(f64, f64)>,
        cached: &mut Option<(String, String)>,
        text_filter: Option<&str>,
    ) -> Result<Option<(f64, f64)>> {
        let Some(client) = client else {
            return Ok(None);
        };
        let Some(session_id) = self
            .iframe_session_id(Some(client), iframe_src, cached)
            .await?
        else {
            return Ok(None);
        };
        let js = build_element_js(element_selector, skip, text_filter);
        let value = client.evaluate(&session_id, &js).await?;
        Ok(parse_local_center(&value))
    }

    /// Count visible (non-zero-size) matching elements inside the iframe, using
    /// the same OOPIF session machinery. Returns `Ok(Some(n))` when the iframe
    /// target is reachable, `Ok(None)` when not yet ready.
    async fn element_count_in_iframe(
        &self,
        client: Option<&crate::runtime::task_context::oopif::OopifClient>,
        iframe_src: &str,
        element_selector: &str,
        cached: &mut Option<(String, String)>,
    ) -> Result<Option<usize>> {
        let Some(client) = client else {
            return Ok(None);
        };
        let Some(session_id) = self
            .iframe_session_id(Some(client), iframe_src, cached)
            .await?
        else {
            return Ok(None);
        };
        let js = build_count_js(element_selector);
        let value = client.evaluate(&session_id, &js).await?;
        Ok(value.as_u64().map(|n| n as usize))
    }

    /// Resolve the iframe target via host and return an attached session id.
    /// Reuses the cached `(target_id, session_id)` while the target is unchanged.
    /// Returns `Ok(None)` when the target is not yet found (pollable).
    async fn iframe_session_id(
        &self,
        client: Option<&crate::runtime::task_context::oopif::OopifClient>,
        iframe_src: &str,
        cached: &mut Option<(String, String)>,
    ) -> Result<Option<String>> {
        let Some(client) = client else {
            return Ok(None);
        };
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
        let session_id = match cached {
            Some((tid, sid)) if tid == target_id => sid.clone(),
            _ => {
                let sid = client.attach(target_id).await?;
                *cached = Some((target_id.to_string(), sid.clone()));
                sid
            }
        };
        Ok(Some(session_id))
    }
}

/// Parse the evaluated JS result into a local center point.
///
/// Strict: both `x` and `y` must be present, numeric, and finite. A partial or
/// malformed object (e.g. `{x: 5}` missing `y`) returns `None` rather than a
/// half-known point that would be clicked at the wrong location.
fn parse_local_center(value: &serde_json::Value) -> Option<(f64, f64)> {
    let x = value.get("x").and_then(serde_json::Value::as_f64)?;
    let y = value.get("y").and_then(serde_json::Value::as_f64)?;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some((x, y))
}

/// Resolve an iframe element by CSS selector or XPath, returning its viewport
/// rect and `src` attribute.
async fn resolve_iframe(page: &Page, selector: &str) -> Result<Option<(Rect, String)>> {
    let js = build_resolve_iframe_js(selector);
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

/// Build the JS that counts visible (non-zero-size) matches of `element_selector`
/// inside the iframe. Used to know how many items exist (e.g. "Go" buttons)
/// without polling until timeout.
fn build_count_js(element_selector: &str) -> String {
    let escaped = escape_js_string(element_selector);
    format!(
        r#"(() => {{
            let count = 0;
            const els = document.querySelectorAll('{escaped}');
            for (const el of els) {{
                const r = el.getBoundingClientRect();
                if (r.width > 0 && r.height > 0) count++;
            }}
            return count;
        }})()"#
    )
}

/// Split a URL into `(scheme, host)`.
fn scheme_host(url: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = url.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    Some((scheme, host))
}

/// Escape a selector for embedding in a single-quoted JS string.
fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

/// Build the JS that resolves the mini-app element's local center inside the
/// iframe: iterate matches, scroll each into view, skip the point within 5px of
/// `skip`, optionally require an exact trimmed text content (`text_filter`),
/// and hit-verify with `elementFromPoint` before returning the center.
fn build_element_js(
    element_selector: &str,
    skip: Option<(f64, f64)>,
    text_filter: Option<&str>,
) -> String {
    let escaped = escape_js_string(element_selector);
    // A non-finite skip coordinate is meaningless — `Math.abs(cx - NaN) < 5` is
    // always false, silently disabling the skip. Treat it as "no skip".
    let finite_skip = skip.filter(|(x, y)| x.is_finite() && y.is_finite());
    let skip_js = match finite_skip {
        Some((sx, sy)) => format!("const skipX = {sx}, skipY = {sy};"),
        None => "const skipX = null, skipY = null;".to_string(),
    };
    let text_js = match text_filter {
        Some(text) => format!("const textFilter = '{}';", escape_js_string(text)),
        None => "const textFilter = null;".to_string(),
    };
    format!(
        r#"(() => {{
            {skip_js}
            {text_js}
            function scrollIntoFullView(el) {{
                el.scrollIntoView({{ block: 'center', inline: 'center' }});
                // Some mini-apps use nested scroll containers that scrollIntoView
                // cannot reach — scroll each scrollable ancestor manually so the
                // element's center lands inside the iframe viewport.
                let node = el.parentElement;
                while (node) {{
                    const cs = getComputedStyle(node);
                    const overflowY = cs.overflowY;
                    const scrollable = overflowY === 'auto' || overflowY === 'scroll' || overflowY === 'overlay';
                    if (scrollable && node.scrollHeight > node.clientHeight) {{
                        const er = el.getBoundingClientRect();
                        const nr = node.getBoundingClientRect();
                        const target = node.scrollTop + (er.top - nr.top) - node.clientHeight / 2 + er.height / 2;
                        node.scrollTop = Math.max(0, Math.min(target, node.scrollHeight - node.clientHeight));
                    }}
                    node = node.parentElement;
                }}
            }}
            function inViewport(r) {{
                return r.width > 0 && r.height > 0 &&
                    r.x >= 0 && r.y >= 0 &&
                    r.x + r.width <= window.innerWidth &&
                    r.y + r.height <= window.innerHeight;
            }}
            const els = document.querySelectorAll('{escaped}');
            for (const el of els) {{
                if (textFilter !== null && (el.textContent || '').trim() !== textFilter) continue;
                scrollIntoFullView(el);
                const r = el.getBoundingClientRect();
                if (!inViewport(r)) continue;
                const cx = r.x + r.width / 2;
                const cy = r.y + r.height / 2;
                if (skipX !== null && Math.abs(cx - skipX) < 5 && Math.abs(cy - skipY) < 5) continue;
                const hit = document.elementFromPoint(cx, cy);
                if (hit && (el === hit || el.contains(hit) || hit.contains(el))) {{
                    return {{ x: cx, y: cy }};
                }}
            }}
            return null;
        }})()"#
    )
}

/// Build the JS that resolves the iframe element (CSS or XPath) into its
/// viewport rect and `src` attribute.
fn build_resolve_iframe_js(selector: &str) -> String {
    let escaped = escape_js_string(selector);
    format!(
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
    )
}

#[cfg(test)]
mod tests {
    use super::{
        build_count_js, build_element_js, build_resolve_iframe_js, escape_js_string,
        parse_local_center, scheme_host,
    };

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

    #[test]
    fn escape_js_string_escapes_quotes_and_backslashes() {
        assert_eq!(escape_js_string("plain"), "plain");
        assert_eq!(escape_js_string("it's"), "it\\'s");
        assert_eq!(escape_js_string("a\\b"), "a\\\\b");
    }

    #[test]
    fn build_element_js_embeds_selector_and_skip() {
        let js = build_element_js("[id^=\"btn-x\"]", Some((10.5, 20.0)), None);
        assert!(js.contains("querySelectorAll('[id^=\"btn-x\"]')"));
        assert!(js.contains("const skipX = 10.5, skipY = 20;"));
        assert!(js.contains("elementFromPoint"));
        assert!(js.contains("scrollIntoView"));
    }

    #[test]
    fn build_element_js_no_skip_sets_null() {
        let js = build_element_js(".btn", None, None);
        assert!(js.contains("const skipX = null, skipY = null;"));
        assert!(!js.contains("skipX = 0"));
    }

    #[test]
    fn build_element_js_non_finite_skip_disables_skip() {
        // A NaN/Infinity skip coordinate is meaningless — `Math.abs(cx - NaN) < 5`
        // is always false, so the skip would silently never trigger. Spec: treat a
        // non-finite skip as "no skip" (skipX = null).
        let nan_js = build_element_js(".btn", Some((f64::NAN, 5.0)), None);
        assert!(nan_js.contains("const skipX = null, skipY = null;"));
        assert!(!nan_js.contains("skipX = NaN"));

        let inf_js = build_element_js(".btn", Some((f64::INFINITY, 5.0)), None);
        assert!(inf_js.contains("const skipX = null, skipY = null;"));
        assert!(!inf_js.contains("skipX = Infinity"));

        let neg_inf_js = build_element_js(".btn", Some((5.0, f64::NEG_INFINITY)), None);
        assert!(neg_inf_js.contains("const skipX = null, skipY = null;"));
    }

    #[test]
    fn build_element_js_escapes_selector_quotes() {
        // Selector with a single quote must be escaped inside the JS string.
        let js = build_element_js("button[onclick=\"f('x')\"]", None, None);
        assert!(js.contains("f(\\'x\\')"));
    }

    #[test]
    fn build_element_js_requires_viewport_containment() {
        // The element must be fully inside the iframe viewport, otherwise the
        // click would land outside the iframe (below the mini-app).
        let js = build_element_js(".btn", None, None);
        assert!(js.contains("function inViewport"));
        assert!(js.contains("r.x + r.width <= window.innerWidth"));
        assert!(js.contains("r.y + r.height <= window.innerHeight"));
        assert!(js.contains("if (!inViewport(r)) continue;"));
    }

    #[test]
    fn build_element_js_scrolls_nested_containers() {
        // Some mini-apps use nested scroll containers that plain scrollIntoView
        // cannot reach — the JS must walk scrollable ancestors manually.
        let js = build_element_js(".btn", None, None);
        assert!(js.contains("function scrollIntoFullView"));
        assert!(js.contains("overflowY"));
        assert!(js.contains("node.scrollHeight > node.clientHeight"));
        assert!(js.contains("scrollIntoView({ block: 'center', inline: 'center' })"));
    }

    #[test]
    fn build_resolve_iframe_js_handles_css_and_xpath() {
        let css_js = build_resolve_iframe_js("iframe.payment-verification");
        assert!(css_js.contains("const q = 'iframe.payment-verification';"));
        assert!(css_js.contains("document.querySelector(q)"));
        assert!(css_js.contains("tagName !== 'IFRAME'"));

        let xpath_js = build_resolve_iframe_js("/html/body/div[10]/div/div[2]/div/div/iframe");
        assert!(xpath_js.contains("q.startsWith('/')"));
        assert!(xpath_js.contains("XPathResult.FIRST_ORDERED_NODE_TYPE"));
        assert!(xpath_js.contains("error: String"));
    }

    #[test]
    fn build_resolve_iframe_js_escapes_xpath_quotes() {
        let js = build_resolve_iframe_js("//div[@id='main']/iframe");
        assert!(js.contains("\\'main\\'"));
    }

    #[test]
    fn parse_local_center_valid_object() {
        assert_eq!(
            parse_local_center(&serde_json::json!({ "x": 10.5, "y": 20.0 })),
            Some((10.5, 20.0))
        );
    }

    #[test]
    fn parse_local_center_requires_both_coordinates() {
        // Missing y must NOT silently become (x, 0.0) — a half-known point is unusable.
        assert_eq!(parse_local_center(&serde_json::json!({ "x": 10.5 })), None);
        assert_eq!(parse_local_center(&serde_json::json!({ "y": 20.0 })), None);
        assert_eq!(parse_local_center(&serde_json::json!({})), None);
    }

    #[test]
    fn parse_local_center_rejects_non_number_or_non_finite() {
        assert_eq!(
            parse_local_center(&serde_json::json!({ "x": "5", "y": 1.0 })),
            None
        );
        assert_eq!(
            parse_local_center(&serde_json::json!({ "x": 1.0, "y": f64::NAN })),
            None
        );
        assert_eq!(
            parse_local_center(&serde_json::json!({ "x": f64::INFINITY, "y": 1.0 })),
            None
        );
    }

    #[test]
    fn parse_local_center_non_object_is_none() {
        assert_eq!(parse_local_center(&serde_json::json!(null)), None);
        assert_eq!(parse_local_center(&serde_json::json!([1.0, 2.0])), None);
        assert_eq!(parse_local_center(&serde_json::Value::Null), None);
    }

    #[test]
    fn build_count_js_embeds_selector_and_counts_visible() {
        let js = build_count_js(".task-btn");
        assert!(js.contains("document.querySelectorAll('.task-btn')"));
        // Only counts elements with non-zero size (visible).
        assert!(js.contains("r.width > 0 && r.height > 0"));
        assert!(js.contains("count++"));
        assert!(js.contains("return count"));
    }

    #[test]
    fn build_count_js_escapes_selector() {
        let js = build_count_js("button[onclick=\"f('x')\"]");
        assert!(js.contains("f(\\'x\\')"));
    }

    // ── Additional edge-case coverage ────────────────────────────────

    #[test]
    fn scheme_host_with_port() {
        assert_eq!(
            scheme_host("http://127.0.0.1:8080/path"),
            Some(("http", "127.0.0.1:8080"))
        );
    }

    #[test]
    fn scheme_host_with_query_and_fragment() {
        assert_eq!(
            scheme_host("https://example.com/page?q=1#top"),
            Some(("https", "example.com"))
        );
    }

    #[test]
    fn scheme_host_no_path() {
        assert_eq!(
            scheme_host("ws://127.0.0.1:9222"),
            Some(("ws", "127.0.0.1:9222"))
        );
    }

    #[test]
    fn scheme_host_empty_string() {
        assert_eq!(scheme_host(""), None);
    }

    #[test]
    fn scheme_host_no_scheme() {
        assert_eq!(scheme_host("example.com/path"), None);
    }

    #[test]
    fn scheme_host_wss() {
        assert_eq!(
            scheme_host("wss://secure.example.com/ws"),
            Some(("wss", "secure.example.com"))
        );
    }

    #[test]
    fn escape_js_string_empty() {
        assert_eq!(escape_js_string(""), "");
    }

    #[test]
    fn escape_js_string_multiple_quotes() {
        assert_eq!(escape_js_string("a'b\"c"), "a\\'b\"c");
    }

    #[test]
    fn escape_js_string_multiple_backslashes() {
        assert_eq!(escape_js_string("\\\\"), "\\\\\\\\");
    }

    #[test]
    fn escape_js_string_mixed() {
        assert_eq!(
            escape_js_string("it's a \\backslash"),
            "it\\'s a \\\\backslash"
        );
    }

    #[test]
    fn build_element_js_with_complex_selector() {
        let js = build_element_js("div.card > button.primary", None, None);
        assert!(js.contains("div.card > button.primary"));
        assert!(js.contains("const skipX = null, skipY = null;"));
    }

    #[test]
    fn build_element_js_skip_with_negative_coords() {
        let js = build_element_js(".btn", Some((-10.0, -5.5)), None);
        assert!(js.contains("const skipX = -10, skipY = -5.5;"));
    }

    #[test]
    fn build_element_js_skip_with_zero() {
        let js = build_element_js(".btn", Some((0.0, 0.0)), None);
        assert!(js.contains("const skipX = 0, skipY = 0;"));
    }

    #[test]
    fn build_element_js_with_text_filter() {
        let js = build_element_js(".btn-small#btn-youtube_like_comment", None, Some("Go"));
        assert!(js.contains("const textFilter = 'Go';"));
        assert!(js.contains("(el.textContent || '').trim() !== textFilter"));
    }

    #[test]
    fn build_element_js_without_text_filter_sets_null() {
        let js = build_element_js(".btn", None, None);
        assert!(js.contains("const textFilter = null;"));
    }

    #[test]
    fn build_element_js_escapes_text_filter_quotes() {
        let js = build_element_js(".btn", None, Some("it's Go"));
        assert!(js.contains("const textFilter = 'it\\'s Go';"));
    }

    #[test]
    fn build_resolve_iframe_js_with_special_chars_in_selector() {
        let js = build_resolve_iframe_js("iframe[data-role='payment']");
        assert!(js.contains("iframe[data-role=\\'payment\\']"));
        assert!(js.contains("querySelector"));
    }

    #[test]
    fn build_resolve_iframe_js_xpath_deep_nesting() {
        let xpath = "/html/body/div[1]/div[2]/div[3]/iframe";
        let js = build_resolve_iframe_js(xpath);
        assert!(js.contains("q.startsWith('/')"));
        assert!(js.contains(xpath));
    }

    #[test]
    fn build_element_js_selector_with_single_quotes() {
        let js = build_element_js("input[name='user']", None, None);
        // Single quotes in selector get escaped in JS string
        assert!(js.contains("input[name=\\'user\\']"));
    }
}
