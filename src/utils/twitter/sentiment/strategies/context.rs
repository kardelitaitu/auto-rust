//! Context-aware sentiment strategy.
//!
//! Extracted from `sentiment/analyzer.rs` — spec 0020.

use super::super::helpers::analyze_contextual_modifiers;
use super::super::SentimentStrategy;

#[derive(Debug)]
pub struct ContextStrategy;

impl SentimentStrategy for ContextStrategy {
    fn analyze(&self, text: &str) -> f32 {
        analyze_contextual_modifiers(text)
    }
}
