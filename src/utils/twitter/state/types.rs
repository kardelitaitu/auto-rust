//! Core types for Twitter activity state: errors, templates, candidate context/results.

use crate::prelude::TaskContext;
use crate::utils::twitter::sentiment::types::ConversationIndicator;
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

/// Parses `{x, y}` coordinates from a JSON value returned by JS evaluation.
/// Returns `None` if the value is not an object with valid numeric `x` and `y`.
///
/// Shared utility — used by both `twitteractivity_interact` and
/// `twitteractivity_humanized` modules.
#[must_use]
pub fn parse_button_coordinates(value: &Value) -> Option<(f64, f64)> {
    value.as_object().and_then(|obj| {
        let x = obj.get("x")?.as_f64()?;
        let y = obj.get("y")?.as_f64()?;
        Some((x, y))
    })
}

/// Parses a `{sent: bool, reason: string}` verification result from reply sending.
/// - `sent: true` → `EngagementOutcome::Completed`
/// - `sent: false` → `EngagementOutcome::Failed`
/// - Missing/invalid `sent` → `EngagementOutcome::Unverified`
#[must_use]
pub fn parse_reply_verification(value: &Value) -> crate::utils::twitter::EngagementOutcome {
    value
        .as_object()
        .and_then(|obj| obj.get("sent").and_then(Value::as_bool))
        .map_or(
            crate::utils::twitter::EngagementOutcome::Unverified,
            |sent| {
                if sent {
                    crate::utils::twitter::EngagementOutcome::Completed
                } else {
                    crate::utils::twitter::EngagementOutcome::Failed
                }
            },
        )
}

/// Parses a boolean following-check result from a JS evaluation.
/// Returns `false` for any non-boolean or missing value.
#[must_use]
pub fn parse_following_result(value: &Value) -> bool {
    value.as_bool().unwrap_or(false)
}

/// Parses `{x, y}` coordinates from a JSON value, defaulting to `(0.0, 0.0)`
/// if the value is not an object with valid numeric `x` and `y`.
///
/// Unlike `parse_button_coordinates` (which returns `None` on failure),
/// this returns defaults — useful when the caller wants to proceed with
/// fallback coordinates instead of skipping the action.
#[must_use]
pub fn parse_coordinates_with_default(value: &Value) -> (f64, f64) {
    value.as_object().map_or((0.0, 0.0), |obj| {
        let x = obj.get("x").and_then(Value::as_f64).unwrap_or(0.0);
        let y = obj.get("y").and_then(Value::as_f64).unwrap_or(0.0);
        (x, y)
    })
}

// ============================================================================
// Trending Bias Computation
// ============================================================================

/// Compute a trending bias (-1.0 to 1.0) from tweet engagement signals.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn compute_trending_bias(tweet_obj: &Value) -> f32 {
    let likes = tweet_obj
        .get("metrics")
        .and_then(|m| m.get("likes"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as f32;
    let retweets = tweet_obj
        .get("metrics")
        .and_then(|m| m.get("retweets"))
        .and_then(|v| v.as_f64())
        .or_else(|| tweet_obj.get("retweet_count").and_then(|v| v.as_f64()))
        .unwrap_or(0.0) as f32;
    let replies = tweet_obj
        .get("metrics")
        .and_then(|m| m.get("replies"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as f32;

    // Log-scale trending: higher engagement = more trending
    let total = likes + retweets * 2.0 + replies * 1.5;
    if total <= 0.0 {
        return 0.0;
    }

    // Map engagement to a -1..1 scale via log
    // 0 → 0.0, 10 → 0.3, 100 → 0.6, 1000 → 0.9
    let log_score = (total + 1.0).ln() / 7.0; // ln(1)=0, ln(1000)≈6.9
    log_score.clamp(-1.0_f32, 1.0_f32)
}

// ============================================================================
// Conversation Pattern Detection
// ============================================================================

const AGREEMENT_PATTERNS: &[&str] = &[
    "i agree",
    "totally agree",
    "absolutely",
    "exactly",
    "you're right",
    "well said",
];
const DISAGREEMENT_PATTERNS: &[&str] = &[
    "i disagree",
    "totally disagree",
    "you're wrong",
    "not sure",
    "doubt it",
];
const QUESTION_PATTERNS: &[&str] = &[
    "what if",
    "how come",
    "why is",
    "what do you",
    "can you explain",
];
const CLARIFICATION_PATTERNS: &[&str] = &[
    "to clarify",
    "let me explain",
    "what i mean",
    "in other words",
];
const HUMOR_PATTERNS: &[&str] = &["lol", "haha", "😂", "🤣", "joke", "funny"];
const SUPPORT_PATTERNS: &[&str] = &["i support", "good luck", "keep going", "you're doing great"];
const CRITICISM_PATTERNS: &[&str] = &[
    "that's bad",
    "you shouldn't",
    "that's wrong",
    "disappointing",
];
const SARCASM_INDICATORS: &[&str] = &["oh sure", "yeah right", "as if", "oh please", "oh come on"];

/// Detect conversation indicators (Agreement, Disagreement, Question, etc.)
/// from tweet text by matching against known patterns.
#[must_use]
pub fn detect_conversation_indicators(text: &str) -> Vec<ConversationIndicator> {
    let lower = text.to_lowercase();
    let mut indicators = Vec::new();
    if AGREEMENT_PATTERNS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Agreement);
    }
    if DISAGREEMENT_PATTERNS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Disagreement);
    }
    if QUESTION_PATTERNS.iter().any(|&p| lower.contains(p)) || text.contains('?') {
        indicators.push(ConversationIndicator::Question);
    }
    if CLARIFICATION_PATTERNS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Clarification);
    }
    if HUMOR_PATTERNS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Humor);
    }
    if SUPPORT_PATTERNS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Support);
    }
    if CRITICISM_PATTERNS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Criticism);
    }
    if SARCASM_INDICATORS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Sarcasm);
    }
    indicators
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
    use super::{
        compute_trending_bias, detect_conversation_indicators, parse_coordinates_with_default,
        ConversationIndicator, SentimentTemplates,
    };
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

    // ====================================================================
    // parse_coordinates_with_default
    // ====================================================================

    #[test]
    fn coords_with_default_valid() {
        let value = serde_json::json!({"x": 100.5, "y": 200.3});
        assert_eq!(parse_coordinates_with_default(&value), (100.5, 200.3));
    }

    #[test]
    fn coords_with_default_missing_x() {
        let value = serde_json::json!({"y": 200.3});
        assert_eq!(parse_coordinates_with_default(&value), (0.0, 200.3));
    }

    #[test]
    fn coords_with_default_missing_y() {
        let value = serde_json::json!({"x": 100.5});
        assert_eq!(parse_coordinates_with_default(&value), (100.5, 0.0));
    }

    #[test]
    fn coords_with_default_empty_object() {
        let value = serde_json::json!({});
        assert_eq!(parse_coordinates_with_default(&value), (0.0, 0.0));
    }

    #[test]
    fn coords_with_default_non_object() {
        assert_eq!(
            parse_coordinates_with_default(&serde_json::json!(42)),
            (0.0, 0.0)
        );
        assert_eq!(
            parse_coordinates_with_default(&serde_json::json!("string")),
            (0.0, 0.0)
        );
        assert_eq!(
            parse_coordinates_with_default(&serde_json::json!(null)),
            (0.0, 0.0)
        );
    }

    #[test]
    fn coords_with_default_integer_values() {
        let value = serde_json::json!({"x": 100, "y": 200});
        let coords: (f64, f64) = parse_coordinates_with_default(&value);
        assert!((coords.0 - 100.0).abs() < f64::EPSILON);
        assert!((coords.1 - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn coords_with_default_null_values() {
        let value = serde_json::json!({"x": null, "y": 200.3});
        assert_eq!(parse_coordinates_with_default(&value), (0.0, 200.3));
    }

    // ====================================================================
    // compute_trending_bias
    // ====================================================================

    #[test]
    fn trending_bias_zero_engagement() {
        assert_eq!(compute_trending_bias(&serde_json::json!({})), 0.0);
    }

    #[test]
    fn trending_bias_missing_metrics_field() {
        let tweet = serde_json::json!({"text": "just a tweet"});
        assert_eq!(compute_trending_bias(&tweet), 0.0);
    }

    #[test]
    fn trending_bias_only_likes_via_metrics() {
        let tweet = serde_json::json!({"metrics": {"likes": 100, "retweets": 0, "replies": 0}});
        let score = compute_trending_bias(&tweet);
        // likes=100, retweets=0, replies=0 → total=100
        // log(101)/7 ≈ 0.66
        assert!(
            (score - 0.66).abs() < 0.02,
            "Expected ~0.66 for 100 likes, got {score}"
        );
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn trending_bias_retweets_weighted_double() {
        let tweet = serde_json::json!({"metrics": {"likes": 0, "retweets": 50, "replies": 0}});
        let score = compute_trending_bias(&tweet);
        // retweets=50 * 2 = 100 → total=100 → log(101)/7 ≈ 0.66
        assert!(
            (score - 0.66).abs() < 0.02,
            "Expected ~0.66 for 50 retweets, got {score}"
        );
    }

    #[test]
    fn trending_bias_replies_weighted_one_point_five() {
        let tweet = serde_json::json!({"metrics": {"likes": 0, "retweets": 0, "replies": 100}});
        let score = compute_trending_bias(&tweet);
        // replies=100 * 1.5 = 150 → total=150 → log(151)/7 ≈ 0.72
        assert!(
            (score - 0.72).abs() < 0.02,
            "Expected ~0.72 for 100 replies, got {score}"
        );
    }

    #[test]
    fn trending_bias_fallback_to_retweet_count() {
        let tweet = serde_json::json!({
            "metrics": {"likes": 10, "replies": 5},
            "retweet_count": 25
        });
        let score = compute_trending_bias(&tweet);
        // likes=10, retweets=25 (from retweet_count), replies=5*1.5=7.5
        // total = 10 + 50 + 7.5 = 67.5 → log(68.5)/7 ≈ 0.60
        assert!(
            (score - 0.60).abs() < 0.02,
            "Expected ~0.60 with retweet_count fallback, got {score}"
        );
    }

    #[test]
    fn trending_bias_high_engagement_clamps_at_one() {
        let tweet = serde_json::json!({"metrics": {"likes": 100_000, "retweets": 50_000, "replies": 25_000}});
        let score = compute_trending_bias(&tweet);
        // very high engagement should clamp to 1.0
        assert!(
            (score - 1.0).abs() < f32::EPSILON,
            "Very high engagement should clamp to 1.0, got {score}"
        );
    }

    #[test]
    fn trending_bias_low_engagement_is_positive_but_small() {
        let tweet = serde_json::json!({"metrics": {"likes": 1, "retweets": 0, "replies": 0}});
        let score = compute_trending_bias(&tweet);
        // total=1 → log(2)/7 ≈ 0.099
        assert!(
            (score - 0.099).abs() < 0.01,
            "Expected ~0.099 for 1 like, got {score}"
        );
    }

    #[test]
    fn trending_bias_all_metrics_non_numeric_graceful() {
        let tweet =
            serde_json::json!({"metrics": {"likes": "string", "retweets": null, "replies": true}});
        // All non-numeric → defaults to 0.0 → total=0 → score=0.0
        assert_eq!(compute_trending_bias(&tweet), 0.0);
    }

    // ====================================================================
    // detect_conversation_indicators
    // ====================================================================

    #[test]
    fn detect_indicators_empty_text() {
        let result = detect_conversation_indicators("");
        assert!(result.is_empty(), "Empty text should produce no indicators");
    }

    #[test]
    fn detect_indicators_no_match() {
        let result = detect_conversation_indicators(
            "This is a completely neutral statement about the weather.",
        );
        assert!(
            result.is_empty(),
            "No matching patterns should produce empty result"
        );
    }

    #[test]
    fn detect_indicators_agreement() {
        let result = detect_conversation_indicators("I totally agree with this!");
        assert!(
            result.contains(&ConversationIndicator::Agreement),
            "Should detect Agreement"
        );
    }

    #[test]
    fn detect_indicators_disagreement() {
        let result = detect_conversation_indicators("I disagree with that point");
        assert!(
            result.contains(&ConversationIndicator::Disagreement),
            "Should detect Disagreement"
        );
    }

    #[test]
    fn detect_indicators_question_via_question_mark() {
        let result = detect_conversation_indicators("What do you think about this?");
        assert!(
            result.contains(&ConversationIndicator::Question),
            "Question mark should trigger Question indicator"
        );
    }

    #[test]
    fn detect_indicators_question_via_pattern() {
        let result = detect_conversation_indicators("how come this happened?");
        assert!(
            result.contains(&ConversationIndicator::Question),
            "Question pattern should trigger Question indicator"
        );
    }

    #[test]
    fn detect_indicators_clarification() {
        let result = detect_conversation_indicators("Let me explain what I mean");
        assert!(
            result.contains(&ConversationIndicator::Clarification),
            "Should detect Clarification"
        );
    }

    #[test]
    fn detect_indicators_humor() {
        let result = detect_conversation_indicators("lol that's hilarious");
        assert!(
            result.contains(&ConversationIndicator::Humor),
            "Should detect Humor"
        );
    }

    #[test]
    fn detect_indicators_support() {
        let result = detect_conversation_indicators("Good luck with your project!");
        assert!(
            result.contains(&ConversationIndicator::Support),
            "Should detect Support"
        );
    }

    #[test]
    fn detect_indicators_criticism() {
        let result = detect_conversation_indicators("You shouldn't do that");
        assert!(
            result.contains(&ConversationIndicator::Criticism),
            "Should detect Criticism"
        );
    }

    #[test]
    fn detect_indicators_sarcasm() {
        let result = detect_conversation_indicators("Oh sure, as if that would work");
        assert!(
            result.contains(&ConversationIndicator::Sarcasm),
            "Should detect Sarcasm"
        );
    }

    #[test]
    fn detect_indicators_multiple_matches() {
        let result =
            detect_conversation_indicators("I agree, but how come this is so expensive? lol");
        assert!(
            result.contains(&ConversationIndicator::Agreement),
            "Should detect Agreement"
        );
        assert!(
            result.contains(&ConversationIndicator::Question),
            "Should detect Question"
        );
        assert!(
            result.contains(&ConversationIndicator::Humor),
            "Should detect Humor"
        );
    }

    #[test]
    fn detect_indicators_case_insensitive() {
        let result = detect_conversation_indicators("I TOTALLY AGREE!");
        assert!(
            result.contains(&ConversationIndicator::Agreement),
            "Should detect Agreement case-insensitively"
        );
    }

    #[test]
    fn detect_indicators_all_eight_variants_are_recognized() {
        // One sentence that hits all 8 indicators
        let text = "I agree (totally agree!), but I disagree too. How come? Let me explain. Lol. I support you. That's bad. Oh sure.";
        let result = detect_conversation_indicators(text);
        assert_eq!(
            result.len(),
            8,
            "Should detect all 8 indicator types, got {}: {:?}",
            result.len(),
            result
        );
    }

    #[test]
    fn detect_indicators_exactly_absolutely_matches_agreement() {
        let result = detect_conversation_indicators("absolutely");
        assert!(result.contains(&ConversationIndicator::Agreement));
    }

    #[test]
    fn detect_indicators_exactly_doubt_it_matches_disagreement() {
        let result = detect_conversation_indicators("doubt it");
        assert!(result.contains(&ConversationIndicator::Disagreement));
    }

    #[test]
    fn detect_indicators_exactly_youre_right_matches_agreement() {
        let result = detect_conversation_indicators("you're right");
        assert!(result.contains(&ConversationIndicator::Agreement));
    }

    #[test]
    fn detect_indicators_emoji_humor() {
        let result = detect_conversation_indicators("😂");
        assert!(
            result.contains(&ConversationIndicator::Humor),
            "Laughing emoji should detect Humor"
        );
    }

    #[test]
    fn detect_indicators_keep_going_matches_support() {
        let result = detect_conversation_indicators("Keep going!");
        assert!(result.contains(&ConversationIndicator::Support));
    }

    #[test]
    fn detect_indicators_what_if_matches_question() {
        let result = detect_conversation_indicators("what if we tried something else");
        assert!(result.contains(&ConversationIndicator::Question));
    }

    #[test]
    fn detect_indicators_disappointing_matches_criticism() {
        let result = detect_conversation_indicators("disappointing result");
        assert!(result.contains(&ConversationIndicator::Criticism));
    }

    #[test]
    fn detect_indicators_oh_please_matches_sarcasm() {
        let result = detect_conversation_indicators("oh please");
        assert!(result.contains(&ConversationIndicator::Sarcasm));
    }
}
