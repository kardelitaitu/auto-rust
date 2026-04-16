use chromiumoxide::Page;
use anyhow::Result;
use crate::utils::math::{random_in_range, gaussian};
use crate::utils::timing::human_pause;

#[allow(dead_code)]
pub async fn natural_typing(page: &Page, selector: &str, text: &str, typo_rate: f64) -> Result<()> {
    page.evaluate(format!("document.querySelector('{selector}').focus();")).await?;

    // Type each character with human-like timing
    for (i, ch) in text.chars().enumerate() {
        // Occasionally make typos based on typo rate
        if (random_in_range(0, 100) as f64 / 100.0) < typo_rate && i > 0 {
            await_typo_correction(page, ch).await?;
        } else {
            await_type_character(page, ch).await?;
        }
    }

    Ok(())
}

#[allow(dead_code)]
async fn await_type_character(page: &Page, ch: char) -> Result<()> {
    // Simulate key press with human timing
    let key_delay = gaussian(120.0, 40.0, 50.0, 300.0) as u64;

    // Type the character
    page.evaluate(format!("
        const element = document.activeElement;
        if (element) {{
            const inputEvent = new InputEvent('input', {{
                data: '{ch}',
                bubbles: true
            }});
            element.value += '{ch}';
            element.dispatchEvent(inputEvent);
        }}
    ")).await?;

    human_pause(key_delay, 30).await;
    Ok(())
}

#[allow(dead_code)]
async fn await_typo_correction(page: &Page, correct_char: char) -> Result<()> {
    // Type a wrong character first
    let wrong_char = get_similar_char(correct_char);
    await_type_character(page, wrong_char).await?;

    // Brief pause before correction
    human_pause(300, 50).await;

    // Backspace
    press_key(page, "Backspace").await?;

    // Pause before retyping
    human_pause(200, 50).await;

    // Type correct character
    await_type_character(page, correct_char).await?;

    Ok(())
}

#[allow(dead_code)]
pub async fn press_key(page: &Page, key: &str) -> Result<()> {
    // Simulate key press
    page.evaluate(format!("
        const element = document.activeElement;
        if (element) {{
            const keyEvent = new KeyboardEvent('keydown', {{
                key: '{key}',
                code: '{key}',
                bubbles: true
            }});
            element.dispatchEvent(keyEvent);
        }}
    ")).await?;

    human_pause(50, 20).await;
    Ok(())
}

#[allow(dead_code)]
fn should_make_typo() -> bool {
    // 2% chance of typo
    random_in_range(0, 100) < 2
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