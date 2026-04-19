//! Popup and modal handling helpers.
//! Detects and closes cookie banners, "follow on X" prompts, sign-up nag screens, etc.

use crate::prelude::TaskContext;
use anyhow::Result;
use serde_json::Value;

use super::twitteractivity_navigation::is_login_flow;
use super::{twitteractivity_selectors::*, twitteractivity_humanized::*};

/// Checks if any known popup/overlay/modal is present on the page.
/// Returns a description of the popup type or `None` if none detected.
pub async fn detect_popup(api: &TaskContext) -> Result<Option<String>> {
    // Check for overlay/modal
    let js = selector_popup_overlay();
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value();
    if value.is_some() && !value.as_ref().unwrap().is_null() {
        return Ok(Some("overlay".to_string()));
    }

    // Check for "Follow on X" external redirect confirmation
    let js_confirm = selector_follow_confirm_modal();
    let result = api.page().evaluate(js_confirm.to_string()).await?;
    let value = result.value();
    if value.is_some() && !value.as_ref().unwrap().is_null() {
        return Ok(Some("follow_confirm".to_string()));
    }

    // Check if login flow is showing
    if is_login_flow(api).await? {
        return Ok(Some("login_flow".to_string()));
    }

    Ok(None)
}

/// Attempts to close the currently active popup by clicking its close button.
/// Returns true if a popup was found and closed.
pub async fn close_active_popup(api: &TaskContext) -> Result<bool> {
    if let Some(popup_type) = detect_popup(api).await? {
        match popup_type.as_str() {
            "follow_confirm" => {
                // "Follow on X" confirmation: try to find "Cancel" or close button
                let cancel_js = r#"
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

    Ok(false)
}

/// Dismisses cookie banners using known selector patterns.
/// Returns true if a cookie banner was found and dismissed.
pub async fn dismiss_cookie_banner(api: &TaskContext) -> Result<bool> {
    // Try known cookie banner selectors
    let cookie_selectors = [
        "button[aria-label*='Accept']",
        "button[data-testid*='accept']",
        "button:contains('Accept all')",
        "div[role='button']:contains('Accept')",
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

    Ok(false)
}

/// Closes any "sign up to join the conversation" nag screens.
/// Returns true if a signup nag was dismissed.
pub async fn dismiss_signup_nag(api: &TaskContext) -> Result<bool> {
    let js = r#"
        (function() {
            var nag = document.querySelector('div[data-testid="sidebarColumn"]') ||
                      document.querySelector('div[aria-label="Sign up"]');
            if (nag) {
                var closeBtn = nag.querySelector('button[aria-label="Close"]');
                if (closeBtn) {
                    var r = closeBtn.getBoundingClientRect();
                    return { x: r.x + r.width/2, y: r.y + r.height/2 };
                }
            }
            return null;
        })()
    "#;
    let result = api.page().evaluate(js.to_string()).await?;
    let value = result.value();
    if let Some(obj) = value.and_then(|v: &Value| v.as_object()) {
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
    Ok(false)
}


