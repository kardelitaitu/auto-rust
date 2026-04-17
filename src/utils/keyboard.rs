use anyhow::Result;
use chromiumoxide::Page;
use crate::utils::math::{gaussian, random_in_range};
use crate::utils::timing::human_pause;

#[derive(Debug, Clone)]
pub struct PressOptions {
    pub modifiers: Vec<String>,
    pub delay: u64,
    pub repeat: u32,
    pub down_and_up: bool,
}

impl Default for PressOptions {
    fn default() -> Self {
        Self {
            modifiers: vec![],
            delay: 0,
            repeat: 1,
            down_and_up: true,
        }
    }
}

fn normalize_modifier(modifier: &str) -> String {
    match modifier.to_lowercase().as_str() {
        "ctrl" | "control" => "Control".to_string(),
        "shift" => "Shift".to_string(),
        "alt" => "Alt".to_string(),
        "meta" | "cmd" | "command" | "win" => "Meta".to_string(),
        _ => modifier.to_string(),
    }
}

fn is_modifier(key: &str) -> bool {
    matches!(key, "Control" | "Shift" | "Alt" | "Meta")
}

pub async fn press(page: &Page, key: &str) -> Result<()> {
    press_with_options(page, key, &PressOptions::default()).await
}

pub async fn press_with_options(page: &Page, key: &str, options: &PressOptions) -> Result<()> {
    let keys: Vec<&str> = if key.contains('+') {
        key.split('+').collect()
    } else {
        vec![key]
    };

    let normalized_modifiers: Vec<String> = options
        .modifiers
        .iter()
        .map(|m| normalize_modifier(m))
        .collect();

    for _ in 0..options.repeat {
        for mod_key in &normalized_modifiers {
            dispatch_key_event(page, "keydown", mod_key).await?;
        }

        for (i, k) in keys.iter().enumerate() {
            if is_modifier(k) {
                continue;
            }

            if options.down_and_up {
                dispatch_key_event(page, "keydown", k).await?;
                human_pause(gaussian(35.0, 15.0, 20.0, 50.0) as u64, 20).await;
                dispatch_key_event(page, "keyup", k).await?;
            } else {
                dispatch_key_event(page, "keypress", k).await?;
            }

            if i < keys.len() - 1 && options.delay > 0 {
                human_pause(options.delay, 20).await;
            }
        }

        for mod_key in &normalized_modifiers {
            dispatch_key_event(page, "keyup", mod_key).await?;
        }

        if options.repeat > 1 {
            human_pause(gaussian(100.0, 50.0, 50.0, 150.0) as u64, 20).await;
        }
    }

    Ok(())
}

pub async fn press_with_modifiers(page: &Page, key: &str, modifiers: &[&str]) -> Result<()> {
    let mod_strings: Vec<String> = modifiers.iter().map(|s| s.to_string()).collect();
    let options = PressOptions {
        modifiers: mod_strings,
        ..Default::default()
    };
    press_with_options(page, key, &options).await
}

pub async fn type_text(page: &Page, text: &str) -> Result<()> {
    type_text_with_options(page, text, 100).await
}

pub async fn type_text_with_options(page: &Page, text: &str, base_delay_ms: u64) -> Result<()> {
    for ch in text.chars() {
        dispatch_input_event(page, ch).await?;

        let char_delay = gaussian(
            base_delay_ms as f64,
            base_delay_ms as f64 * 0.3,
            30.0,
            base_delay_ms as f64 * 3.0,
        ) as u64;

        if ".!?,;:".contains(ch) {
            human_pause(char_delay + random_in_range(100, 300), 20).await;
        } else {
            human_pause(char_delay, 20).await;
        }
    }
    Ok(())
}

pub async fn hold(page: &Page, key: &str, duration_ms: u64) -> Result<()> {
    dispatch_key_event(page, "keydown", key).await?;
    human_pause(duration_ms, 10).await;
    dispatch_key_event(page, "keyup", key).await?;
    Ok(())
}

pub async fn release_all(page: &Page) -> Result<()> {
    dispatch_key_event(page, "keyup", "Shift").await.ok();
    dispatch_key_event(page, "keyup", "Control").await.ok();
    dispatch_key_event(page, "keyup", "Alt").await.ok();
    dispatch_key_event(page, "keyup", "Meta").await.ok();
    Ok(())
}

pub async fn natural_typing(page: &Page, selector: &str, text: &str, typo_rate: f64) -> Result<()> {
    let selector_json = serde_json::to_string(selector)?;
    page.evaluate(format!(
        "(function() {{
            const el = document.querySelector({selector_json});
            if (el) el.focus();
        }})();"
    ))
    .await?;

    for (i, ch) in text.chars().enumerate() {
        if (random_in_range(0, 100) as f64 / 100.0) < typo_rate && i > 0 {
            typo_correction(page, ch).await?;
        } else {
            type_character(page, ch).await?;
        }
    }

    Ok(())
}

async fn dispatch_key_event(page: &Page, event_type: &str, key: &str) -> Result<()> {
    let key_json = serde_json::to_string(key)?;
    let js = format!(
        "(function() {{
            const event = new KeyboardEvent('{event_type}', {{
                key: {key_json},
                code: {key_json},
                bubbles: true,
                cancelable: true
            }});
            const el = document.activeElement || document.body;
            el.dispatchEvent(event);
        }})();"
    );

    page.evaluate(js).await?;
    Ok(())
}

async fn dispatch_input_event(page: &Page, ch: char) -> Result<()> {
    let text_json = serde_json::to_string(&ch.to_string())?;
    let js = format!(
        "(function() {{
            const el = document.activeElement;
            if (!el || el.tagName === 'IFRAME') return;

            const text = {text_json};
            if (el.isContentEditable) {{
                document.execCommand('insertText', false, text);
            }} else if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                const start = el.selectionStart ?? el.value.length;
                el.value = el.value.slice(0, start) + text + el.value.slice(start);
                el.dispatchEvent(new InputEvent('input', {{ bubbles: true, data: text }}));
            }}
        }})();"
    );

    page.evaluate(js).await?;
    Ok(())
}

async fn type_character(page: &Page, ch: char) -> Result<()> {
    let key_delay = gaussian(120.0, 40.0, 50.0, 300.0) as u64;
    dispatch_input_event(page, ch).await?;
    human_pause(key_delay, 30).await;
    Ok(())
}

async fn typo_correction(page: &Page, correct_char: char) -> Result<()> {
    let wrong_char = get_similar_char(correct_char);
    type_character(page, wrong_char).await?;

    human_pause(300, 50).await;
    press(page, "Backspace").await?;
    human_pause(200, 50).await;
    type_character(page, correct_char).await?;
    Ok(())
}

#[allow(dead_code)]
fn get_similar_char(ch: char) -> char {
    match ch.to_ascii_lowercase() {
        'a' => 's',
        's' => 'a',
        'd' => 'f',
        'f' => 'd',
        'e' => 'r',
        'r' => 'e',
        'w' => 'q',
        'q' => 'w',
        't' => 'y',
        'y' => 't',
        'o' => 'p',
        'p' => 'o',
        'i' => 'o',
        'n' => 'm',
        'm' => 'n',
        _ => ch,
    }
}
