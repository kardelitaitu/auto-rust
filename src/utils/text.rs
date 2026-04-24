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
    fn test_preview_chars_shorter_than_max() {
        assert_eq!(preview_chars("hello", 10), "hello");
    }

    #[test]
    fn test_preview_chars_exact_length() {
        assert_eq!(preview_chars("hello", 5), "hello");
    }

    #[test]
    fn test_preview_chars_longer_than_max() {
        assert_eq!(preview_chars("hello world", 5), "hello");
    }

    #[test]
    fn test_preview_chars_zero_max() {
        assert_eq!(preview_chars("hello", 0), "");
    }

    #[test]
    fn test_preview_chars_empty_string() {
        assert_eq!(preview_chars("", 10), "");
    }

    #[test]
    fn test_truncate_chars_shorter_than_max() {
        assert_eq!(truncate_chars("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_chars_exact_length() {
        assert_eq!(truncate_chars("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_chars_longer_than_max() {
        assert_eq!(truncate_chars("hello world", 5), "hello");
    }

    #[test]
    fn test_truncate_chars_zero_max() {
        assert_eq!(truncate_chars("hello", 0), "");
    }

    #[test]
    fn test_truncate_chars_empty_string() {
        assert_eq!(truncate_chars("", 10), "");
    }

    #[test]
    fn test_truncate_with_ellipsis_no_truncation() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_with_ellipsis_exact_length() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_with_ellipsis_truncates() {
        assert_eq!(truncate_with_ellipsis("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_with_ellipsis_zero_max() {
        assert_eq!(truncate_with_ellipsis("hello", 0), "");
    }

    #[test]
    fn test_truncate_with_ellipsis_less_than_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello", 2), "");
    }

    #[test]
    fn test_truncate_with_ellipsis_exactly_ellipsis() {
        assert_eq!(truncate_with_ellipsis("hello", 3), "...");
    }

    #[test]
    fn test_truncate_with_ellipsis_empty_string() {
        assert_eq!(truncate_with_ellipsis("", 10), "");
    }

    #[test]
    fn test_truncate_with_ellipsis_multibyte_characters() {
        assert_eq!(truncate_with_ellipsis("naïve", 4), "naï...");
    }

    #[test]
    fn test_truncate_with_ellipsis_emoji() {
        assert_eq!(truncate_with_ellipsis("🙂🙃😊", 4), "🙂🙃...");
    }

    #[test]
    fn test_truncate_with_ellipsis_whitespace() {
        assert_eq!(truncate_with_ellipsis("a b c", 3), "a...");
    }

    #[test]
    fn test_truncate_with_ellipsis_large_max() {
        let text = "x".repeat(1000);
        assert_eq!(truncate_with_ellipsis(&text, 1000), text);
    }

    #[test]
    fn test_preview_chars_multibyte() {
        assert_eq!(preview_chars("naïve", 4), "naïv");
    }

    #[test]
    fn test_truncate_chars_multibyte() {
        assert_eq!(truncate_chars("naïve", 4), "naïv");
    }
}
