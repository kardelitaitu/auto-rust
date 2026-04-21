use crate::capabilities::keyboard;
use crate::prelude::TaskContext;
use anyhow::Result;
use chromiumoxide::Page;
use log::{info, warn};
use serde_json::Value;

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    info!("Task started");

    let url = extract_url_from_payload(&payload)?;
    let mut typing = api.behavior_runtime().typing;
    if let Some(override_rate) = extract_typo_rate(&payload) {
        typing.typo_rate_pct = override_rate * 100.0;
    }
    let typo_rate = typing.typo_rate_pct / 100.0;
    info!("Navigating to: {}", url);
    info!("Typo rate: {:.2}", typo_rate);

    api.navigate_to(&url, 30000).await?;

    if let Err(e) = api.wait_for_load(10000).await {
        warn!("Failed to wait for load: {}", e);
    }

    perform_keyboard_demos(api, &typing).await?;

    info!("Task completed");
    Ok(())
}

async fn perform_keyboard_demos(
    api: &TaskContext,
    typing: &crate::internal::profile::TypingBehavior,
) -> Result<()> {
    let page = api.page();
    let clipboard = api.clipboard();
    info!("Looking for textarea...");

    let exists = page
        .evaluate(
            "document.querySelector('textarea, input[type=text], [contenteditable]') !== null",
        )
        .await?
        .value()
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false);

    if !exists {
        info!("No interactive element found, performing keyboard demos on page");
        demo_page_keyboard(api).await?;
        return Ok(());
    }

    info!("Clicking to focus...");
    focus_element(page).await?;
    api.pause(500).await;

    info!("=== Demo 1: Type text ===");
    type_or_typo_text(api, "Hello World!", typing).await?;
    api.pause(1000).await;

    info!("=== Demo 2: Type second line ===");
    api.press("End").await?;
    api.pause(200).await;
    type_or_typo_text(api, "\nSecond line added by demo-keyboard.", typing).await?;
    api.pause(800).await;

    info!("=== Demo 3: Select all (textarea API fallback) ===");
    select_all_text(page).await?;
    api.pause(300).await;

    info!("=== Demo 4: Copy to session clipboard ===");
    let copied = clipboard.copy(page).await?;
    info!("Copied {} chars", copied.chars().count());
    api.pause(300).await;

    info!("=== Demo 5: Ctrl+V (paste multiple times) ===");
    for i in 0..3 {
        info!("Paste {}", i + 1);
        clipboard.paste(page).await?;
        api.pause(250).await;
    }
    api.pause(500).await;

    info!("=== Demo 6: ArrowLeft and Backspace ===");
    api.press("ArrowLeft").await?;
    api.pause(120).await;
    api.press("ArrowLeft").await?;
    api.pause(120).await;
    api.press("Backspace").await?;
    api.pause(300).await;

    info!("=== Demo 7: Ctrl+A then cut to session clipboard ===");
    select_all_text(page).await?;
    api.pause(200).await;
    let cut = clipboard.cut(page).await?;
    info!("Cut {} chars", cut.chars().count());
    api.pause(500).await;

    info!("=== Demo 8: Ctrl+V (paste back multiple times) ===");
    for i in 0..2 {
        info!("Paste back {}", i + 1);
        clipboard.paste(page).await?;
        api.pause(250).await;
    }
    api.pause(800).await;

    info!("=== Demo 9: Arrow keys ===");
    api.press("ArrowRight").await?;
    api.pause(200).await;
    api.press("ArrowRight").await?;
    api.pause(200).await;

    info!("=== Demo 10: Home/End ===");
    api.press("Home").await?;
    api.pause(300).await;
    api.press("End").await?;
    api.pause(300).await;

    info!("=== Demo 11: Shift+Arrow (selection) ===");
    for _ in 0..3 {
        api.press_with_modifiers("ArrowLeft", &["shift"]).await?;
        api.pause(100).await;
    }
    api.pause(500).await;

    info!("=== Demo 12: Ctrl+Z (undo) ===");
    api.press_with_modifiers("z", &["control"]).await?;
    api.pause(500).await;

    info!("=== Demo 13: Ctrl+Y (redo) ===");
    api.press_with_modifiers("y", &["control"]).await?;
    api.pause(500).await;

    info!("=== Demo 14: Type more at end ===");
    api.press("End").await?;
    api.pause(200).await;
    api.type_text(" - Added!").await?;
    api.pause(500).await;

    Ok(())
}

async fn demo_page_keyboard(api: &TaskContext) -> Result<()> {
    info!("Page keyboard demo: pressing various keys");

    let keys = ["Tab", "Enter", "ArrowDown", "ArrowUp", "End", "Home"];

    for key in keys.iter() {
        api.press(key).await?;
        api.pause(200).await;
    }

    info!("Page keyboard demo complete");
    Ok(())
}

async fn focus_element(page: &Page) -> Result<()> {
    page.evaluate(
        r#"
        (function() {
            const el = document.querySelector('textarea, input[type=text], [contenteditable]');
            if (el) {
                el.focus();
                const rect = el.getBoundingClientRect();
                return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
            }
            return null;
        })()
    "#,
    )
    .await?;
    Ok(())
}

fn extract_url_from_payload(payload: &Value) -> Result<String> {
    if let Some(url) = payload.get("url") {
        if let Some(url_str) = url.as_str() {
            return Ok(url_str.to_string());
        }
    }

    if let Some(value) = payload.get("value") {
        if let Some(value_str) = value.as_str() {
            return Ok(value_str.to_string());
        }
    }

    Ok("https://textarea.online/".to_string())
}

async fn select_all_text(page: &Page) -> Result<()> {
    page.evaluate(
        r#"
        (function() {
            const el = document.querySelector('textarea, input[type=text], [contenteditable]');
            if (!el) return;
            if (typeof el.select === 'function') {
                el.select();
                return;
            }
            if (el.isContentEditable) {
                const range = document.createRange();
                range.selectNodeContents(el);
                const sel = window.getSelection();
                sel.removeAllRanges();
                sel.addRange(range);
            }
        })();
        "#,
    )
    .await?;
    Ok(())
}

async fn type_or_typo_text(
    api: &TaskContext,
    text: &str,
    typing: &crate::internal::profile::TypingBehavior,
) -> Result<()> {
    if typing.typo_rate_pct > 0.0 {
        keyboard::natural_typing_profiled(
            api.page(),
            "textarea, input[type=text], [contenteditable]",
            text,
            typing,
        )
        .await
    } else {
        keyboard::type_text_profiled(api.page(), text, typing).await
    }
}

fn extract_typo_rate(payload: &Value) -> Option<f64> {
    payload
        .get("typo_rate")
        .and_then(|v| v.as_f64())
        .map(|v| v.clamp(0.0, 1.0))
}
