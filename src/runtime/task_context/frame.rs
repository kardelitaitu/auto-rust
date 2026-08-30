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
use crate::runtime::task_context::oopif::OopifClient;
use crate::runtime::task_context::{Rect, TaskContext};
use crate::utils::{ClickOutcome, ClickStatus};
use serde_json::Value;

/// Interval between poll iterations while waiting for an iframe/element to
/// become usable.
const IFRAME_POLL_INTERVAL: Duration = Duration::from_millis(200);

impl TaskContext {
    /// Open ONE raw CDP connection for the duration of a public call (reused
    /// across that call's poll loop). Fails fast when the browser debug URL is
    /// not configured or unreachable — instead of degrading into a misleading
    /// "element not found" after the full timeout.
    async fn oopif_client(&self) -> Result<OopifClient> {
        if self.browser_ws_url.is_empty() {
            return Err(anyhow!(
                "OOPIF iframe access requires a browser debug WebSocket URL \
                 (browser_ws_url is not configured)"
            ));
        }
        OopifClient::connect(&self.browser_ws_url)
            .await
            .map_err(|e| {
                anyhow!(
                    "OOPIF client connect to '{}' failed: {e}",
                    self.browser_ws_url
                )
            })
    }

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

    /// Resolve an element's LOCAL center inside the iframe (OOPIF-safe), without
    /// clicking. Returns `Ok(None)` when the element is not yet found. Combine
    /// with [`Self::iframe_click_at`] + a small random offset to click at varied
    /// positions instead of the exact same center every time.
    pub async fn iframe_center(
        &self,
        iframe_selector: &str,
        element_selector: &str,
        timeout_ms: u64,
    ) -> Result<Option<(f64, f64)>> {
        let client = self.oopif_client().await?;
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut session_cache: Option<(String, String)> = None;

        loop {
            if let Ok(Some((rect, src))) = resolve_iframe(self.page(), iframe_selector).await {
                if rect.width > 0.0 && rect.height > 0.0 {
                    if let Ok(Some(center)) = self
                        .element_local_center(
                            &client,
                            &src,
                            element_selector,
                            None,
                            None,
                            Some(&mut session_cache),
                        )
                        .await
                    {
                        return Ok(Some(center));
                    }
                }
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            tokio::time::sleep(IFRAME_POLL_INTERVAL).await;
        }
    }

    /// Evaluate arbitrary JS inside the iframe (OOPIF-safe) and return its JSON
    /// value. The primary "read/scan/parse" primitive: pass a script that
    /// returns a JSON object describing the mini-app state, and drive actions
    /// from the parsed result. Returns `Err` only if the iframe cannot be
    /// reached (the script itself should return null on empty state).
    pub async fn iframe_eval(
        &self,
        iframe_selector: &str,
        expression: &str,
        timeout_ms: u64,
    ) -> Result<serde_json::Value> {
        let client = self.oopif_client().await?;
        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut session_cache: Option<(String, String)> = None;

        loop {
            if let Ok(Some((rect, src))) = resolve_iframe(self.page(), iframe_selector).await {
                if rect.width > 0.0 && rect.height > 0.0 {
                    if let Ok(Some(session_id)) = self
                        .iframe_session_id(&client, &src, Some(&mut session_cache))
                        .await
                    {
                        match client.evaluate(&session_id, expression).await {
                            Ok(value) => return Ok(value),
                            Err(e) => {
                                log::debug!(
                                    "[iframe] iframe_eval evaluate failed, clearing session cache: {e}"
                                );
                                session_cache = None;
                            }
                        }
                    }
                }
            }
            if Instant::now() >= deadline {
                return Err(anyhow!(
                    "iframe_eval: iframe '{iframe_selector}' not usable within {timeout_ms}ms"
                ));
            }
            tokio::time::sleep(IFRAME_POLL_INTERVAL).await;
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
        let client = self.oopif_client().await?;

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
                            &client,
                            &src,
                            element_selector,
                            skip,
                            text_filter,
                            Some(&mut session_cache),
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
                        Ok(None) => {
                            // Diagnose WHY the element couldn't be resolved
                            // (read-only, shares the cached session).
                            self.log_click_diag(
                                &client,
                                &src,
                                element_selector,
                                text_filter,
                                &mut session_cache,
                            )
                            .await;
                            "element not found inside iframe yet".to_string()
                        }
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
            tokio::time::sleep(IFRAME_POLL_INTERVAL).await;
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
        let client = self.oopif_client().await?;

        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut last_status = String::new();
        let mut session_cache: Option<(String, String)> = None;

        loop {
            let status = match resolve_iframe(self.page(), iframe_selector).await {
                Ok(Some((rect, src))) if rect.width > 0.0 && rect.height > 0.0 => {
                    match self
                        .element_count_in_iframe(
                            &client,
                            &src,
                            element_selector,
                            Some(&mut session_cache),
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
            tokio::time::sleep(IFRAME_POLL_INTERVAL).await;
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
            tokio::time::sleep(IFRAME_POLL_INTERVAL).await;
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

    /// Read-only diagnostic for a failed element resolve: reports how many
    /// elements matched, how many passed the text filter, the first candidate's
    /// rect/viewport state, and what `elementFromPoint` sees at its center.
    /// Shares the caller's session cache (no extra `attachToTarget` per poll)
    /// and never mutates page state — the old inline version re-attached on
    /// every iteration and scrolled the frame as a side effect.
    async fn log_click_diag(
        &self,
        client: &OopifClient,
        iframe_src: &str,
        element_selector: &str,
        text_filter: Option<&str>,
        session_cache: &mut Option<(String, String)>,
    ) {
        let Ok(Some(dsession)) = self
            .iframe_session_id(client, iframe_src, Some(session_cache))
            .await
        else {
            return;
        };
        let dsel = js_string_literal(element_selector);
        let dtf = match text_filter {
            Some(t) => format!("const textFilter = {};", js_string_literal(t)),
            None => "const textFilter = null;".to_string(),
        };
        let djs = format!(
            r#"(() => {{
                {dtf}
                const els = [...document.querySelectorAll({dsel})];
                const txt = e => (e.textContent || '').trim();
                const inVp = r => {{
                    if (!r || r.width <= 0 || r.height <= 0) return false;
                    const cx = r.x + r.width / 2;
                    const cy = r.y + r.height / 2;
                    return cx >= 0 && cy >= 0 && cx <= innerWidth && cy <= innerHeight;
                }};
                const withText = els.filter(e => textFilter === null || txt(e) === textFilter);
                const first = withText[0];
                const r = first ? first.getBoundingClientRect() : null;
                return JSON.stringify({{
                    matches: els.length,
                    withText: withText.length,
                    innerW: innerWidth,
                    innerH: innerHeight,
                    scrollY: Math.round(window.scrollY),
                    docH: document.documentElement.scrollHeight,
                    rect: r ? [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)] : null,
                    inViewport: r ? inVp(r) : false,
                    hit: (() => {{
                        if (!r) return null;
                        const h = document.elementFromPoint(r.x + r.width/2, r.y + r.height/2);
                        return h ? (h.id || h.className || h.tagName).toString().slice(0,40) : 'null';
                    }})()
                }});
            }})()"#
        );
        match client.evaluate(&dsession, &djs).await {
            Ok(dval) => log::info!("[iframe_click] '{element_selector}' resolve diag: {dval}"),
            Err(e) => log::debug!("[iframe_click] resolve diag failed: {e}"),
        }
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
        client: &OopifClient,
        iframe_src: &str,
        element_selector: &str,
        skip: Option<(f64, f64)>,
        text_filter: Option<&str>,
        mut session_cache: Option<&mut Option<(String, String)>>,
    ) -> Result<Option<(f64, f64)>> {
        let session_id = match self
            .iframe_session_id(client, iframe_src, session_cache.as_deref_mut())
            .await
        {
            Ok(Some(sid)) => sid,
            _ => return Ok(None),
        };
        let js = build_element_js(element_selector, skip, text_filter);
        match client.evaluate(&session_id, &js).await {
            Ok(value) => Ok(parse_local_center(&value)),
            Err(e) => {
                log::debug!(
                    "[iframe] element_local_center evaluate failed, clearing session cache: {e}"
                );
                if let Some(cache) = session_cache {
                    *cache = None;
                }
                Ok(None)
            }
        }
    }

    /// Count visible (non-zero-size) matching elements inside the iframe, using
    /// the same OOPIF session machinery. Returns `Ok(Some(n))` when the iframe
    /// target is reachable, `Ok(None)` when not yet ready.
    async fn element_count_in_iframe(
        &self,
        client: &OopifClient,
        iframe_src: &str,
        element_selector: &str,
        mut session_cache: Option<&mut Option<(String, String)>>,
    ) -> Result<Option<usize>> {
        let session_id = match self
            .iframe_session_id(client, iframe_src, session_cache.as_deref_mut())
            .await
        {
            Ok(Some(sid)) => sid,
            _ => return Ok(None),
        };
        let js = build_count_js(element_selector);
        match client.evaluate(&session_id, &js).await {
            Ok(value) => Ok(value.as_u64().map(|n| n as usize)),
            Err(e) => {
                log::debug!(
                    "[iframe] element_count_in_iframe evaluate failed, clearing session cache: {e}"
                );
                if let Some(cache) = session_cache {
                    *cache = None;
                }
                Ok(None)
            }
        }
    }

    /// Resolve the iframe target from its `src` URL and attach a session id.
    /// Reuses `session_cache` when target_id matches, avoiding redundant
    /// `attachToTarget` calls on every poll loop iteration.
    /// Returns `Ok(None)` when the target is not yet found or not ready (pollable).
    async fn iframe_session_id(
        &self,
        client: &OopifClient,
        iframe_src: &str,
        session_cache: Option<&mut Option<(String, String)>>,
    ) -> Result<Option<String>> {
        let target = match client.find_iframe_target(iframe_src).await {
            Ok(t) => t,
            Err(e) => {
                log::debug!("[iframe] find_iframe_target failed: {e}");
                return Ok(None);
            }
        };
        let Some(target_id) = target.get("targetId").and_then(Value::as_str) else {
            return Ok(None);
        };

        // Reuse cached session_id if target_id matches
        if let Some(cache) = session_cache.as_ref() {
            if let Some((cached_target_id, session_id)) = cache.as_ref() {
                if cached_target_id == target_id {
                    return Ok(Some(session_id.clone()));
                }
            }
        }

        let session_id = match client.attach(target_id).await {
            Ok(sid) => sid,
            Err(e) => {
                log::debug!("[iframe] attach to target '{target_id}' failed: {e}");
                return Ok(None);
            }
        };

        if let Some(cache) = session_cache {
            *cache = Some((target_id.to_string(), session_id.clone()));
        }

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
    let escaped = js_string_literal(element_selector);
    format!(
        r#"(() => {{
            let count = 0;
            const els = document.querySelectorAll({escaped});
            for (const el of els) {{
                const r = el.getBoundingClientRect();
                if (r.width > 0 && r.height > 0) count++;
            }}
            return count;
        }})()"#
    )
}

/// Render a Rust string as a complete JS string literal (quotes included).
/// JSON escaping is a subset of JS string syntax and covers every control
/// character — scraped text containing newlines (e.g. button labels) can no
/// longer break the generated script, unlike the old quote-only escaper.
fn js_string_literal(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
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
    let escaped = js_string_literal(element_selector);
    // A non-finite skip coordinate is meaningless — `Math.abs(cx - NaN) < 5` is
    // always false, silently disabling the skip. Treat it as "no skip".
    let finite_skip = skip.filter(|(x, y)| x.is_finite() && y.is_finite());
    let skip_js = match finite_skip {
        Some((sx, sy)) => format!("const skipX = {sx}, skipY = {sy};"),
        None => "const skipX = null, skipY = null;".to_string(),
    };
    let text_js = match text_filter {
        Some(text) => format!("const textFilter = {};", js_string_literal(text)),
        None => "const textFilter = null;".to_string(),
    };
    format!(
        r#"(() => {{
            {skip_js}
            {text_js}
            function scrollIntoFullView(el) {{
                el.scrollIntoView({{ block: 'center', inline: 'center' }});
                // Some mini-apps scroll their own window/document instead of a
                // container — scroll the window to center the element too.
                const er = el.getBoundingClientRect();
                window.scrollTo(0, er.top + window.scrollY - window.innerHeight / 2 + er.height / 2);
                // Some mini-apps use nested scroll containers that scrollIntoView
                // cannot reach — scroll each scrollable ancestor manually so the
                // element's center lands inside the iframe viewport.
                let node = el.parentElement;
                while (node) {{
                    const cs = getComputedStyle(node);
                    const overflowY = cs.overflowY;
                    const scrollable = overflowY === 'auto' || overflowY === 'scroll' || overflowY === 'overlay';
                    if (scrollable && node.scrollHeight > node.clientHeight) {{
                        const r2 = el.getBoundingClientRect();
                        const nr = node.getBoundingClientRect();
                        const target = node.scrollTop + (r2.top - nr.top) - node.clientHeight / 2 + r2.height / 2;
                        node.scrollTop = Math.max(0, Math.min(target, node.scrollHeight - node.clientHeight));
                    }}
                    node = node.parentElement;
                }}
                // Ancestor scrolling moved the element — recenter the window.
                const r3 = el.getBoundingClientRect();
                window.scrollTo(0, r3.top + window.scrollY - window.innerHeight / 2 + r3.height / 2);
            }}
            function inViewport(r) {{
                if (r.width <= 0 || r.height <= 0) return false;
                const cx = r.x + r.width / 2;
                const cy = r.y + r.height / 2;
                return cx >= 0 && cy >= 0 && cx <= window.innerWidth && cy <= window.innerHeight;
            }}
            const els = document.querySelectorAll({escaped});
            // Best-effort fallback: first in-viewport candidate whose center we
            // can return even if the topmost hit-test fails (a transparent or
            // non-interactive element covering the button). We never return an
            // off-viewport / zero-size center, so we don't click blank areas.
            let fallback = null;
            for (const el of els) {{
                if (textFilter !== null && (el.textContent || '').trim() !== textFilter) continue;
                scrollIntoFullView(el);
                const r = el.getBoundingClientRect();
                if (!inViewport(r)) continue;
                const cx = r.x + r.width / 2;
                const cy = r.y + r.height / 2;
                if (skipX !== null && Math.abs(cx - skipX) < 5 && Math.abs(cy - skipY) < 5) continue;
                if (fallback === null) fallback = {{ x: cx, y: cy }};
                const hit = document.elementFromPoint(cx, cy);
                if (hit && (el === hit || el.contains(hit) || hit.contains(el))) {{
                    return {{ x: cx, y: cy }};
                }}
            }}
            return fallback;
        }})()"#
    )
}

/// Build the JS that resolves the iframe element (CSS or XPath) into its
/// viewport rect and `src` attribute.
fn build_resolve_iframe_js(selector: &str) -> String {
    let escaped = js_string_literal(selector);
    format!(
        r#"(() => {{
            try {{
                const q = {escaped};
                let el = null;
                if (q.startsWith('/')) {{
                    const xr = document.evaluate(q, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null);
                    el = xr.singleNodeValue;
                }} else {{
                    el = document.querySelector(q);
                }}
                if (!el || el.tagName.toUpperCase() !== 'IFRAME') return null;
                const r = el.getBoundingClientRect();
                // Resolve src against the document base so a relative attribute
                // (e.g. "/miner/index.html") still yields a host for target matching.
                const raw = el.getAttribute('src') || '';
                let src = raw;
                try {{
                    if (raw) src = new URL(raw, location.href).href;
                }} catch (e) {{ /* unparseable base — keep raw */ }}
                return {{
                    x: r.x, y: r.y, width: r.width, height: r.height,
                    src
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
        build_count_js, build_element_js, build_resolve_iframe_js, js_string_literal,
        parse_local_center,
    };

    #[test]
    fn js_string_literal_quotes_and_escapes() {
        assert_eq!(js_string_literal("plain"), "\"plain\"");
        assert_eq!(js_string_literal("it's"), "\"it's\"");
        assert_eq!(js_string_literal("a\"b"), "\"a\\\"b\"");
        assert_eq!(js_string_literal("a\\b"), "\"a\\\\b\"");
        assert_eq!(js_string_literal(""), "\"\"");
    }

    #[test]
    fn build_element_js_embeds_selector_and_skip() {
        let js = build_element_js("[id^=\"btn-x\"]", Some((10.5, 20.0)), None);
        assert!(js.contains(r#"querySelectorAll("[id^=\"btn-x\"]")"#));
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
        // Selector with both quote kinds must survive as one valid literal.
        let js = build_element_js("button[onclick=\"f('x')\"]", None, None);
        assert!(js.contains(r#""button[onclick=\"f('x')\"]""#));
    }

    #[test]
    fn build_element_js_requires_viewport_containment() {
        // The element's center must be inside the iframe viewport, otherwise the
        // click would land outside the iframe (below the mini-app).
        let js = build_element_js(".btn", None, None);
        assert!(js.contains("function inViewport"));
        assert!(js.contains("cx <= window.innerWidth"));
        assert!(js.contains("cy <= window.innerHeight"));
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
        assert!(css_js.contains(r#"const q = "iframe.payment-verification";"#));
        assert!(css_js.contains("document.querySelector(q)"));
        assert!(css_js.contains("tagName.toUpperCase() !== 'IFRAME'"));

        let xpath_js = build_resolve_iframe_js("/html/body/div[10]/div/div[2]/div/div/iframe");
        assert!(xpath_js.contains("q.startsWith('/')"));
        assert!(xpath_js.contains("XPathResult.FIRST_ORDERED_NODE_TYPE"));
        assert!(xpath_js.contains("error: String"));
    }

    #[test]
    fn build_resolve_iframe_js_escapes_xpath_quotes() {
        let js = build_resolve_iframe_js("//div[@id='main']/iframe");
        assert!(js.contains(r#""//div[@id='main']/iframe""#));
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
        assert!(js.contains(r#"document.querySelectorAll(".task-btn")"#));
        // Only counts elements with non-zero size (visible).
        assert!(js.contains("r.width > 0 && r.height > 0"));
        assert!(js.contains("count++"));
        assert!(js.contains("return count"));
    }

    #[test]
    fn build_count_js_escapes_selector() {
        let js = build_count_js("button[onclick=\"f('x')\"]");
        assert!(js.contains(r#""button[onclick=\"f('x')\"]""#));
    }

    // ── Additional edge-case coverage ────────────────────────────────

    #[test]
    fn js_string_literal_escapes_control_chars() {
        // Regression (B3): the old quote-only escaper let raw newlines through,
        // producing a SyntaxError inside the single-quoted JS string.
        assert_eq!(js_string_literal("Go\nConfirm"), "\"Go\\nConfirm\"");
        assert_eq!(js_string_literal("a\tb"), "\"a\\tb\"");
        assert_eq!(js_string_literal("a\r\nb"), "\"a\\r\\nb\"");
    }

    #[test]
    fn build_resolve_iframe_js_resolves_relative_src() {
        // Regression (R2): a relative src attribute must be absolutized against
        // the document base, or host matching can never find the OOPIF target.
        let js = build_resolve_iframe_js("iframe");
        assert!(js.contains("new URL(raw, location.href).href"));
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
        assert!(js.contains("const textFilter = \"Go\";"));
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
        assert!(js.contains("const textFilter = \"it's Go\";"));
    }

    #[test]
    fn build_element_js_escapes_newline_in_text_filter() {
        // Scraped button text can contain line breaks — they must be escaped,
        // never emitted raw into the generated script.
        let js = build_element_js(".btn", None, Some("Go\nConfirm"));
        assert!(js.contains(r#"const textFilter = "Go\nConfirm";"#));
        assert!(!js.contains("Go\nConfirm"));
    }

    #[test]
    fn build_resolve_iframe_js_with_special_chars_in_selector() {
        let js = build_resolve_iframe_js("iframe[data-role='payment']");
        assert!(js.contains(r#""iframe[data-role='payment']""#));
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
        // JSON-style literal: single quotes need no escaping.
        assert!(js.contains(r#"querySelectorAll("input[name='user']")"#));
    }

    #[test]
    fn build_resolve_iframe_js_checks_uppercase_tag_name() {
        let js = build_resolve_iframe_js("iframe");
        assert!(js.contains("el.tagName.toUpperCase() !== 'IFRAME'"));
    }
}
