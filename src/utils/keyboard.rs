//! Keyboard input simulation with human-like typing patterns.
//!
//! Provides utilities for simulating realistic keyboard input:
//! - Human-like typing with variable delay between keystrokes
//! - Special key combinations (Ctrl+C, Ctrl+V, etc.)
//! - Text input with optional typos and corrections
//! - Support for accessibility features and screen readers

use crate::internal::profile::TypingBehavior;
use crate::utils::math::{gaussian, random_in_range};
use crate::utils::timing::human_pause;
use anyhow::Result;
use chromiumoxide::Page;

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

pub async fn type_text_profiled(page: &Page, text: &str, behavior: &TypingBehavior) -> Result<()> {
    type_text_with_options(page, text, behavior.keystroke_mean_ms).await
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
    let behavior = TypingBehavior {
        keystroke_mean_ms: 120,
        keystroke_stddev_ms: 40,
        word_pause_ms: 500,
        typo_rate_pct: typo_rate * 100.0,
        typo_notice_delay_ms: 300,
        typo_retry_delay_ms: 200,
        typo_recovery_chance_pct: 80.0,
    };
    natural_typing_profiled(page, selector, text, &behavior).await
}

pub async fn natural_typing_profiled(
    page: &Page,
    selector: &str,
    text: &str,
    behavior: &TypingBehavior,
) -> Result<()> {
    let selector_json = serde_json::to_string(selector)?;
    page.evaluate(format!(
        "(function() {{
            const el = document.querySelector({selector_json});
            if (el) el.focus();
        }})();"
    ))
    .await?;

    for (i, ch) in text.chars().enumerate() {
        let typo_rate = behavior.typo_rate_pct.clamp(0.0, 100.0) / 100.0;
        if (random_in_range(0, 100) as f64 / 100.0) < typo_rate && i > 0 {
            typo_correction_profiled(page, ch, behavior).await?;
        } else {
            type_character_profiled(page, ch, behavior).await?;
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

            // Phase2: Check readonly/disabled before attempting to type
            if (el.readOnly || el.disabled) {{
                console.warn('dispatch_input_event: element is readonly or disabled');
                return;
            }}

            const text = {text_json};
            if (el.isContentEditable) {{
                document.execCommand('insertText', false, text);
            }} else if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                const hasSelectionApi =
                    typeof el.selectionStart === 'number' &&
                    typeof el.selectionEnd === 'number' &&
                    typeof el.setSelectionRange === 'function';
                if (hasSelectionApi) {{
                    const start = el.selectionStart ?? el.value.length;
                    const end = el.selectionEnd ?? start;
                    el.value = el.value.slice(0, start) + text + el.value.slice(end);
                    const next = start + text.length;
                    try {{
                        el.setSelectionRange(next, next);
                    }} catch (_) {{
                        // Some input types (for example email) do not support selection ranges.
                    }}
                }} else {{
                    el.value = (el.value || '') + text;
                }}
                const opts = {{ bubbles: true, data: text }};
                el.dispatchEvent(new InputEvent('input', opts));
            }}
        }})();"
    );

    page.evaluate(js).await?;
    Ok(())
}

#[allow(dead_code)]
async fn type_character(page: &Page, ch: char) -> Result<()> {
    let key_delay = gaussian(120.0, 40.0, 50.0, 300.0) as u64;
    dispatch_input_event(page, ch).await?;
    human_pause(key_delay, 30).await;
    Ok(())
}

async fn type_character_profiled(page: &Page, ch: char, behavior: &TypingBehavior) -> Result<()> {
    let key_delay = gaussian(
        behavior.keystroke_mean_ms as f64,
        behavior.keystroke_stddev_ms.max(1) as f64,
        50.0,
        500.0,
    ) as u64;
    dispatch_input_event(page, ch).await?;
    human_pause(key_delay, 30).await;
    Ok(())
}

#[allow(dead_code)]
async fn typo_correction(page: &Page, correct_char: char) -> Result<()> {
    let wrong_char = get_similar_char(correct_char);
    type_character(page, wrong_char).await?;

    human_pause(300, 50).await;
    press(page, "Backspace").await?;
    human_pause(200, 50).await;
    type_character(page, correct_char).await?;
    Ok(())
}

async fn typo_correction_profiled(
    page: &Page,
    correct_char: char,
    behavior: &TypingBehavior,
) -> Result<()> {
    let wrong_char = get_similar_char(correct_char);
    type_character_profiled(page, wrong_char, behavior).await?;

    human_pause(behavior.typo_notice_delay_ms, 50).await;
    press(page, "Backspace").await?;
    human_pause(behavior.typo_retry_delay_ms, 50).await;
    type_character_profiled(page, correct_char, behavior).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_press_options_defaults() {
        let options = PressOptions::default();
        assert!(options.modifiers.is_empty());
        assert_eq!(options.delay, 0);
        assert_eq!(options.repeat, 1);
        assert!(options.down_and_up);
    }

    #[test]
    fn test_normalize_modifier_ctrl() {
        assert_eq!(normalize_modifier("ctrl"), "Control");
        assert_eq!(normalize_modifier("control"), "Control");
        assert_eq!(normalize_modifier("CTRL"), "Control");
    }

    #[test]
    fn test_normalize_modifier_shift() {
        assert_eq!(normalize_modifier("shift"), "Shift");
    }

    #[test]
    fn test_normalize_modifier_alt() {
        assert_eq!(normalize_modifier("alt"), "Alt");
    }

    #[test]
    fn test_normalize_modifier_meta() {
        assert_eq!(normalize_modifier("meta"), "Meta");
        assert_eq!(normalize_modifier("cmd"), "Meta");
        assert_eq!(normalize_modifier("command"), "Meta");
        assert_eq!(normalize_modifier("win"), "Meta");
    }

    #[test]
    fn test_is_modifier() {
        assert!(is_modifier("Control"));
        assert!(is_modifier("Shift"));
        assert!(is_modifier("Alt"));
        assert!(is_modifier("Meta"));
        assert!(!is_modifier("a"));
        assert!(!is_modifier("Enter"));
    }

    #[test]
    fn test_get_similar_char_row() {
        assert_eq!(get_similar_char('a'), 's');
        assert_eq!(get_similar_char('s'), 'a');
        assert_eq!(get_similar_char('d'), 'f');
    }

    #[test]
    fn test_get_similar_char_middle() {
        assert_eq!(get_similar_char('e'), 'r');
        assert_eq!(get_similar_char('r'), 'e');
        assert_eq!(get_similar_char('w'), 'q');
        assert_eq!(get_similar_char('q'), 'w');
    }

    #[test]
    fn test_get_similar_char_home() {
        assert_eq!(get_similar_char('t'), 'y');
        assert_eq!(get_similar_char('y'), 't');
        assert_eq!(get_similar_char('o'), 'p');
        assert_eq!(get_similar_char('p'), 'o');
    }

    #[test]
    fn test_get_similar_char_no_match() {
        assert_eq!(get_similar_char('z'), 'z');
        assert_eq!(get_similar_char('1'), '1');
    }

    #[test]
    fn test_press_options_custom() {
        let options = PressOptions {
            modifiers: vec!["Control".to_string(), "Shift".to_string()],
            delay: 100,
            repeat: 3,
            down_and_up: false,
        };
        assert_eq!(options.modifiers.len(), 2);
        assert_eq!(options.delay, 100);
        assert_eq!(options.repeat, 3);
        assert!(!options.down_and_up);
    }

    #[test]
    fn test_normalize_modifier_case_insensitive() {
        assert_eq!(normalize_modifier("CTRL"), "Control");
        assert_eq!(normalize_modifier("ShIfT"), "Shift");
        assert_eq!(normalize_modifier("ALT"), "Alt");
    }

    #[test]
    fn test_normalize_modifier_unknown() {
        assert_eq!(normalize_modifier("unknown"), "unknown");
        assert_eq!(normalize_modifier("F1"), "F1");
    }

    #[test]
    fn test_is_modifier_case_sensitive() {
        assert!(is_modifier("Control"));
        assert!(!is_modifier("control"));
        assert!(is_modifier("Shift"));
        assert!(!is_modifier("shift"));
    }

    #[test]
    fn test_get_similar_char_case_preservation() {
        // Lowercase input
        assert_eq!(get_similar_char('a'), 's');
        // Uppercase input should still return lowercase (since it uses to_ascii_lowercase)
        assert_eq!(get_similar_char('A'), 's');
    }

    #[test]
    fn test_get_similar_char_bottom_row() {
        assert_eq!(get_similar_char('n'), 'm');
        assert_eq!(get_similar_char('m'), 'n');
    }

    #[test]
    fn test_get_similar_char_special_chars() {
        assert_eq!(get_similar_char('@'), '@');
        assert_eq!(get_similar_char('#'), '#');
        assert_eq!(get_similar_char(' '), ' ');
    }

    #[test]
    fn test_get_similar_char_numbers() {
        assert_eq!(get_similar_char('0'), '0');
        assert_eq!(get_similar_char('9'), '9');
    }

    #[test]
    fn test_get_similar_char_punctuation() {
        assert_eq!(get_similar_char('.'), '.');
        assert_eq!(get_similar_char(','), ',');
        assert_eq!(get_similar_char('!'), '!');
    }

    #[test]
    fn test_press_options_clone() {
        let options = PressOptions {
            modifiers: vec!["Control".to_string()],
            delay: 50,
            repeat: 2,
            down_and_up: true,
        };
        let cloned = options.clone();
        assert_eq!(cloned.modifiers, options.modifiers);
        assert_eq!(cloned.delay, options.delay);
        assert_eq!(cloned.repeat, options.repeat);
        assert_eq!(cloned.down_and_up, options.down_and_up);
    }

    #[test]
    fn test_press_options_debug() {
        let options = PressOptions::default();
        let debug_str = format!("{:?}", options);
        assert!(debug_str.contains("PressOptions"));
    }

    #[test]
    fn test_normalize_modifier_variations() {
        assert_eq!(normalize_modifier("CtRl"), "Control");
        assert_eq!(normalize_modifier("SHIFT"), "Shift");
        assert_eq!(normalize_modifier("aLt"), "Alt");
        assert_eq!(normalize_modifier("MeTa"), "Meta");
    }

    #[test]
    fn test_normalize_modifier_windows_variants() {
        assert_eq!(normalize_modifier("Windows"), "Windows");
        assert_eq!(normalize_modifier("Meta"), "Meta");
        assert_eq!(normalize_modifier("Command"), "Meta");
    }

    #[test]
    fn test_is_modifier_all_standard() {
        assert!(is_modifier("Control"));
        assert!(is_modifier("Alt"));
        assert!(is_modifier("Meta"));
    }

    #[test]
    fn test_is_modifier_function_keys() {
        assert!(!is_modifier("F1"));
        assert!(!is_modifier("F12"));
        assert!(!is_modifier("Escape"));
    }

    #[test]
    fn test_is_modifier_navigation_keys() {
        assert!(!is_modifier("ArrowUp"));
        assert!(!is_modifier("ArrowDown"));
        assert!(!is_modifier("Home"));
        assert!(!is_modifier("End"));
    }

    #[test]
    fn test_get_similar_char_i() {
        assert_eq!(get_similar_char('i'), 'o');
    }

    #[test]
    fn test_get_similar_char_unicode() {
        assert_eq!(get_similar_char('é'), 'é');
        assert_eq!(get_similar_char('ñ'), 'ñ');
    }

    #[test]
    fn test_get_similar_char_empty() {
        assert_eq!(get_similar_char('\0'), '\0');
    }

    #[test]
    fn test_press_options_repeat_zero() {
        let options = PressOptions {
            repeat: 0,
            ..Default::default()
        };
        assert_eq!(options.repeat, 0);
    }

    #[test]
    fn test_press_options_high_repeat() {
        let options = PressOptions {
            repeat: 100,
            ..Default::default()
        };
        assert_eq!(options.repeat, 100);
    }

    #[test]
    fn test_press_options_multiple_modifiers() {
        let options = PressOptions {
            modifiers: vec![
                "Control".to_string(),
                "Shift".to_string(),
                "Alt".to_string(),
            ],
            ..Default::default()
        };
        assert_eq!(options.modifiers.len(), 3);
    }

    #[test]
    fn test_press_options_no_modifiers() {
        let options = PressOptions {
            modifiers: vec![],
            ..Default::default()
        };
        assert!(options.modifiers.is_empty());
    }

    #[test]
    fn test_press_options_delay_variations() {
        let options1 = PressOptions {
            delay: 0,
            ..Default::default()
        };
        let options2 = PressOptions {
            delay: 1000,
            ..Default::default()
        };
        assert_eq!(options1.delay, 0);
        assert_eq!(options2.delay, 1000);
    }

    #[test]
    fn test_normalize_modifier_mixed_case() {
        assert_eq!(normalize_modifier("cOnTrOl"), "Control");
        assert_eq!(normalize_modifier("sHiFt"), "Shift");
    }

    #[test]
    fn test_get_similar_char_all_mapped() {
        // Test all mapped characters
        assert_eq!(get_similar_char('a'), 's');
        assert_eq!(get_similar_char('s'), 'a');
        assert_eq!(get_similar_char('d'), 'f');
        assert_eq!(get_similar_char('f'), 'd');
        assert_eq!(get_similar_char('e'), 'r');
        assert_eq!(get_similar_char('r'), 'e');
        assert_eq!(get_similar_char('w'), 'q');
        assert_eq!(get_similar_char('q'), 'w');
        assert_eq!(get_similar_char('t'), 'y');
        assert_eq!(get_similar_char('y'), 't');
        assert_eq!(get_similar_char('o'), 'p');
        assert_eq!(get_similar_char('p'), 'o');
        assert_eq!(get_similar_char('i'), 'o');
        assert_eq!(get_similar_char('n'), 'm');
        assert_eq!(get_similar_char('m'), 'n');
    }
}
