//! DOM element interaction and inspection utilities.

use anyhow::Result;
#[cfg(feature = "accessibility-locator")]
use chromiumoxide::cdp::browser_protocol::accessibility::{
    AxNode, EnableParams, QueryAxTreeParams,
};
#[cfg(feature = "accessibility-locator")]
use chromiumoxide::cdp::browser_protocol::dom::{
    GetBoxModelParams, GetDocumentParams, QuerySelectorParams,
};
use chromiumoxide::Page;
use tokio::time::{timeout, Duration};
use tracing::debug;

#[cfg(feature = "accessibility-locator")]
use crate::utils::accessibility_locator::{
    parse_selector_input, AccessibilityLocator, LocatorMatchMode, ParsedSelector,
};
use crate::utils::page_size;

pub async fn focus(page: &Page, selector: &str) -> Result<()> {
    let selector_json = serde_json::to_string(selector)?;
    let js = format!(
        r"(() => {{
            const el = document.querySelector({selector_json});
            if (!el) return false;

            if (typeof el.focus === 'function') {{
                try {{
                    el.focus({{ preventScroll: true }});
                }} catch (_) {{
                    el.focus();
                }}
            }}

            const active = document.activeElement;
            return active === el || (active && el.contains(active));
        }})()",
    );

    page.evaluate(js).await?;
    Ok(())
}

pub async fn selector_exists(page: &Page, selector: &str) -> Result<bool> {
    #[cfg(feature = "accessibility-locator")]
    {
        match parse_selector_for_navigation(selector)? {
            ParsedSelector::Css(css) => {
                let found = css_selector_exists(page, &css).await?;
                emit_selector_observation(
                    "css",
                    None,
                    if found { "ok" } else { "not_found" },
                    None,
                    None,
                );
                Ok(found)
            }
            ParsedSelector::Accessibility(locator) => {
                let nodes = query_ax_nodes(page, &locator).await?;
                let classification = classify_locator_exists(nodes.len());
                if classification == "not_found" {
                    emit_selector_observation(
                        "a11y",
                        Some(&locator.role),
                        classification,
                        Some(locator_match_mode_name(locator.match_mode)),
                        locator.scope.as_deref(),
                    );
                    Err(locator_not_found_error(&locator))
                } else if classification == "ambiguous" {
                    emit_selector_observation(
                        "a11y",
                        Some(&locator.role),
                        classification,
                        Some(locator_match_mode_name(locator.match_mode)),
                        locator.scope.as_deref(),
                    );
                    Err(anyhow::anyhow!(
                        "locator_ambiguous: role='{}' name='{}' matched {} nodes",
                        locator.role,
                        locator.name,
                        nodes.len()
                    ))
                } else {
                    emit_selector_observation(
                        "a11y",
                        Some(&locator.role),
                        classification,
                        Some(locator_match_mode_name(locator.match_mode)),
                        locator.scope.as_deref(),
                    );
                    Ok(true)
                }
            }
        }
    }

    #[cfg(not(feature = "accessibility-locator"))]
    {
        let found = css_selector_exists(page, selector).await?;
        emit_selector_observation(
            "css",
            None,
            if found { "ok" } else { "not_found" },
            None,
            None,
        );
        Ok(found)
    }
}

pub async fn selector_is_visible(page: &Page, selector: &str) -> Result<bool> {
    #[cfg(feature = "accessibility-locator")]
    {
        match parse_selector_for_navigation(selector)? {
            ParsedSelector::Css(css) => {
                let found = css_selector_is_visible(page, &css).await?;
                emit_selector_observation(
                    "css",
                    None,
                    if found { "ok" } else { "not_found" },
                    None,
                    None,
                );
                Ok(found)
            }
            ParsedSelector::Accessibility(locator) => {
                let nodes = query_ax_nodes(page, &locator).await?;
                let visible_count = nodes.iter().filter(|n| ax_node_is_visible(n)).count();
                let classification = classify_locator_visible(nodes.len(), visible_count);
                if classification == "ambiguous" {
                    emit_selector_observation(
                        "a11y",
                        Some(&locator.role),
                        classification,
                        Some(locator_match_mode_name(locator.match_mode)),
                        locator.scope.as_deref(),
                    );
                    Err(anyhow::anyhow!(
                        "locator_ambiguous: role='{}' name='{}' matched {} visible nodes",
                        locator.role,
                        locator.name,
                        visible_count
                    ))
                } else if classification == "not_found" {
                    emit_selector_observation(
                        "a11y",
                        Some(&locator.role),
                        classification,
                        Some(locator_match_mode_name(locator.match_mode)),
                        locator.scope.as_deref(),
                    );
                    Err(locator_not_found_error(&locator))
                } else {
                    emit_selector_observation(
                        "a11y",
                        Some(&locator.role),
                        classification,
                        Some(locator_match_mode_name(locator.match_mode)),
                        locator.scope.as_deref(),
                    );
                    Ok(true)
                }
            }
        }
    }

    #[cfg(not(feature = "accessibility-locator"))]
    {
        let found = css_selector_is_visible(page, selector).await?;
        emit_selector_observation(
            "css",
            None,
            if found { "ok" } else { "not_found" },
            None,
            None,
        );
        Ok(found)
    }
}

pub async fn selector_text(page: &Page, selector: &str) -> Result<Option<String>> {
    #[cfg(feature = "accessibility-locator")]
    {
        match parse_selector_for_navigation(selector)? {
            ParsedSelector::Css(css) => {
                let value = css_selector_text(page, &css).await?;
                emit_selector_observation(
                    "css",
                    None,
                    if value.is_some() { "ok" } else { "not_found" },
                    None,
                    None,
                );
                Ok(value)
            }
            ParsedSelector::Accessibility(locator) => {
                let nodes = query_ax_nodes(page, &locator).await?;
                let value = nodes.first().and_then(ax_node_accessible_name);
                let classification = classify_locator_text(nodes.len(), value.is_some());
                if classification == "ambiguous" {
                    emit_selector_observation(
                        "a11y",
                        Some(&locator.role),
                        classification,
                        Some(locator_match_mode_name(locator.match_mode)),
                        locator.scope.as_deref(),
                    );
                    Err(anyhow::anyhow!(
                        "locator_ambiguous: role='{}' name='{}' matched {} nodes",
                        locator.role,
                        locator.name,
                        nodes.len()
                    ))
                } else if classification == "not_found" {
                    emit_selector_observation(
                        "a11y",
                        Some(&locator.role),
                        classification,
                        Some(locator_match_mode_name(locator.match_mode)),
                        locator.scope.as_deref(),
                    );
                    Err(locator_not_found_error(&locator))
                } else {
                    emit_selector_observation(
                        "a11y",
                        Some(&locator.role),
                        classification,
                        Some(locator_match_mode_name(locator.match_mode)),
                        locator.scope.as_deref(),
                    );
                    Ok(value)
                }
            }
        }
    }

    #[cfg(not(feature = "accessibility-locator"))]
    {
        let value = css_selector_text(page, selector).await?;
        emit_selector_observation(
            "css",
            None,
            if value.is_some() { "ok" } else { "not_found" },
            None,
            None,
        );
        Ok(value)
    }
}

#[must_use]
pub fn selector_uses_accessibility_locator(selector: &str) -> bool {
    #[cfg(feature = "accessibility-locator")]
    {
        selector.trim_start().starts_with("role=")
    }
    #[cfg(not(feature = "accessibility-locator"))]
    {
        let _ = selector;
        false
    }
}

pub async fn selector_action_point(page: &Page, selector: &str) -> Result<(f64, f64)> {
    #[cfg(feature = "accessibility-locator")]
    {
        match parse_selector_for_navigation(selector)? {
            ParsedSelector::Css(css) => page_size::get_element_center(page, &css).await,
            ParsedSelector::Accessibility(locator) => ax_locator_action_point(page, &locator).await,
        }
    }

    #[cfg(not(feature = "accessibility-locator"))]
    {
        page_size::get_element_center(page, selector).await
    }
}

pub async fn focus_at_point(page: &Page, x: f64, y: f64) -> Result<()> {
    let js = format!(
        r"(() => {{
            const el = document.elementFromPoint({x}, {y});
            if (!el) return false;
            if (typeof el.focus === 'function') {{
                try {{
                    el.focus({{ preventScroll: true }});
                }} catch (_) {{
                    el.focus();
                }}
            }}
            const active = document.activeElement;
            return active === el || (active && el.contains(active));
        }})()"
    );
    let result = page.evaluate(js).await?;
    let focused = result
        .value()
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !focused {
        anyhow::bail!("[task-api] focus: no focusable element at resolved action point");
    }
    Ok(())
}

fn emit_selector_observation(
    selector_mode: &str,
    locator_role: Option<&str>,
    locator_result: &str,
    locator_match_mode: Option<&str>,
    locator_scope_used: Option<&str>,
) {
    debug!(
        selector_mode,
        locator_role = locator_role.unwrap_or(""),
        locator_result,
        locator_match_mode = locator_match_mode.unwrap_or(""),
        locator_scope_used = locator_scope_used.unwrap_or(""),
        "selector resolution"
    );
}

#[cfg(feature = "accessibility-locator")]
fn locator_not_found_error(locator: &AccessibilityLocator) -> anyhow::Error {
    anyhow::anyhow!(
        "locator_not_found: role='{}' name='{}'",
        locator.role,
        locator.name
    )
}

#[cfg(feature = "accessibility-locator")]
fn locator_unsupported_error(operation: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "locator_unsupported: operation='{}' requires css selector",
        operation
    )
}

#[cfg(feature = "accessibility-locator")]
fn locator_match_mode_name(match_mode: LocatorMatchMode) -> &'static str {
    match match_mode {
        LocatorMatchMode::Exact => "exact",
        LocatorMatchMode::Contains => "contains",
    }
}

#[cfg(feature = "accessibility-locator")]
fn classify_locator_exists(nodes_len: usize) -> &'static str {
    if nodes_len == 0 {
        "not_found"
    } else if nodes_len > 1 {
        "ambiguous"
    } else {
        "ok"
    }
}

#[cfg(feature = "accessibility-locator")]
fn classify_locator_visible(nodes_len: usize, visible_count: usize) -> &'static str {
    if nodes_len == 0 || visible_count == 0 {
        "not_found"
    } else if visible_count > 1 {
        "ambiguous"
    } else {
        "ok"
    }
}

#[cfg(feature = "accessibility-locator")]
fn classify_locator_text(nodes_len: usize, has_text: bool) -> &'static str {
    if nodes_len == 0 || !has_text {
        "not_found"
    } else if nodes_len > 1 {
        "ambiguous"
    } else {
        "ok"
    }
}

#[cfg(feature = "accessibility-locator")]
async fn ax_locator_action_point(
    page: &Page,
    locator: &AccessibilityLocator,
) -> Result<(f64, f64)> {
    let nodes = query_ax_nodes(page, locator).await?;
    let visible_nodes: Vec<&AxNode> = nodes.iter().filter(|n| ax_node_is_visible(n)).collect();

    if visible_nodes.is_empty() {
        return Err(locator_not_found_error(locator));
    }
    if visible_nodes.len() > 1 {
        return Err(anyhow::anyhow!(
            "locator_ambiguous: role='{}' name='{}' matched {} visible nodes",
            locator.role,
            locator.name,
            visible_nodes.len()
        ));
    }

    let backend_node_id = visible_nodes[0]
        .backend_dom_node_id
        .ok_or_else(|| locator_not_found_error(locator))?;
    let box_model = page
        .execute(
            GetBoxModelParams::builder()
                .backend_node_id(backend_node_id)
                .build(),
        )
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "locator_not_found: role='{}' name='{}' ({})",
                locator.role,
                locator.name,
                e
            )
        })?;
    quad_center(box_model.model.content.inner()).ok_or_else(|| locator_not_found_error(locator))
}

/// Compute the center of a content quad (4 pairs of x,y coordinates).
#[cfg(any(feature = "accessibility-locator", test))]
fn quad_center(points: &[f64]) -> Option<(f64, f64)> {
    if points.len() < 8 {
        return None;
    }
    let x = [points[0], points[2], points[4], points[6]];
    let y = [points[1], points[3], points[5], points[7]];
    if x.iter().any(|v| !v.is_finite()) || y.iter().any(|v| !v.is_finite()) {
        return None;
    }
    Some((x.iter().sum::<f64>() / 4.0, y.iter().sum::<f64>() / 4.0))
}

async fn css_selector_exists(page: &Page, selector: &str) -> Result<bool> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r"(() => {{
            return !!document.querySelector({selector_js});
        }})()"
    );
    let result = page.evaluate(js).await?;
    Ok(result
        .value()
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false))
}

async fn css_selector_is_visible(page: &Page, selector: &str) -> Result<bool> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return false;
            const rect = el.getBoundingClientRect();
            if (rect.width <= 0 || rect.height <= 0) return false;
            const style = getComputedStyle(el);
            if (style.display === 'none' || style.visibility === 'hidden') return false;

            // Phase2: Check if element is actually in the viewport
            const windowHeight = window.innerHeight || document.documentElement.clientHeight;
            const windowWidth = window.innerWidth || document.documentElement.clientWidth;
            if (rect.top >= windowHeight || rect.bottom <= 0) return false;
            if (rect.left >= windowWidth || rect.right <= 0) return false;

            return true;
        }})()",
    );

    let result = page.evaluate(js).await?;
    Ok(result
        .value()
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false))
}

async fn css_selector_text(page: &Page, selector: &str) -> Result<Option<String>> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return null;
            const text = (el.innerText || el.textContent || "").trim();
            return text.length ? text : null;
        }})()"#,
    );

    let result = page.evaluate(js).await?;
    Ok(result
        .value()
        .and_then(|v| v.as_str().map(std::string::ToString::to_string)))
}

#[cfg(feature = "accessibility-locator")]
fn parse_selector_for_navigation(selector: &str) -> Result<ParsedSelector> {
    parse_selector_input(selector).map_err(|e| anyhow::anyhow!("locator_parse_error: {}", e))
}

#[cfg(feature = "accessibility-locator")]
async fn query_ax_nodes(page: &Page, locator: &AccessibilityLocator) -> Result<Vec<AxNode>> {
    page.execute(EnableParams::default()).await?;
    let root = page.execute(GetDocumentParams::default()).await?;
    let mut scope_node_id = root.root.node_id;

    if let Some(scope_css) = &locator.scope {
        let scope_result = page
            .execute(QuerySelectorParams::new(scope_node_id, scope_css))
            .await
            .map_err(|e| anyhow::anyhow!("locator_scope_invalid: {}", e))?;
        scope_node_id = scope_result.node_id;
    }

    let mut query = QueryAxTreeParams::builder()
        .node_id(scope_node_id)
        .role(locator.role.clone());
    if matches!(locator.match_mode, LocatorMatchMode::Exact) {
        query = query.accessible_name(locator.name.clone());
    }
    let response = page.execute(query.build()).await?;

    let nodes = if matches!(locator.match_mode, LocatorMatchMode::Contains) {
        response
            .nodes
            .clone()
            .into_iter()
            .filter(|n| {
                ax_node_accessible_name(n)
                    .map(|name| name.contains(&locator.name))
                    .unwrap_or(false)
            })
            .collect()
    } else {
        response.nodes.clone()
    };

    Ok(nodes)
}

#[cfg(feature = "accessibility-locator")]
fn ax_node_accessible_name(node: &AxNode) -> Option<String> {
    node.name
        .as_ref()
        .and_then(|v| v.value.as_ref())
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(feature = "accessibility-locator")]
fn ax_node_value(node: &AxNode) -> Option<String> {
    node.value
        .as_ref()
        .and_then(|v| v.value.as_ref())
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(feature = "accessibility-locator")]
fn ax_node_is_visible(node: &AxNode) -> bool {
    if node.ignored {
        return false;
    }
    let hidden_reason = node.ignored_reasons.as_ref().map(|reasons| {
        reasons.iter().any(|reason| {
            matches!(
                reason.name.as_ref(),
                "notVisible"
                    | "notRendered"
                    | "ariaHiddenElement"
                    | "ariaHiddenSubtree"
                    | "inertElement"
                    | "inertSubtree"
                    | "hiddenRoot"
            )
        })
    });
    !hidden_reason.unwrap_or(false)
}

pub async fn selector_html(page: &Page, selector: &str) -> Result<Option<String>> {
    #[cfg(feature = "accessibility-locator")]
    {
        match parse_selector_for_navigation(selector)? {
            ParsedSelector::Css(css) => {
                let value = css_selector_html(page, &css).await?;
                emit_selector_observation(
                    "css",
                    None,
                    if value.is_some() { "ok" } else { "not_found" },
                    None,
                    None,
                );
                Ok(value)
            }
            ParsedSelector::Accessibility(locator) => {
                emit_selector_observation(
                    "a11y",
                    Some(&locator.role),
                    "unsupported",
                    Some(locator_match_mode_name(locator.match_mode)),
                    locator.scope.as_deref(),
                );
                Err(locator_unsupported_error("html"))
            }
        }
    }

    #[cfg(not(feature = "accessibility-locator"))]
    {
        let value = css_selector_html(page, selector).await?;
        emit_selector_observation(
            "css",
            None,
            if value.is_some() { "ok" } else { "not_found" },
            None,
            None,
        );
        Ok(value)
    }
}

pub async fn selector_attr(page: &Page, selector: &str, name: &str) -> Result<Option<String>> {
    #[cfg(feature = "accessibility-locator")]
    {
        match parse_selector_for_navigation(selector)? {
            ParsedSelector::Css(css) => {
                let value = css_selector_attr(page, &css, name).await?;
                emit_selector_observation(
                    "css",
                    None,
                    if value.is_some() { "ok" } else { "not_found" },
                    None,
                    None,
                );
                Ok(value)
            }
            ParsedSelector::Accessibility(locator) => {
                emit_selector_observation(
                    "a11y",
                    Some(&locator.role),
                    "unsupported",
                    Some(locator_match_mode_name(locator.match_mode)),
                    locator.scope.as_deref(),
                );
                Err(locator_unsupported_error("attr"))
            }
        }
    }

    #[cfg(not(feature = "accessibility-locator"))]
    {
        let value = css_selector_attr(page, selector, name).await?;
        emit_selector_observation(
            "css",
            None,
            if value.is_some() { "ok" } else { "not_found" },
            None,
            None,
        );
        Ok(value)
    }
}

pub async fn selector_value(page: &Page, selector: &str) -> Result<Option<String>> {
    #[cfg(feature = "accessibility-locator")]
    {
        match parse_selector_for_navigation(selector)? {
            ParsedSelector::Css(css) => {
                let value = css_selector_value(page, &css).await?;
                emit_selector_observation(
                    "css",
                    None,
                    if value.is_some() { "ok" } else { "not_found" },
                    None,
                    None,
                );
                Ok(value)
            }
            ParsedSelector::Accessibility(locator) => {
                let nodes = query_ax_nodes(page, &locator).await?;
                let value = nodes.first().and_then(ax_node_value);
                let classification = classify_locator_text(nodes.len(), value.is_some());
                if classification == "ambiguous" {
                    emit_selector_observation(
                        "a11y",
                        Some(&locator.role),
                        classification,
                        Some(locator_match_mode_name(locator.match_mode)),
                        locator.scope.as_deref(),
                    );
                    Err(anyhow::anyhow!(
                        "locator_ambiguous: role='{}' name='{}' matched {} nodes",
                        locator.role,
                        locator.name,
                        nodes.len()
                    ))
                } else if classification == "not_found" {
                    emit_selector_observation(
                        "a11y",
                        Some(&locator.role),
                        classification,
                        Some(locator_match_mode_name(locator.match_mode)),
                        locator.scope.as_deref(),
                    );
                    Err(locator_not_found_error(&locator))
                } else {
                    emit_selector_observation(
                        "a11y",
                        Some(&locator.role),
                        classification,
                        Some(locator_match_mode_name(locator.match_mode)),
                        locator.scope.as_deref(),
                    );
                    Ok(value)
                }
            }
        }
    }

    #[cfg(not(feature = "accessibility-locator"))]
    {
        let value = css_selector_value(page, selector).await?;
        emit_selector_observation(
            "css",
            None,
            if value.is_some() { "ok" } else { "not_found" },
            None,
            None,
        );
        Ok(value)
    }
}

async fn css_selector_html(page: &Page, selector: &str) -> Result<Option<String>> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return null;
            const html = (el.innerHTML || "").trim();
            return html.length ? html : null;
        }})()"#,
    );

    let result = page.evaluate(js).await?;
    Ok(result
        .value()
        .and_then(|v| v.as_str().map(std::string::ToString::to_string)))
}

async fn css_selector_attr(page: &Page, selector: &str, name: &str) -> Result<Option<String>> {
    let selector_js = serde_json::to_string(selector)?;
    let name_js = serde_json::to_string(name)?;
    let js = format!(
        r"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return null;
            const value = el.getAttribute({name_js});
            if (value == null) return null;
            const trimmed = String(value).trim();
            return trimmed.length ? trimmed : null;
        }})()",
    );

    let result = page.evaluate(js).await?;
    Ok(result
        .value()
        .and_then(|v| v.as_str().map(std::string::ToString::to_string)))
}

async fn css_selector_value(page: &Page, selector: &str) -> Result<Option<String>> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return null;
            const value = typeof el.value === 'string' ? el.value : null;
            if (value == null) return null;
            const trimmed = String(value).trim();
            return trimmed.length ? trimmed : null;
        }})()",
    );

    let result = page.evaluate(js).await?;
    Ok(result
        .value()
        .and_then(|v| v.as_str().map(std::string::ToString::to_string)))
}

pub async fn wait_for_selector(page: &Page, selector: &str, timeout_ms: u64) -> Result<bool> {
    match timeout(Duration::from_millis(timeout_ms), async {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            if selector_exists(page, selector).await.unwrap_or(false) {
                return Ok(true);
            } else if std::time::Instant::now() >= deadline {
                return Ok(false);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => Ok(false), // Timeout elapsed, selector not found
    }
}

pub async fn wait_for_visible_selector(
    page: &Page,
    selector: &str,
    timeout_ms: u64,
) -> Result<bool> {
    match timeout(Duration::from_millis(timeout_ms), async {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            if selector_is_visible(page, selector).await.unwrap_or(false) {
                return Ok(true);
            } else if std::time::Instant::now() >= deadline {
                return Ok(false);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => Ok(false), // Timeout elapsed, selector not found
    }
}

pub async fn wait_for_any_visible_selector(
    page: &Page,
    selectors: &[&str],
    timeout_ms: u64,
) -> Result<bool> {
    match timeout(Duration::from_millis(timeout_ms), async {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            for selector in selectors {
                if selector_is_visible(page, selector).await.unwrap_or(false) {
                    return Ok(true);
                }
            }

            if std::time::Instant::now() >= deadline {
                return Ok(false);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => Ok(false), // Timeout elapsed, no selector found
    }
}

#[cfg(test)]
mod tests {
    use super::{quad_center, selector_uses_accessibility_locator};

    #[test]
    fn plain_css_selector_is_not_an_accessibility_locator() {
        assert!(!selector_uses_accessibility_locator("button.btn-primary"));
        assert!(!selector_uses_accessibility_locator("#submit"));
        assert!(!selector_uses_accessibility_locator(""));
    }

    #[cfg(feature = "accessibility-locator")]
    #[test]
    fn role_prefix_is_an_accessibility_locator() {
        assert!(selector_uses_accessibility_locator("role=button"));
        assert!(selector_uses_accessibility_locator(
            "role=button name=Submit"
        ));
        assert!(selector_uses_accessibility_locator("  role=button")); // leading whitespace trimmed
    }

    #[test]
    fn quad_center_computes_average_of_corners() {
        // (0,0) (100,0) (100,100) (0,100) -> center (50,50)
        let quad = [0.0, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0];
        assert_eq!(quad_center(&quad), Some((50.0, 50.0)));
    }

    #[test]
    fn quad_center_requires_eight_points() {
        assert_eq!(quad_center(&[]), None);
        assert_eq!(quad_center(&[0.0, 0.0, 1.0, 1.0, 2.0, 2.0]), None);
        assert_eq!(quad_center(&[0.0; 7]), None);
    }

    #[test]
    fn quad_center_rejects_non_finite_values() {
        assert_eq!(
            quad_center(&[f64::NAN, 0.0, 100.0, 0.0, 100.0, 100.0, 0.0, 100.0]),
            None
        );
        assert_eq!(
            quad_center(&[0.0, 0.0, f64::INFINITY, 0.0, 100.0, 100.0, 0.0, 100.0]),
            None
        );
    }

    #[test]
    fn quad_center_handles_negative_coordinates() {
        // (-10,-10) (10,-10) (10,10) (-10,10) -> (0,0)
        let quad = [-10.0, -10.0, 10.0, -10.0, 10.0, 10.0, -10.0, 10.0];
        assert_eq!(quad_center(&quad), Some((0.0, 0.0)));
    }
}
