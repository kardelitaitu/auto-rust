//! Shared utilities for sentiment analysis.
//! Centralizes string parsing and tokenization logic.

use std::collections::HashMap;

/// Tokenize text into words, normalizing case and handling punctuation.
/// This centralizes the tokenization logic used across different sentiment strategies.
///
/// # Arguments
/// * `text` - The text to tokenize
///
/// # Returns
/// Vector of lowercase word tokens
pub fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| {
            word.chars()
                .filter(|c| c.is_alphanumeric() || *c == '\'')
                .collect::<String>()
                .to_lowercase()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

/// Check if text contains a word (case-insensitive).
/// Uses tokenization for consistent matching.
///
/// # Arguments
/// * `text` - The text to search
/// * `word` - The word to find
pub fn contains_word(text: &str, word: &str) -> bool {
    let tokens = tokenize(text);
    let word_lower = word.to_lowercase();
    tokens.contains(&word_lower)
}

/// Count occurrences of words in text.
/// Returns a HashMap of word -> count.
///
/// # Arguments
/// * `text` - The text to analyze
pub fn word_counts(text: &str) -> HashMap<String, usize> {
    let tokens = tokenize(text);
    let mut counts = HashMap::new();

    for token in tokens {
        *counts.entry(token).or_insert(0) += 1;
    }

    counts
}

/// Extract substrings within word boundaries.
/// Useful for detecting phrases and patterns.
///
/// # Arguments
/// * `text` - The text to search
/// * `pattern` - The substring pattern to find
pub fn contains_substring(text: &str, pattern: &str) -> bool {
    text.to_lowercase().contains(&pattern.to_lowercase())
}

/// Normalize text for consistent processing.
/// Converts to lowercase and trims whitespace.
///
/// # Arguments
/// * `text` - The text to normalize
pub fn normalize_text(text: &str) -> String {
    text.to_lowercase().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_basic() {
        let tokens = tokenize("Hello world!");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_punctuation() {
        let tokens = tokenize("Hello, world! How are you?");
        assert_eq!(tokens, vec!["hello", "world", "how", "are", "you"]);
    }

    #[test]
    fn test_tokenize_apostrophe() {
        let tokens = tokenize("Don't can't won't");
        assert_eq!(tokens, vec!["don't", "can't", "won't"]);
    }

    #[test]
    fn test_contains_word() {
        assert!(contains_word("Hello world", "hello"));
        assert!(contains_word("HELLO WORLD", "world"));
        assert!(!contains_word("Hello world", "goodbye"));
    }

    #[test]
    fn test_word_counts() {
        let counts = word_counts("hello world hello");
        assert_eq!(counts.get("hello"), Some(&2));
        assert_eq!(counts.get("world"), Some(&1));
    }

    #[test]
    fn test_contains_substring() {
        assert!(contains_substring("Hello world", "ello wor"));
        assert!(contains_substring("HELLO WORLD", "world"));
        assert!(!contains_substring("Hello world", "goodbye"));
    }

    #[test]
    fn test_normalize_text() {
        assert_eq!(normalize_text("  HELLO WORLD  "), "hello world");
    }
}
