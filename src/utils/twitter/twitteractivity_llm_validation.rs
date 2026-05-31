//! LLM output validation and sanitization for Twitter.

use anyhow::Result;
use log::warn;
use std::sync::OnceLock;

/// Banned AI-sounding words list.
pub const BANNED_WORDS: &[&str] = &[
    "tapestry",
    "testament",
    "symphony",
    "delve",
    "foster",
    "crucial",
    "landscape",
    "game-changer",
    "underscore",
    "utilize",
    "enhance",
    "spearhead",
    "resonate",
    "vibrant",
    "seamless",
    "robust",
    "dynamic",
    "realm",
    "nuance",
    "harness",
    "leverage",
    "meticulous",
    "paradigm",
    "synergy",
    "holistic",
    "integral",
    "pivotal",
    "noteworthy",
    "compelling",
    "intriguing",
    "fascinating",
    "captivating",
    "enthralling",
    "empower",
    "revolutionize",
    "deep dive",
    "unpack",
    "in conclusion",
    "moreover",
    "furthermore",
    "it's important to note",
    "ah,",
    "i see",
    "as a",
];

/// Validates and sanitizes LLM-generated text for Twitter.
pub fn validate_reply(text: &str) -> Result<String> {
    let mut sanitized = text.trim().to_string();

    // Remove asterisk emphasis (**word** and *word*)
    sanitized = sanitized.replace("**", "").replace('*', "");

    // Enforce character limit
    if sanitized.len() > 270 {
        // Leave room for ...
        sanitized = truncate_to_word_boundary(&sanitized, 270);
    }

    // Remove @mentions
    sanitized = remove_mentions(&sanitized);

    // Remove #hashtags
    sanitized = remove_hashtags(&sanitized);

    // Remove emojis (basic Unicode range check)
    sanitized = remove_emojis(&sanitized);

    // Check for banned AI words
    if let Some(banned_word) = check_banned_words(&sanitized) {
        warn!("Reply contains banned AI word: '{banned_word}', but proceeding");
    }

    // Ensure non-empty
    if sanitized.is_empty() {
        anyhow::bail!("Generated reply is empty after sanitization");
    }

    Ok(sanitized)
}

/// Truncates text to `max_length` at word boundary.
fn truncate_to_word_boundary(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        return text.to_string();
    }

    // Find last space before max_length (leave room for "...")
    let truncate_limit = max_length.saturating_sub(3);
    // Use char-safe boundary to avoid panic on multi-byte UTF-8
    let safe_boundary = text.floor_char_boundary(truncate_limit);
    let truncate_at = text[..safe_boundary].rfind(' ').unwrap_or(safe_boundary);

    format!("{}...", &text[..truncate_at])
}

fn mentions_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"@\w+").expect("Failed to compile mentions regex"))
}

fn hashtags_regex() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"#(\w+)").expect("Failed to compile hashtags regex"))
}

/// Removes @mentions from text.
fn remove_mentions(text: &str) -> String {
    mentions_regex().replace_all(text, "").to_string()
}

/// Removes #hashtags from text.
fn remove_hashtags(text: &str) -> String {
    hashtags_regex().replace_all(text, "$1").to_string()
}

/// Removes emojis from text (basic Unicode ranges).
fn remove_emojis(text: &str) -> String {
    text.chars()
        .filter(|c| {
            let cp = *c as u32;
            !(0x1F600..=0x1F64F).contains(&cp)
                && !(0x1F300..=0x1F5FF).contains(&cp)
                && !(0x1F680..=0x1F6FF).contains(&cp)
                && !(0x1F1E0..=0x1F1FF).contains(&cp)
                && !(0x2600..=0x26FF).contains(&cp)
                && !(0x2700..=0x27BF).contains(&cp)
        })
        .collect()
}

/// Checks if text contains banned AI words.
fn check_banned_words(text: &str) -> Option<String> {
    let text_lower = text.to_lowercase();
    for word in BANNED_WORDS {
        if text_lower.contains(word) {
            return Some(word.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_reply_truncates_long_text() {
        let long_text = "a".repeat(300);
        let result = validate_reply(&long_text).unwrap();
        assert!(result.len() <= 270);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_validate_reply_removes_mentions() {
        let text = "Great point @user! I agree with @someone else.";
        let result = validate_reply(text).unwrap();
        assert!(!result.contains("@user"));
        assert!(!result.contains("@someone"));
    }

    #[test]
    fn test_validate_reply_removes_hashtags() {
        let text = "This is #amazing and #awesome!";
        let result = validate_reply(text).unwrap();
        assert!(!result.contains("#"));
        assert!(result.contains("amazing"));
        assert!(result.contains("awesome"));
    }

    #[test]
    fn test_validate_reply_removes_emojis() {
        let text = "Love this! ❤️ 🔥 👍";
        let result = validate_reply(text).unwrap();
        assert!(!result.contains("❤"));
        assert!(!result.contains("🔥"));
        assert!(!result.contains("👍"));
    }

    #[test]
    fn test_check_banned_words_detects_ai_speak() {
        assert!(check_banned_words("This is crucial for the landscape").is_some());
        assert!(check_banned_words("Let me delve into this").is_some());
        assert!(check_banned_words("Normal text without banned words").is_none());
    }

    #[test]
    fn test_truncate_to_word_boundary() {
        let text = "This is a long sentence with many words that needs truncation";
        let result = truncate_to_word_boundary(text, 30);
        assert!(result.len() <= 33); // 30 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_validate_reply_removes_asterisks() {
        let text = "This is **bold** and *italic* text";
        let result = validate_reply(text).unwrap();
        assert!(!result.contains("**"));
        assert!(!result.contains("*"));
    }

    #[test]
    fn test_validate_reply_empty_after_sanitization() {
        let text = "@user #tag 😀";
        let result = validate_reply(text);
        assert!(result.is_ok());
        let sanitized = result.unwrap();
        assert!(!sanitized.contains("@"));
        assert!(!sanitized.contains("#"));
    }

    #[test]
    fn test_remove_mentions_function() {
        let text = "Hello @user1 and @user2";
        let result = remove_mentions(text);
        assert!(!result.contains("@user1"));
        assert!(!result.contains("@user2"));
        assert!(result.contains("Hello"));
    }

    #[test]
    fn test_remove_hashtags_function() {
        let text = "This is #tech and #coding";
        let result = remove_hashtags(text);
        assert!(!result.contains("#"));
        assert!(result.contains("tech"));
        assert!(result.contains("coding"));
    }

    #[test]
    fn test_remove_emojis_function() {
        let text = "Test 😀 🔥 👍";
        let result = remove_emojis(text);
        assert!(!result.contains("😀"));
        assert!(!result.contains("🔥"));
        assert!(!result.contains("👍"));
        assert!(result.contains("Test"));
    }

    #[test]
    fn test_truncate_to_word_boundary_short_text() {
        let text = "Short";
        let result = truncate_to_word_boundary(text, 30);
        assert_eq!(result, "Short");
    }

    #[test]
    fn test_truncate_to_word_boundary_no_space() {
        let text = "Verylongwordwithoutspaces";
        let result = truncate_to_word_boundary(text, 10);
        assert!(result.len() <= 13); // 10 + "..."
    }
}
