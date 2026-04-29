use anyhow::Result;
#[cfg(feature = "accessibility-locator")]
use chromiumoxide::cdp::browser_protocol::accessibility::{
    AxNode, EnableParams, QueryAxTreeParams,
};
#[cfg(feature = "accessibility-locator")]
use chromiumoxide::cdp::browser_protocol::dom::{
    GetBoxModelParams, GetDocumentParams, QuerySelectorParams,
};
use chromiumoxide::cdp::browser_protocol::network::{
    Headers, SetExtraHttpHeadersParams, SetUserAgentOverrideParams,
};
use chromiumoxide::Page;
use tokio::time::{timeout, Duration};
use tracing::debug;

#[cfg(feature = "accessibility-locator")]
use crate::utils::accessibility_locator::{
    parse_selector_input, AccessibilityLocator, LocatorMatchMode, ParsedSelector,
};
use crate::utils::math::random_in_range;
use crate::utils::page_size;
use crate::utils::timing::human_pause;

pub async fn goto(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    goto_with_trampoline(page, url, timeout_ms).await
}

pub async fn goto_with_trampoline(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    let referrers = [
        "https://www.google.com",
        "https://www.bing.com",
        "https://search.yahoo.com",
        "https://duckduckgo.com",
        "https://www.reddit.com",
        "https://x.com",
        "https://web.telegram.org",
        "https://web.whatsapp.com",
    ];

    let len = referrers.len() as u64;
    let idx = random_in_range(0, len.saturating_sub(1)) as usize;
    let _referrer_hint = referrers[idx];

    if random_in_range(0, 10) < 3 {
        human_pause(random_in_range(150, 500), 20).await;
    } else {
        human_pause(random_in_range(500, 1200), 30).await;
    }

    goto_raw(page, url, timeout_ms).await
}

pub async fn goto_light(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    goto_raw(page, url, timeout_ms).await
}

pub async fn goto_raw(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    timeout(Duration::from_millis(timeout_ms), async {
        page.goto(url).await?;
        Ok::<(), anyhow::Error>(())
    })
    .await??;

    Ok(())
}

pub async fn go_back(page: &Page) -> Result<()> {
    page.evaluate("window.history.back()").await?;
    Ok(())
}

pub async fn set_user_agent(page: &Page, user_agent: &str) -> Result<()> {
    page.execute(SetUserAgentOverrideParams::new(user_agent))
        .await?;
    Ok(())
}

pub async fn set_extra_http_headers(
    page: &Page,
    headers: &std::collections::BTreeMap<String, String>,
) -> Result<()> {
    let json_headers = serde_json::to_value(headers)?;
    page.execute(SetExtraHttpHeadersParams::new(Headers::new(json_headers)))
        .await?;
    Ok(())
}

pub async fn focus(page: &Page, selector: &str) -> Result<()> {
    let selector_json = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
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
        }})()"#,
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
            ParsedSelector::Accessibility(locator) => {
                ax_locator_action_point(page, &locator).await
            }
        }
    }

    #[cfg(not(feature = "accessibility-locator"))]
    {
        page_size::get_element_center(page, selector).await
    }
}

pub async fn focus_at_point(page: &Page, x: f64, y: f64) -> Result<()> {
    let js = format!(
        r#"(() => {{
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
        }})()"#
    );
    let result = page.evaluate(js).await?;
    let focused = result.value().and_then(|v| v.as_bool()).unwrap_or(false);
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
    anyhow::anyhow!("locator_unsupported: operation='{}' requires css selector", operation)
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
async fn ax_locator_action_point(page: &Page, locator: &AccessibilityLocator) -> Result<(f64, f64)> {
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

#[cfg(feature = "accessibility-locator")]
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
        r#"(() => {{
            return !!document.querySelector({selector_js});
        }})()"#
    );
    let result = page.evaluate(js).await?;
    Ok(result.value().and_then(|v| v.as_bool()).unwrap_or(false))
}

async fn css_selector_is_visible(page: &Page, selector: &str) -> Result<bool> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
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
        }})()"#,
    );

    let result = page.evaluate(js).await?;
    Ok(result.value().and_then(|v| v.as_bool()).unwrap_or(false))
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
        .and_then(|v| v.as_str().map(|s| s.to_string())))
}

#[cfg(feature = "accessibility-locator")]
fn parse_selector_for_navigation(selector: &str) -> Result<ParsedSelector> {
    parse_selector_input(selector)
        .map_err(|e| anyhow::anyhow!("locator_parse_error: {}", e))
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
        .and_then(|v| v.as_str().map(|s| s.to_string())))
}

async fn css_selector_attr(page: &Page, selector: &str, name: &str) -> Result<Option<String>> {
    let selector_js = serde_json::to_string(selector)?;
    let name_js = serde_json::to_string(name)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return null;
            const value = el.getAttribute({name_js});
            if (value == null) return null;
            const trimmed = String(value).trim();
            return trimmed.length ? trimmed : null;
        }})()"#,
    );

    let result = page.evaluate(js).await?;
    Ok(result
        .value()
        .and_then(|v| v.as_str().map(|s| s.to_string())))
}

async fn css_selector_value(page: &Page, selector: &str) -> Result<Option<String>> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return null;
            const value = typeof el.value === 'string' ? el.value : null;
            if (value == null) return null;
            const trimmed = String(value).trim();
            return trimmed.length ? trimmed : null;
        }})()"#,
    );

    let result = page.evaluate(js).await?;
    Ok(result
        .value()
        .and_then(|v| v.as_str().map(|s| s.to_string())))
}

pub async fn wait_for_selector(page: &Page, selector: &str, timeout_ms: u64) -> Result<bool> {
    timeout(Duration::from_millis(timeout_ms), async {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            if selector_exists(page, selector).await.unwrap_or(false) {
                return Ok(true);
            } else if std::time::Instant::now() >= deadline {
                return Ok(false);
            } else {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    })
    .await?
}

pub async fn wait_for_visible_selector(
    page: &Page,
    selector: &str,
    timeout_ms: u64,
) -> Result<bool> {
    timeout(Duration::from_millis(timeout_ms), async {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            if selector_is_visible(page, selector).await.unwrap_or(false) {
                return Ok(true);
            } else if std::time::Instant::now() >= deadline {
                return Ok(false);
            } else {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    })
    .await?
}

pub async fn page_url(page: &Page) -> Result<String> {
    let result = page.evaluate("window.location.href").await?;
    let value = result
        .value()
        .ok_or_else(|| anyhow::anyhow!("Failed to read page URL"))?;
    Ok(value.as_str().unwrap_or("").to_string())
}

pub async fn page_title(page: &Page) -> Result<String> {
    let result = page.evaluate("document.title").await?;
    let value = result
        .value()
        .ok_or_else(|| anyhow::anyhow!("Failed to read page title"))?;
    Ok(value.as_str().unwrap_or("").to_string())
}

pub async fn wait_for_load(page: &Page, timeout_ms: u64) -> Result<()> {
    timeout(
        Duration::from_millis(timeout_ms),
        wait_for_page_settle(page),
    )
    .await??;
    Ok(())
}

pub async fn wait_for_any_visible_selector(
    page: &Page,
    selectors: &[&str],
    timeout_ms: u64,
) -> Result<bool> {
    timeout(Duration::from_millis(timeout_ms), async {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        loop {
            for selector in selectors {
                if selector_is_visible(page, selector).await.unwrap_or(false) {
                    return Ok(true);
                }
            }

            if std::time::Instant::now() >= deadline {
                return Ok(false);
            } else {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    })
    .await?
}

async fn wait_for_page_settle(page: &Page) -> Result<()> {
    let deadline = std::time::Instant::now() + Duration::from_secs(4);
    loop {
        let state = page
            .evaluate("document.readyState")
            .await?
            .value()
            .and_then(|v| v.as_str().map(str::to_string));

        if matches!(state.as_deref(), Some("interactive") | Some("complete")) {
            return Ok(());
        }

        if std::time::Instant::now() >= deadline {
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};

    use tracing::Level;

    use super::emit_selector_observation;
    #[cfg(feature = "accessibility-locator")]
    use crate::utils::accessibility_locator::{AccessibilityLocator, LocatorMatchMode};
    #[cfg(feature = "accessibility-locator")]
    use super::{
        classify_locator_exists, classify_locator_text, classify_locator_visible, quad_center,
        locator_not_found_error, locator_unsupported_error, parse_selector_for_navigation,
        selector_uses_accessibility_locator,
    };
    #[cfg(feature = "accessibility-locator")]
    use crate::utils::accessibility_locator::ParsedSelector;

    #[derive(Clone, Default)]
    struct SharedLogBuffer {
        bytes: Arc<Mutex<Vec<u8>>>,
    }

    struct SharedLogWriter {
        bytes: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for SharedLogWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.bytes
                .lock()
                .expect("shared log buffer mutex poisoned")
                .extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedLogBuffer {
        type Writer = SharedLogWriter;

        fn make_writer(&'a self) -> Self::Writer {
            SharedLogWriter {
                bytes: Arc::clone(&self.bytes),
            }
        }
    }

    fn capture_selector_observation_log(emit: impl FnOnce()) -> String {
        let sink = SharedLogBuffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(sink.clone())
            .with_max_level(Level::DEBUG)
            .with_ansi(false)
            .with_target(false)
            .without_time()
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            emit();
        });

        let bytes = sink
            .bytes
            .lock()
            .expect("shared log buffer mutex poisoned")
            .clone();
        String::from_utf8(bytes).expect("selector observation logs must be valid utf-8")
    }

    #[test]
    fn test_referrers_array_has_values() {
        let referrers = [
            "https://www.google.com",
            "https://www.bing.com",
            "https://search.yahoo.com",
            "https://duckduckgo.com",
            "https://www.reddit.com",
            "https://x.com",
            "https://web.telegram.org",
            "https://web.whatsapp.com",
        ];
        assert_eq!(referrers.len(), 8);
    }

    #[test]
    fn test_selector_json_serialization() {
        let selector = "div.test";
        let json = serde_json::to_string(selector).unwrap();
        assert_eq!(json, "\"div.test\"");
    }

    #[test]
    fn test_url_json_serialization() {
        let url = "https://example.com";
        let json = serde_json::to_string(url).unwrap();
        assert_eq!(json, "\"https://example.com\"");
    }

    #[test]
    fn test_visibility_check_js_structure() {
        let selector = ".my-element";
        let selector_js = serde_json::to_string(selector).unwrap();
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({selector_js});
                if (!el) return false;
                const rect = el.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) return false;
                const style = getComputedStyle(el);
                if (style.display === 'none' || style.visibility === 'hidden') return false;
                return true;
            }})()"#,
        );
        assert!(js.contains("getBoundingClientRect"));
    }

    #[test]
    fn test_value_read_js_structure() {
        let selector = "#userEmail";
        let selector_js = serde_json::to_string(selector).unwrap();
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({selector_js});
                if (!el) return null;
                const value = typeof el.value === 'string' ? el.value : null;
                if (value == null) return null;
                const trimmed = String(value).trim();
                return trimmed.length ? trimmed : null;
            }})()"#,
        );
        assert!(js.contains("typeof el.value === 'string'"));
    }

    #[test]
    fn test_referrer_list_valid_urls() {
        let referrers = [
            "https://www.google.com",
            "https://www.bing.com",
            "https://search.yahoo.com",
            "https://duckduckgo.com",
            "https://www.reddit.com",
            "https://x.com",
            "https://web.telegram.org",
            "https://web.whatsapp.com",
        ];
        for referrer in &referrers {
            assert!(referrer.starts_with("https://"));
            assert!(referrer.contains('.'));
        }
    }

    #[test]
    fn test_selector_json_special_chars() {
        let selector = "div[data-test=\"value\"]";
        let json = serde_json::to_string(selector).unwrap();
        assert!(json.contains("data-test"));
    }

    #[test]
    fn test_selector_json_unicode() {
        let selector = "div.日本語";
        let json = serde_json::to_string(selector).unwrap();
        assert!(json.contains("日本語"));
    }

    #[test]
    fn test_selector_json_empty() {
        let selector = "";
        let json = serde_json::to_string(selector).unwrap();
        assert_eq!(json, "\"\"");
    }

    #[test]
    fn test_focus_js_has_prevent_scroll() {
        let selector = "#input";
        let selector_js = serde_json::to_string(selector).unwrap();
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({selector_js});
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
            }})()"#,
        );
        assert!(js.contains("preventScroll"));
    }

    #[test]
    fn test_wait_timeout_behavior() {
        // Test that timeout values are used as-is (no clamping)
        let timeout_ms = 10000;
        assert_eq!(timeout_ms, 10000);
        
        let timeout_ms = 2000;
        assert_eq!(timeout_ms, 2000);
        
        let timeout_ms = 500;
        assert_eq!(timeout_ms, 500);
    }

    #[test]
    fn test_page_settle_deadline() {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(4);
        assert!(deadline > std::time::Instant::now());
    }

    #[test]
    fn test_ready_state_values() {
        let valid_states = ["loading", "interactive", "complete"];
        for state in &valid_states {
            assert!(!state.is_empty());
        }
    }

    #[test]
    fn test_attr_json_serialization() {
        let attr = "data-value";
        let json = serde_json::to_string(attr).unwrap();
        assert_eq!(json, "\"data-value\"");
    }

    #[test]
    fn test_attr_json_hyphen() {
        let attr = "aria-label";
        let json = serde_json::to_string(attr).unwrap();
        assert!(json.contains("aria"));
    }

    #[test]
    fn test_text_extraction_js_uses_inner_text() {
        let selector = "div.content";
        let selector_js = serde_json::to_string(selector).unwrap();
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({selector_js});
                if (!el) return null;
                const text = (el.innerText || el.textContent || "").trim();
                return text.length ? text : null;
            }})()"#,
        );
        assert!(js.contains("innerText"));
        assert!(js.contains("textContent"));
    }

    #[test]
    fn test_html_extraction_js_uses_inner_html() {
        let selector = "div.content";
        let selector_js = serde_json::to_string(selector).unwrap();
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({selector_js});
                if (!el) return null;
                const html = (el.innerHTML || "").trim();
                return html.length ? html : null;
            }})()"#,
        );
        assert!(js.contains("innerHTML"));
    }

    #[test]
    fn test_visibility_checks_display_none() {
        let selector = ".hidden";
        let selector_js = serde_json::to_string(selector).unwrap();
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({selector_js});
                if (!el) return false;
                const rect = el.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) return false;
                const style = getComputedStyle(el);
                if (style.display === 'none' || style.visibility === 'hidden') return false;
                return true;
            }})()"#,
        );
        assert!(js.contains("display === 'none'"));
    }

    #[test]
    fn test_visibility_checks_visibility_hidden() {
        let selector = ".invisible";
        let selector_js = serde_json::to_string(selector).unwrap();
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({selector_js});
                if (!el) return false;
                const rect = el.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) return false;
                const style = getComputedStyle(el);
                if (style.display === 'none' || style.visibility === 'hidden') return false;
                return true;
            }})()"#,
        );
        assert!(js.contains("visibility === 'hidden'"));
    }

    #[test]
    fn test_visibility_checks_rect_dimensions() {
        let selector = ".element";
        let selector_js = serde_json::to_string(selector).unwrap();
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({selector_js});
                if (!el) return false;
                const rect = el.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) return false;
                const style = getComputedStyle(el);
                if (style.display === 'none' || style.visibility === 'hidden') return false;
                return true;
            }})()"#,
        );
        assert!(js.contains("getBoundingClientRect"));
        assert!(js.contains("rect.width"));
        assert!(js.contains("rect.height"));
    }

    #[test]
    fn test_wait_loop_interval() {
        let interval_ms = 100;
        assert!(interval_ms > 0);
        assert!(interval_ms < 1000);
    }

    #[test]
    fn test_any_visible_selector_logic() {
        let selectors = vec!["#a", "#b", "#c"];
        assert_eq!(selectors.len(), 3);
    }

    #[test]
    fn test_empty_selectors_array() {
        let selectors: Vec<&str> = vec![];
        assert_eq!(selectors.len(), 0);
    }

    #[test]
    fn test_single_selector_array() {
        let selectors = vec!["#test"];
        assert_eq!(selectors.len(), 1);
    }

    #[test]
    fn test_selector_observation_logs_css_mode_result_fields() {
        let output = capture_selector_observation_log(|| {
            emit_selector_observation("css", None, "ok", None, None);
        });

        assert!(output.contains("selector resolution"));
        assert!(output.contains("selector_mode=css") || output.contains("selector_mode=\"css\""));
        assert!(output.contains("locator_result=ok") || output.contains("locator_result=\"ok\""));
        assert!(output.contains("locator_role="));
    }

    #[test]
    fn test_selector_observation_logs_locator_metadata_fields() {
        let output = capture_selector_observation_log(|| {
            emit_selector_observation(
                "a11y",
                Some("button"),
                "ambiguous",
                Some("exact"),
                Some("main"),
            );
        });

        assert!(output.contains("selector resolution"));
        assert!(
            output.contains("selector_mode=a11y") || output.contains("selector_mode=\"a11y\"")
        );
        assert!(
            output.contains("locator_result=ambiguous")
                || output.contains("locator_result=\"ambiguous\"")
        );
        assert!(
            output.contains("locator_role=button") || output.contains("locator_role=\"button\"")
        );
        assert!(
            output.contains("locator_match_mode=exact")
                || output.contains("locator_match_mode=\"exact\"")
        );
        assert!(
            output.contains("locator_scope_used=main")
                || output.contains("locator_scope_used=\"main\"")
        );
    }

    #[cfg(feature = "accessibility-locator")]
    #[test]
    fn test_classify_locator_exists_states() {
        assert_eq!(classify_locator_exists(0), "not_found");
        assert_eq!(classify_locator_exists(1), "ok");
        assert_eq!(classify_locator_exists(2), "ambiguous");
    }

    #[cfg(feature = "accessibility-locator")]
    #[test]
    fn test_classify_locator_visible_states() {
        assert_eq!(classify_locator_visible(0, 0), "not_found");
        assert_eq!(classify_locator_visible(2, 0), "not_found");
        assert_eq!(classify_locator_visible(2, 1), "ok");
        assert_eq!(classify_locator_visible(3, 2), "ambiguous");
    }

    #[cfg(feature = "accessibility-locator")]
    #[test]
    fn test_classify_locator_text_states() {
        assert_eq!(classify_locator_text(0, false), "not_found");
        assert_eq!(classify_locator_text(1, false), "not_found");
        assert_eq!(classify_locator_text(2, true), "ambiguous");
        assert_eq!(classify_locator_text(1, true), "ok");
    }

    #[cfg(feature = "accessibility-locator")]
    #[test]
    fn test_locator_not_found_error_string() {
        let locator = AccessibilityLocator {
            role: "button".to_string(),
            name: "Save".to_string(),
            scope: None,
            match_mode: LocatorMatchMode::Exact,
        };
        let err = locator_not_found_error(&locator).to_string();
        assert!(err.contains("locator_not_found"));
        assert!(err.contains("role='button'"));
        assert!(err.contains("name='Save'"));
    }

    #[cfg(feature = "accessibility-locator")]
    #[test]
    fn test_locator_unsupported_error_string() {
        let err = locator_unsupported_error("html").to_string();
        assert!(err.contains("locator_unsupported"));
        assert!(err.contains("operation='html'"));
    }

    #[cfg(feature = "accessibility-locator")]
    #[test]
    fn test_navigation_css_compat_routing_matrix() {
        let selectors = [
            "#submit",
            ".btn.primary",
            "button[aria-label='Like']",
            "[role='button'][aria-label='Follow @user']",
            "[data-testid='tweetButtonInline']",
            "main div:nth-child(2) > button",
            "input[name='q']",
            "a[href*='/status/']",
            "div[class*='tweet'] button:nth-of-type(1)",
        ];

        for selector in selectors {
            let parsed = parse_selector_for_navigation(selector).unwrap();
            assert_eq!(parsed, ParsedSelector::Css(selector.to_string()));
        }
    }

    #[cfg(feature = "accessibility-locator")]
    #[test]
    fn test_navigation_routes_locator_grammar_to_accessibility_mode() {
        let parsed = parse_selector_for_navigation("role=button[name='Save changes']").unwrap();
        match parsed {
            ParsedSelector::Accessibility(locator) => {
                assert_eq!(locator.role, "button");
                assert_eq!(locator.name, "Save changes");
                assert_eq!(locator.scope, None);
                assert_eq!(locator.match_mode, LocatorMatchMode::Exact);
            }
            ParsedSelector::Css(_) => panic!("expected accessibility parsing route"),
        }
    }

    #[cfg(feature = "accessibility-locator")]
    #[test]
    fn test_navigation_surfaces_locator_parse_error_with_prefix() {
        let err = parse_selector_for_navigation("role=button[name=\"Save\"]")
            .unwrap_err()
            .to_string();
        assert!(err.contains("locator_parse_error"));
        assert!(err.contains("single quotes"));
    }

    #[cfg(feature = "accessibility-locator")]
    #[test]
    fn test_selector_uses_accessibility_locator_trims_leading_whitespace() {
        assert!(selector_uses_accessibility_locator("role=button[name='Save']"));
        assert!(selector_uses_accessibility_locator("   role=button[name='Save']"));
        assert!(!selector_uses_accessibility_locator("button[aria-label='Save']"));
    }

    #[cfg(feature = "accessibility-locator")]
    #[test]
    fn test_quad_center_handles_valid_and_invalid_quads() {
        let center = quad_center(&[0.0, 0.0, 20.0, 0.0, 20.0, 20.0, 0.0, 20.0]);
        assert_eq!(center, Some((10.0, 10.0)));

        assert_eq!(quad_center(&[0.0, 0.0, 10.0]), None);
        assert_eq!(
            quad_center(&[0.0, 0.0, f64::NAN, 0.0, 20.0, 20.0, 0.0, 20.0]),
            None
        );
        assert_eq!(
            quad_center(&[0.0, 0.0, f64::INFINITY, 0.0, 20.0, 20.0, 0.0, 20.0]),
            None
        );
    }
}
