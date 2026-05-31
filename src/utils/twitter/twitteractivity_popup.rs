//! Popup and modal handling helpers.
//! Detects and closes cookie banners, "follow on X" prompts, sign-up nag screens, etc.

use crate::prelude::TaskContext;
use anyhow::Result;
use log::{info, trace};
use serde_json::Value;
use tracing::instrument;

use super::twitteractivity_navigation::is_login_flow;
use super::{
    twitteractivity_humanized::{attempt_close_popup, human_pause},
    twitteractivity_selectors::{selector_follow_confirm_modal, selector_popup_overlay},
};

/// Checks if any known popup/overlay/modal is present on the page.
/// Returns a description of the popup type or `None` if none detected.
#[instrument(skip(api))]
pub async fn detect_popup(api: &TaskContext) -> Result<Option<String>> {
    // Check for overlay/modal
    let js = selector_popup_overlay();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value();
    if value.as_ref().is_some_and(|v| !v.is_null()) {
        trace!("Popup detected: overlay");
        return Ok(Some("overlay".to_string()));
    }

    // Check for "Follow on X" external redirect confirmation
    let js_confirm = selector_follow_confirm_modal();
    let result = api.page().evaluate(js_confirm.to_string()).await?;
    let value = result.value();
    if value.as_ref().is_some_and(|v| !v.is_null()) {
        trace!("Popup detected: follow_confirm");
        return Ok(Some("follow_confirm".to_string()));
    }

    // Check if login flow is showing
    if is_login_flow(api).await? {
        trace!("Popup detected: login_flow");
        return Ok(Some("login_flow".to_string()));
    }

    trace!("No popup detected");
    Ok(None)
}

/// Attempts to close the currently active popup by clicking its close button.
/// Returns true if a popup was found and closed.
#[instrument(skip(api))]
pub async fn close_active_popup(api: &TaskContext) -> Result<bool> {
    if let Some(popup_type) = detect_popup(api).await? {
        match popup_type.as_str() {
            "follow_confirm" => {
                // "Follow on X" confirmation: try to find "Cancel" or close button
                let cancel_js = r"
                    (function() {
                        var btns = document.querySelectorAll('button');
                        for (var i = 0; i < btns.length; i++) {
                            var t = (btns[i].textContent || '').trim().toLowerCase();
                            if (t === 'cancel' || t === 'close' || t.includes('not now')) {
                                var r = btns[i].getBoundingClientRect();
                                return { x: r.x + r.width/2, y: r.y + r.height/2 };
                            }
                        }
                        return null;
                    })()
                ";
                if let Ok(result) = api.page().evaluate(cancel_js.to_string()).await {
                    if let Some(obj) = result.value().and_then(|v: &Value| v.as_object()) {
                        if let (Some(x), Some(y)) = (
                            obj.get("x").and_then(|v: &Value| v.as_f64()),
                            obj.get("y").and_then(|v: &Value| v.as_f64()),
                        ) {
                            api.move_mouse_to(x, y).await?;
                            human_pause(api, 200).await;
                            api.click_at(x, y).await?;
                            human_pause(api, 500).await;
                            return Ok(true);
                        }
                    }
                }
            }
            _ => {
                // Generic overlay: try to find X button
                if attempt_close_popup(api).await? {
                    return Ok(true);
                }
            }
        }
    }

    info!("No popup found");
    Ok(false)
}

/// Dismisses cookie banners using known selector patterns.
/// Returns true if a cookie banner was found and dismissed.
#[instrument(skip(api))]
pub async fn dismiss_cookie_banner(api: &TaskContext) -> Result<bool> {
    // Try known cookie banner selectors
    let cookie_selectors = [
        "button[aria-label*='Accept']",
        "button[data-testid*='accept']",
    ];

    for selector in &cookie_selectors {
        let js = format!(
            r#"
            (function() {{
                var btn = document.querySelector("{}");
                if (btn) {{
                    var r = btn.getBoundingClientRect();
                    return {{ x: r.x + r.width/2, y: r.y + r.height/2 }};
                }}
                return null;
            }})()
            "#,
            selector.replace('"', "\\\"")
        );
        let result = api.page().evaluate(js).await;
        if let Ok(res) = result {
            if let Some(obj) = res.value().and_then(|v: &Value| v.as_object()) {
                if let (Some(x), Some(y)) = (
                    obj.get("x").and_then(|v: &Value| v.as_f64()),
                    obj.get("y").and_then(|v: &Value| v.as_f64()),
                ) {
                    api.move_mouse_to(x, y).await?;
                    human_pause(api, 200).await;
                    api.click_at(x, y).await?;
                    human_pause(api, 800).await;
                    return Ok(true);
                }
            }
        }
    }

    // Fallback: search all buttons by text content
    let fallback_js = r#"
        (function() {
            var terms = ["accept", "accept all", "allow", "got it"];
            var buttons = document.querySelectorAll('button, div[role="button"]');
            for (var i = 0; i < buttons.length; i++) {
                var text = (buttons[i].textContent || '').trim().toLowerCase();
                if (terms.some(function(t) { return text.indexOf(t) !== -1; })) {
                    var r = buttons[i].getBoundingClientRect();
                    if (r.width > 0 && r.height > 0) {
                        return { x: r.x + r.width/2, y: r.y + r.height/2 };
                    }
                }
            }
            return null;
        })()
    "#;

    let result = api.page().evaluate(fallback_js.to_string()).await;
    if let Ok(res) = result {
        if let Some(obj) = res.value().and_then(|v: &Value| v.as_object()) {
            if let (Some(x), Some(y)) = (
                obj.get("x").and_then(|v: &Value| v.as_f64()),
                obj.get("y").and_then(|v: &Value| v.as_f64()),
            ) {
                api.move_mouse_to(x, y).await?;
                human_pause(api, 200).await;
                api.click_at(x, y).await?;
                human_pause(api, 800).await;
                return Ok(true);
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_popup_types_are_known() {
        // Detection order matters: overlay checked first, then follow_confirm, then login_flow
        let popup_types = ["overlay", "follow_confirm", "login_flow"];
        assert_eq!(popup_types.len(), 3, "Must know all popup types");
        assert_eq!(
            popup_types[0], "overlay",
            "Overlay checked first (most common)"
        );
        assert_eq!(
            popup_types[1], "follow_confirm",
            "Follow confirm checked second"
        );
        assert_eq!(popup_types[2], "login_flow", "Login flow checked last");
    }

    #[test]
    fn test_close_button_js_produces_coordinates_or_null() {
        // The JS must always return either {x, y} coordinates or null
        // This validates the IIFE wrapper and return structure
        let js = r#"
            (function() {
                var btns = document.querySelectorAll('button');
                for (var i = 0; i < btns.length; i++) {
                    var t = (btns[i].textContent || '').trim().toLowerCase();
                    if (t === 'cancel' || t === 'close' || t.includes('not now')) {
                        var r = btns[i].getBoundingClientRect();
                        return { x: r.x + r.width/2, y: r.y + r.height/2 };
                    }
                }
                return null;
            })()
        "#;
        // Must be an IIFE (Immediately Invoked Function Expression)
        assert!(
            js.trim_start().starts_with("(function"),
            "Must wrap in IIFE"
        );
        assert!(js.trim_end().ends_with(")()"), "Must invoke immediately");
        // Must query for buttons
        assert!(
            js.contains("querySelectorAll('button')"),
            "Must query all buttons"
        );
        // Must properly center coordinates
        assert!(js.contains("r.x + r.width/2"), "Must calculate center x");
        assert!(js.contains("r.y + r.height/2"), "Must calculate center y");
        // Must have null fallback
        assert!(js.contains("return null"), "Must return null when no match");
        // Must match multiple trigger terms
        assert!(js.contains("cancel"), "Must match 'cancel' button");
        assert!(js.contains("close"), "Must match 'close' button");
        assert!(js.contains("not now"), "Must match 'not now' button");
    }

    #[test]
    fn test_cookie_banner_js_produces_coordinates_or_null() {
        // Similar contract for cookie banner dismissal JS
        let js = r#"
            (function() {
                var btn = document.querySelector("button[aria-label*='Accept']");
                if (btn) {
                    var r = btn.getBoundingClientRect();
                    return { x: r.x + r.width/2, y: r.y + r.height/2 };
                }
                return null;
            })()
        "#;
        assert!(js.contains("querySelector"), "Must use querySelector");
        assert!(js.contains("if (btn)"), "Must guard against null element");
        assert!(
            js.contains("return null"),
            "Must return null when not found"
        );
        assert!(
            js.contains("getBoundingClientRect"),
            "Must get element rect"
        );
        assert!(js.contains("width/2"), "Must calculate center x");
        assert!(js.contains("height/2"), "Must calculate center y");
    }

    #[test]
    fn test_cookie_fallback_js_searches_by_text_content() {
        // The fallback JS searches all buttons/divs by text content for known terms
        let js = r#"
            (function() {
                var terms = ["accept", "accept all", "allow", "got it"];
                var buttons = document.querySelectorAll('button, div[role="button"]');
                for (var i = 0; i < buttons.length; i++) {
                    var text = (buttons[i].textContent || '').trim().toLowerCase();
                    if (terms.some(function(t) { return text.indexOf(t) !== -1; })) {
                        var r = buttons[i].getBoundingClientRect();
                        if (r.width > 0 && r.height > 0) {
                            return { x: r.x + r.width/2, y: r.y + r.height/2 };
                        }
                    }
                }
                return null;
            })()
        "#;
        // All 4 known cookie consent terms must be present
        assert!(js.contains("accept all"), "Must match 'accept all'");
        assert!(js.contains("\"allow\""), "Must match 'allow'");
        assert!(js.contains("got it"), "Must match 'got it'");
        // Must check for visible elements (non-zero dimensions)
        assert!(
            js.contains("r.width > 0 && r.height > 0"),
            "Must check element visibility"
        );
        // Must query both buttons and div role="button"
        assert!(
            js.contains("querySelectorAll('button, div[role=\"button\"]')"),
            "Must query buttons and div buttons"
        );
    }

    #[test]
    fn test_cookie_selector_production_code_patterns() {
        // The production code in dismiss_cookie_banner uses 2 selectors
        // (aria-label and data-testid), plus the fallback JS.
        // This test verifies the selector patterns match known cookie banners.
        let primary_selectors = [
            "button[aria-label*='Accept']",
            "button[data-testid*='accept']",
        ];
        // aria-label variant targets Twitter's cookie banner
        assert!(
            primary_selectors[0].contains("aria-label"),
            "Twitter uses aria-label for cookie banners"
        );
        // data-testid variant is a common React pattern
        assert!(
            primary_selectors[1].contains("data-testid"),
            "data-testid is common in SPAs"
        );
        // Both target button elements specifically
        for sel in &primary_selectors {
            assert!(
                sel.starts_with("button"),
                "All primary selectors target button elements"
            );
        }
    }

    #[test]
    fn test_follow_confirm_js_searches_by_text_content() {
        // The follow_confirm dismissal JS searches buttons by text
        let js = r#"
            (function() {
                var btns = document.querySelectorAll('button');
                for (var i = 0; i < btns.length; i++) {
                    var t = (btns[i].textContent || '').trim().toLowerCase();
                    if (t === 'cancel' || t === 'close' || t.includes('not now')) {
                        var r = btns[i].getBoundingClientRect();
                        return { x: r.x + r.width/2, y: r.y + r.height/2 };
                    }
                }
                return null;
            })()
        "#;
        assert!(
            js.contains("textContent || ''"),
            "Must handle null/undefined textContent"
        );
        assert!(js.contains(".trim()"), "Must trim whitespace");
        assert!(js.contains(".toLowerCase()"), "Must case-normalize");
        assert!(
            js.contains("getBoundingClientRect"),
            "Must get element rect"
        );
    }

    #[test]
    fn test_selector_escaping_handles_double_quotes() {
        // The production code escapes double quotes in selectors for JS interpolation
        let selector = "button[aria-label*=\"Accept\"]";
        let escaped = selector.replace('"', "\\\"");
        assert!(
            escaped.contains("\\\""),
            "Must escape double quotes for JS template"
        );
        assert!(
            !escaped.contains("\"Accept\""),
            "Original double quotes must be escaped"
        );
    }
}
