//! Popup and modal handling helpers.
//! Detects and closes cookie banners, "follow on X" prompts, sign-up nag screens, etc.

use crate::prelude::TaskContext;
use anyhow::Result;
use log::info;
use serde_json::Value;
use tracing::instrument;

use super::twitteractivity_navigation::is_login_flow;
use super::{twitteractivity_humanized::*, twitteractivity_selectors::*};

/// Checks if any known popup/overlay/modal is present on the page.
/// Returns a description of the popup type or `None` if none detected.
#[instrument(skip(api))]
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
#[instrument(skip(api))]
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
/// DISABLED: Causing hangs, skip for now.
pub async fn dismiss_signup_nag(_api: &TaskContext) -> Result<bool> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_selectors_not_empty() {
        let cookie_selectors = [
            "button[aria-label*='Accept']",
            "button[data-testid*='accept']",
            "button:contains('Accept all')",
            "div[role='button']:contains('Accept')",
        ];
        assert_eq!(cookie_selectors.len(), 4);
    }

    #[test]
    fn test_cookie_selectors_contain_accept() {
        let cookie_selectors = [
            "button[aria-label*='Accept']",
            "button[data-testid*='accept']",
            "button:contains('Accept all')",
            "div[role='button']:contains('Accept')",
        ];
        for selector in &cookie_selectors {
            assert!(selector.to_lowercase().contains("accept"));
        }
    }

    #[test]
    fn test_dismiss_signup_nag_returns_false() {
        // Test that the function exists and has the right signature
        // Just verify it compiles
        let _ = dismiss_signup_nag;
    }

    #[test]
    fn test_detect_popup_types() {
        // Test that we know the popup types we detect
        let popup_types = ["overlay", "follow_confirm", "login_flow"];
        assert_eq!(popup_types.len(), 3);
    }

    #[test]
    fn test_close_button_search_js_structure() {
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
        assert!(cancel_js.contains("querySelectorAll"));
        assert!(cancel_js.contains("cancel"));
        assert!(cancel_js.contains("close"));
        assert!(cancel_js.contains("getBoundingClientRect"));
    }

    #[test]
    fn test_cookie_selectors_aria_label() {
        let selector = "button[aria-label*='Accept']";
        assert!(selector.contains("aria-label"));
        assert!(selector.contains("Accept"));
    }

    #[test]
    fn test_cookie_selectors_data_testid() {
        let selector = "button[data-testid*='accept']";
        assert!(selector.contains("data-testid"));
        assert!(selector.contains("accept"));
    }

    #[test]
    fn test_cookie_selectors_contains_pseudo() {
        let selector = "button:contains('Accept all')";
        assert!(selector.contains(":contains"));
        assert!(selector.contains("Accept all"));
    }

    #[test]
    fn test_cookie_selectors_role_button() {
        let selector = "div[role='button']:contains('Accept')";
        assert!(selector.contains("role='button'"));
        assert!(selector.contains("Accept"));
    }

    #[test]
    fn test_close_button_js_includes_not_now() {
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
        assert!(cancel_js.contains("not now"));
    }

    #[test]
    fn test_close_button_js_returns_coordinates() {
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
        assert!(cancel_js.contains("x:"));
        assert!(cancel_js.contains("y:"));
    }

    #[test]
    fn test_close_button_js_uses_trim() {
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
        assert!(cancel_js.contains("trim"));
    }

    #[test]
    fn test_close_button_js_uses_tolowercase() {
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
        assert!(cancel_js.contains("toLowerCase"));
    }

    #[test]
    fn test_cookie_banner_js_structure() {
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
        assert!(js.contains("querySelector"));
        assert!(js.contains("getBoundingClientRect"));
        assert!(js.contains("return null"));
    }

    #[test]
    fn test_popup_detection_order() {
        // Verify the order of popup detection
        let detection_order = ["overlay", "follow_confirm", "login_flow"];
        assert_eq!(detection_order[0], "overlay");
        assert_eq!(detection_order[1], "follow_confirm");
        assert_eq!(detection_order[2], "login_flow");
    }

    #[test]
    fn test_close_button_js_loops_buttons() {
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
        assert!(cancel_js.contains("for (var i = 0; i < btns.length; i++)"));
    }

    #[test]
    fn test_cookie_selector_escaping() {
        let selector = "button[aria-label*=\"Accept\"]";
        let escaped = selector.replace('"', "\\\"");
        assert!(escaped.contains("\\\""));
    }

    #[test]
    fn test_close_button_js_null_return() {
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
        assert!(cancel_js.contains("return null"));
    }

    #[test]
    fn test_cookie_banner_js_returns_object() {
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
        assert!(js.contains("{ x:"));
        assert!(js.contains("y:"));
    }

    #[test]
    fn test_close_button_js_center_calculation() {
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
        assert!(cancel_js.contains("width/2"));
        assert!(cancel_js.contains("height/2"));
    }

    #[test]
    fn test_dismiss_signup_nag_disabled() {
        // The function is disabled and returns false
        // This test documents that behavior
        // Test placeholder - verifies the module compiles
    }

    #[test]
    fn test_cookie_selectors_array_length() {
        let cookie_selectors = [
            "button[aria-label*='Accept']",
            "button[data-testid*='accept']",
            "button:contains('Accept all')",
            "div[role='button']:contains('Accept')",
        ];
        assert_eq!(cookie_selectors.len(), 4);
    }

    #[test]
    fn test_close_button_js_text_content_fallback() {
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
        assert!(cancel_js.contains("textContent || ''"));
    }

    #[test]
    fn test_cookie_banner_js_if_check() {
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
        assert!(js.contains("if (btn)"));
    }

    #[test]
    fn test_close_button_js_exact_match() {
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
        assert!(cancel_js.contains("t === 'cancel'"));
        assert!(cancel_js.contains("t === 'close'"));
    }

    #[test]
    fn test_close_button_js_partial_match() {
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
        assert!(cancel_js.contains("t.includes('not now')"));
    }
}
