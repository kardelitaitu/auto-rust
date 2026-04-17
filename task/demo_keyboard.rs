use anyhow::Result;
use chromiumoxide::Page;
use serde_json::Value;
use log::{info, warn};
use crate::utils::keyboard;
use crate::utils::timing::human_pause;
use crate::utils::clipboard::ClipboardState;

pub async fn run(session_id: &str, page: &Page, payload: Value) -> Result<()> {
    info!("[{session_id}][demo-keyboard] Task started");

    let url = extract_url_from_payload(&payload)?;
    let typo_rate = extract_typo_rate(&payload);
    info!("[{session_id}][demo-keyboard] Navigating to: {url}");
    info!("[{session_id}][demo-keyboard] Typo rate: {:.2}", typo_rate);

    crate::utils::navigation::goto(page, &url, 30000).await?;

    if let Err(e) = crate::utils::navigation::wait_for_load(page, 10000).await {
        warn!("[{session_id}][demo-keyboard] Failed to wait for load: {e}");
    }

    let clipboard = ClipboardState::new(session_id);
    perform_keyboard_demos(&clipboard, page, typo_rate).await?;

    info!("[{session_id}][demo-keyboard] Task completed");
    Ok(())
}

async fn perform_keyboard_demos(clipboard: &ClipboardState, page: &Page, typo_rate: f64) -> Result<()> {
    info!("Looking for textarea...");

    let exists = page
        .evaluate("document.querySelector('textarea, input[type=text], [contenteditable]') !== null")
        .await?
        .value()
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false);

    if !exists {
        info!("No interactive element found, performing keyboard demos on page");
        demo_page_keyboard(page).await?;
        return Ok(())
    }

    info!("Clicking to focus...");
    focus_element(page).await?;
    human_pause(500, 20).await;

    info!("=== Demo 1: Type text ===");
    type_or_typo_text(page, "Hello World!", typo_rate).await?;
    human_pause(1000, 20).await;

    info!("=== Demo 2: Type second line ===");
    keyboard::press(page, "End").await?;
    human_pause(200, 20).await;
    type_or_typo_text(page, "\nSecond line added by demo-keyboard.", typo_rate).await?;
    human_pause(800, 20).await;

    info!("=== Demo 3: Select all (textarea API fallback) ===");
    select_all_text(page).await?;
    human_pause(300, 20).await;

    info!("=== Demo 4: Copy to session clipboard ===");
    let copied = clipboard.copy(page).await?;
    info!("Copied {} chars", copied.chars().count());
    human_pause(300, 20).await;

    info!("=== Demo 5: Ctrl+V (paste multiple times) ===");
    for i in 0..3 {
        info!("Paste {}", i + 1);
        clipboard.paste(page).await?;
        human_pause(250, 20).await;
    }
    human_pause(500, 20).await;

    info!("=== Demo 6: ArrowLeft and Backspace ===");
    keyboard::press(page, "ArrowLeft").await?;
    human_pause(120, 20).await;
    keyboard::press(page, "ArrowLeft").await?;
    human_pause(120, 20).await;
    keyboard::press(page, "Backspace").await?;
    human_pause(300, 20).await;

    info!("=== Demo 7: Ctrl+A then cut to session clipboard ===");
    select_all_text(page).await?;
    human_pause(200, 20).await;
    let cut = clipboard.cut(page).await?;
    info!("Cut {} chars", cut.chars().count());
    human_pause(500, 20).await;

    info!("=== Demo 8: Ctrl+V (paste back multiple times) ===");
    for i in 0..2 {
        info!("Paste back {}", i + 1);
        clipboard.paste(page).await?;
        human_pause(250, 20).await;
    }
    human_pause(800, 20).await;

    info!("=== Demo 9: Arrow keys ===");
    keyboard::press(page, "ArrowRight").await?;
    human_pause(200, 20).await;
    keyboard::press(page, "ArrowRight").await?;
    human_pause(200, 20).await;

    info!("=== Demo 10: Home/End ===");
    keyboard::press(page, "Home").await?;
    human_pause(300, 20).await;
    keyboard::press(page, "End").await?;
    human_pause(300, 20).await;

    info!("=== Demo 11: Shift+Arrow (selection) ===");
    for _ in 0..3 {
        keyboard::press_with_modifiers(page, "ArrowLeft", &["shift"]).await?;
        human_pause(100, 20).await;
    }
    human_pause(500, 20).await;

    info!("=== Demo 12: Ctrl+Z (undo) ===");
    keyboard::press_with_modifiers(page, "z", &["control"]).await?;
    human_pause(500, 20).await;

    info!("=== Demo 13: Ctrl+Y (redo) ===");
    keyboard::press_with_modifiers(page, "y", &["control"]).await?;
    human_pause(500, 20).await;

    info!("=== Demo 14: Type more at end ===");
    keyboard::press(page, "End").await?;
    human_pause(200, 20).await;
    keyboard::type_text(page, " - Added!").await?;
    human_pause(500, 20).await;

    Ok(())
}

async fn demo_page_keyboard(page: &Page) -> Result<()> {
    info!("Page keyboard demo: pressing various keys");

    let keys = ["Tab", "Enter", "ArrowDown", "ArrowUp", "End", "Home"];

    for key in keys.iter() {
        keyboard::press(page, key).await?;
        human_pause(200, 30).await;
    }

    info!("Page keyboard demo complete");
    Ok(())
}

async fn focus_element(page: &Page) -> Result<()> {
    page.evaluate(r#"
        (function() {
            const el = document.querySelector('textarea, input[type=text], [contenteditable]');
            if (el) {
                el.focus();
                const rect = el.getBoundingClientRect();
                return { x: rect.x + rect.width/2, y: rect.y + rect.height/2 };
            }
            return null;
        })()
    "#).await?;
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
        "#
    ).await?;
    Ok(())
}

async fn type_or_typo_text(page: &Page, text: &str, typo_rate: f64) -> Result<()> {
    if typo_rate > 0.0 {
        keyboard::natural_typing(page, "textarea, input[type=text], [contenteditable]", text, typo_rate).await
    } else {
        keyboard::type_text(page, text).await
    }
}

fn extract_typo_rate(payload: &Value) -> f64 {
    payload
        .get("typo_rate")
        .and_then(|v| v.as_f64())
        .map(|v| v.clamp(0.0, 1.0))
        .unwrap_or(0.12)
}
