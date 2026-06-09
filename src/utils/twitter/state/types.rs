//! Core types for Twitter activity state: errors, templates, candidate context/results.

use crate::prelude::TaskContext;
use crate::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};
use crate::utils::twitter::twitteractivity_persona::PersonaWeights;
use serde_json::Value;
use std::time::{Duration, Instant};

use super::config::TaskConfig;
use super::tracking::TweetActionTracker;

/// Validation errors for task payload.
#[derive(Debug)]
pub enum TaskValidationError {
    InvalidPositiveNumber {
        field: String,
        value: i64,
    },
    InvalidFieldType {
        field: String,
        expected: &'static str,
        actual: &'static str,
    },
}

impl std::fmt::Display for TaskValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskValidationError::InvalidPositiveNumber { field, value } => {
                write!(f, "Invalid value for '{field}': {value} (must be positive)")
            }
            TaskValidationError::InvalidFieldType {
                field,
                expected,
                actual,
            } => write!(
                f,
                "Invalid value for '{field}': {actual} (must be {expected})"
            ),
        }
    }
}

impl std::error::Error for TaskValidationError {}

/// Configuration for reply and quote text templates by sentiment.
#[derive(Debug, Clone)]
pub struct SentimentTemplates {
    pub reply_positive: Vec<String>,
    pub reply_neutral: Vec<String>,
    pub reply_negative: Vec<String>,
    pub quote_positive: Vec<String>,
    pub quote_neutral: Vec<String>,
    pub quote_negative: Vec<String>,
}

impl Default for SentimentTemplates {
    fn default() -> Self {
        Self {
            reply_positive: vec![
                "Great point!".to_string(),
                "Absolutely agree.".to_string(),
                "Well said.".to_string(),
                "Thanks for sharing!".to_string(),
                "This is spot on.".to_string(),
            ],
            reply_neutral: vec![
                "Interesting.".to_string(),
                "Thanks.".to_string(),
                "Noted.".to_string(),
                "Hmm.".to_string(),
                "I see.".to_string(),
            ],
            reply_negative: vec![
                "I disagree, but good discussion.".to_string(),
                "Different perspective, but thanks.".to_string(),
                "I see your point, though I think otherwise.".to_string(),
                "Respectfully, I have to differ.".to_string(),
            ],
            quote_positive: vec![
                "This is worth sharing.".to_string(),
                "Great perspective here.".to_string(),
                "Agreed with this take.".to_string(),
                "Important point worth highlighting.".to_string(),
                "This resonates.".to_string(),
            ],
            quote_neutral: vec![
                "Worth a read.".to_string(),
                "Interesting take.".to_string(),
                "Good point here.".to_string(),
                "Noting this one.".to_string(),
                "Thoughts on this.".to_string(),
            ],
            quote_negative: vec![
                "Different perspective worth considering.".to_string(),
                "This raises important questions.".to_string(),
                "Worth discussing further.".to_string(),
                "Challenging viewpoint here.".to_string(),
                "Food for thought.".to_string(),
            ],
        }
    }
}

/// Context for processing a single tweet candidate.
/// Groups configuration and mutable state for candidate processing.
pub struct CandidateContext<'a> {
    pub tweet: &'a Value,
    pub persona: &'a PersonaWeights,
    pub task_config: &'a TaskConfig,
    pub api: &'a TaskContext,
    pub limits: &'a EngagementLimits,
    pub scroll_interval: Duration,
    pub action_tracker: &'a mut TweetActionTracker,
    pub counters: &'a mut EngagementCounters,
}

/// Result of processing a single candidate tweet.
/// Replaces the 5-tuple return type `(bool, Instant, u32, u32, Option<ThreadCache>)`
pub struct CandidateResult {
    pub should_break: bool,
    pub next_scroll: Instant,
    pub next_candidate_scan: Instant,
    pub actions_this_scan: u32,
}

#[cfg(test)]
mod display_tests {
    use super::TaskValidationError;

    #[test]
    fn task_validation_error_display_mentions_field() {
        let err = TaskValidationError::InvalidPositiveNumber {
            field: "duration_ms".to_string(),
            value: -100,
        };
        let display = format!("{}", err);
        assert!(display.contains("duration_ms"));
        assert!(display.contains("must be positive"));
    }
}

#[cfg(test)]
mod gap_tests {
    use super::SentimentTemplates;

    // SentimentTemplates default structure
    #[test]
    fn sentiment_templates_default_has_non_empty_vectors() {
        let t = SentimentTemplates::default();
        assert!(
            t.reply_positive.len() >= 3,
            "reply_positive should have templates"
        );
        assert!(
            t.reply_neutral.len() >= 3,
            "reply_neutral should have templates"
        );
        assert!(
            t.reply_negative.len() >= 3,
            "reply_negative should have templates"
        );
        assert!(
            t.quote_positive.len() >= 3,
            "quote_positive should have templates"
        );
        assert!(
            t.quote_neutral.len() >= 3,
            "quote_neutral should have templates"
        );
        assert!(
            t.quote_negative.len() >= 3,
            "quote_negative should have templates"
        );
    }

    #[test]
    fn sentiment_templates_default_no_empty_strings() {
        let t = SentimentTemplates::default();
        for (name, vec) in [
            ("reply_positive", &t.reply_positive),
            ("reply_neutral", &t.reply_neutral),
            ("reply_negative", &t.reply_negative),
            ("quote_positive", &t.quote_positive),
            ("quote_neutral", &t.quote_neutral),
            ("quote_negative", &t.quote_negative),
        ] {
            for (i, s) in vec.iter().enumerate() {
                assert!(!s.is_empty(), "{name}[{i}] should not be empty");
            }
        }
    }

    #[test]
    fn task_validation_error_invalid_field_type_display() {
        use super::TaskValidationError;
        let err = TaskValidationError::InvalidFieldType {
            field: "candidate_count".to_string(),
            expected: "positive u32",
            actual: "string",
        };
        let display = format!("{err}");
        assert!(display.contains("candidate_count"));
        assert!(display.contains("positive u32"));
        assert!(display.contains("string"));
    }
}
