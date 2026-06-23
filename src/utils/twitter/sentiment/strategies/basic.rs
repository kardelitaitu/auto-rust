//! Basic keyword-based sentiment strategy.
//!
//! Extracted from `sentiment/analyzer.rs` — spec 0020.

use super::super::helpers::{calculate_contextual_score, NEGATIVE_WORDS, POSITIVE_WORDS};
use super::super::SentimentStrategy;

#[derive(Debug)]
pub struct BasicKeywordStrategy;

impl SentimentStrategy for BasicKeywordStrategy {
    fn analyze(&self, text: &str) -> f32 {
        let mut score = 0.0;
        let lower = text.to_lowercase();
        for &word in POSITIVE_WORDS {
            if crate::utils::twitter::sentiment::utils::contains_word(&lower, word) {
                score += calculate_contextual_score(&lower, 1.0, word);
            }
        }
        for &word in NEGATIVE_WORDS {
            if crate::utils::twitter::sentiment::utils::contains_word(&lower, word) {
                score += calculate_contextual_score(&lower, -1.0, word);
            }
        }
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::twitter::sentiment::SentimentStrategy;

    #[test]
    fn test_basic_positive_sentiment() {
        let strategy = BasicKeywordStrategy;
        let score = strategy.analyze("This is great and amazing!");
        assert!(
            score > 0.0,
            "Positive text should produce positive score, got {score}"
        );
    }

    #[test]
    fn test_basic_negative_sentiment() {
        let strategy = BasicKeywordStrategy;
        let score = strategy.analyze("This is terrible and awful!");
        assert!(
            score < 0.0,
            "Negative text should produce negative score, got {score}"
        );
    }

    #[test]
    fn test_basic_neutral_text() {
        let strategy = BasicKeywordStrategy;
        let score = strategy.analyze("The sky is blue today.");
        assert_eq!(
            score, 0.0,
            "Neutral text should produce zero score, got {score}"
        );
    }

    #[test]
    fn test_basic_empty_string() {
        let strategy = BasicKeywordStrategy;
        let score = strategy.analyze("");
        assert_eq!(
            score, 0.0,
            "Empty string should produce zero score, got {score}"
        );
    }

    #[test]
    fn test_basic_mixed_sentiment() {
        let strategy = BasicKeywordStrategy;
        // "good" is positive (+1.0), "bad" is negative (-1.0), with no modifiers
        let score = strategy.analyze("good bad");
        // Both matched, no intensifiers or negations — should be ~0.0
        assert!(
            (score).abs() < 0.1,
            "Mixed positive/negative should roughly cancel, got {score}"
        );
    }

    #[test]
    fn test_basic_with_intensifier() {
        let strategy = BasicKeywordStrategy;
        let normal = strategy.analyze("This is good");
        let intensified = strategy.analyze("This is very good");
        assert!(
            intensified.abs() > normal.abs(),
            "Intensified sentiment 'very good' should have higher magnitude than 'good' alone (normal={normal}, intensified={intensified})"
        );
    }

    #[test]
    fn test_basic_with_negation() {
        let strategy = BasicKeywordStrategy;
        let score = strategy.analyze("This is not good");
        // "not good" — negation flips positive to negative
        assert!(
            score < 0.0,
            "Negated positive 'not good' should produce negative score, got {score}"
        );
    }

    #[test]
    fn test_basic_case_insensitive() {
        let strategy = BasicKeywordStrategy;
        let lower = strategy.analyze("this is good");
        let upper = strategy.analyze("THIS IS GOOD");
        let mixed = strategy.analyze("This Is Good");
        assert!(
            (lower - upper).abs() < 0.01,
            "Case should not affect score (lower={lower}, upper={upper})"
        );
        assert!(
            (lower - mixed).abs() < 0.01,
            "Case should not affect score (lower={lower}, mixed={mixed})"
        );
    }

    #[test]
    fn test_basic_positive_word_exact() {
        let strategy = BasicKeywordStrategy;
        // "good" is in POSITIVE_WORDS and reliably matches
        let score = strategy.analyze("good");
        assert!(
            score > 0.0,
            "Exact positive word 'good' should produce positive score, got {score}"
        );
    }

    #[test]
    fn test_basic_negative_word_exact() {
        let strategy = BasicKeywordStrategy;
        // "bad" is in NEGATIVE_WORDS and reliably matches
        let score = strategy.analyze("bad");
        assert!(
            score < 0.0,
            "Exact negative word 'bad' should produce negative score, got {score}"
        );
    }

    #[test]
    fn test_basic_multiple_positive_words() {
        let strategy = BasicKeywordStrategy;
        let single = strategy.analyze("good");
        let multiple = strategy.analyze("good great amazing excellent");
        assert!(
            multiple.abs() > single.abs(),
            "Multiple positive words should produce higher magnitude than one (single={single}, multiple={multiple})"
        );
    }

    #[test]
    fn test_basic_multiple_negative_words() {
        let strategy = BasicKeywordStrategy;
        let single = strategy.analyze("bad");
        let multiple = strategy.analyze("bad terrible awful horrible");
        assert!(
            multiple.abs() > single.abs(),
            "Multiple negative words should produce higher magnitude than one (single={single}, multiple={multiple})"
        );
    }

    #[test]
    fn test_basic_punctuation_does_not_affect() {
        let strategy = BasicKeywordStrategy;
        let clean = strategy.analyze("good");
        let punctuated = strategy.analyze("good!?!?!!");
        assert!(
            (clean - punctuated).abs() < 0.01,
            "Punctuation should not affect word matching (clean={clean}, punctuated={punctuated})"
        );
    }
}
