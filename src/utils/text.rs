//! UTF-8 safe text helpers used by task and utility code.

/// Returns a preview limited to the requested number of characters.
pub fn preview_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

/// Truncates text to the requested number of characters.
pub fn truncate_chars(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

/// Truncates text and appends an ellipsis when truncation occurs.
pub fn truncate_with_ellipsis(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let ellipsis = "...";
    let keep = max_chars.saturating_sub(ellipsis.chars().count());
    let mut result = text.chars().take(keep).collect::<String>();
    result.push_str(ellipsis);
    result
}

#[cfg(test)]
mod tests {
    use super::{preview_chars, truncate_chars, truncate_with_ellipsis};

    #[test]
    fn preview_and_truncate_are_char_safe() {
        assert_eq!(preview_chars("naïve🙂text", 6), "naïve🙂");
        assert_eq!(truncate_chars("naïve🙂text", 6), "naïve🙂");
    }

    #[test]
    fn truncate_with_ellipsis_preserves_utf8() {
        let text = "🙂".repeat(300);
        let out = truncate_with_ellipsis(&text, 280);
        assert_eq!(out.chars().count(), 280);
        assert!(out.ends_with("..."));
    }
}
