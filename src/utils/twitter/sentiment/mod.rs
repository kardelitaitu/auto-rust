//! Consolidated sentiment analysis module.
//! Provides a unified interface using the Strategy Pattern for flexible sentiment analysis.

/// Trait for sentiment analysis strategies.
/// Each strategy analyzes a specific aspect and returns a sentiment score contribution.
pub trait SentimentStrategy: std::fmt::Debug + Send + Sync {
    /// Analyze the given text and return a sentiment score contribution.
    /// Positive scores indicate positive sentiment, negative scores indicate negative sentiment.
    ///
    /// # Arguments
    /// * `text` - The text to analyze
    ///
    /// # Returns
    /// Sentiment score contribution (typically -3.0 to +3.0)
    fn analyze(&self, text: &str) -> f32;
}

pub mod core;
pub mod helpers;
pub mod strategies;
pub mod types;
pub mod utils;

pub use core::*;
pub use helpers::*;
pub use types::*;
