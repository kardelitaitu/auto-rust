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
