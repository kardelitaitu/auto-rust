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

    #[test]
    fn preview_chars_empty_string() {
        assert_eq!(preview_chars("", 10), "");
    }

    #[test]
    fn preview_chars_zero_limit() {
        assert_eq!(preview_chars("hello", 0), "");
    }

    #[test]
    fn preview_chars_longer_than_text() {
        assert_eq!(preview_chars("hello", 100), "hello");
    }

    #[test]
    fn preview_chars_exact_length() {
        assert_eq!(preview_chars("hello", 5), "hello");
    }

    #[test]
    fn truncate_chars_empty_string() {
        assert_eq!(truncate_chars("", 10), "");
    }

    #[test]
    fn truncate_chars_zero_limit() {
        assert_eq!(truncate_chars("hello", 0), "");
    }

    #[test]
    fn truncate_chars_longer_than_text() {
        assert_eq!(truncate_chars("hello", 100), "hello");
    }

    #[test]
    fn truncate_chars_exact_length() {
        assert_eq!(truncate_chars("hello", 5), "hello");
    }

    #[test]
    fn truncate_with_ellipsis_no_truncation_needed() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
    }

    #[test]
    fn truncate_with_ellipsis_exact_length() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn truncate_with_ellipsis_truncates_and_adds_ellipsis() {
        let result = truncate_with_ellipsis("hello world", 8);
        assert_eq!(result, "hello...");
        assert_eq!(result.chars().count(), 8);
    }

    #[test]
    fn truncate_with_ellipsis_empty_string() {
        assert_eq!(truncate_with_ellipsis("", 10), "");
    }

    #[test]
    fn truncate_with_ellipsis_zero_limit() {
        assert_eq!(truncate_with_ellipsis("hello", 0), "...");
    }

    #[test]
    fn truncate_with_ellipsis_shorter_than_ellipsis() {
        let result = truncate_with_ellipsis("hi", 2);
        assert_eq!(result, "hi"); // No truncation needed
    }

    #[test]
    fn truncate_with_ellipsis_one_less_than_ellipsis() {
        let result = truncate_with_ellipsis("hi", 3);
        assert_eq!(result, "hi"); // No truncation needed, text fits within limit
    }

    #[test]
    fn preview_chars_multibyte_characters() {
        let text = "日本語テキスト";
        assert_eq!(preview_chars(text, 3), "日本語");
    }

    #[test]
    fn truncate_chars_multibyte_characters() {
        let text = "日本語テキスト";
        assert_eq!(truncate_chars(text, 3), "日本語");
    }

    #[test]
    fn truncate_with_ellipsis_multibyte_characters() {
        let text = "日本語テキスト";
        let result = truncate_with_ellipsis(text, 5);
        assert_eq!(result.chars().count(), 5);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn preview_chars_single_character() {
        assert_eq!(preview_chars("hello", 1), "h");
    }

    #[test]
    fn truncate_chars_single_character() {
        assert_eq!(truncate_chars("hello", 1), "h");
    }

    #[test]
    fn truncate_with_ellipsis_emoji_sequence() {
        let text = "😀😁😂🤣😃😄😅";
        let result = truncate_with_ellipsis(text, 5);
        assert_eq!(result.chars().count(), 5);
        assert!(result.ends_with("..."));
    }
}
