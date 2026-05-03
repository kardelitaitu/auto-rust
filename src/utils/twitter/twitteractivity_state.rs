//! State structs and context types for Twitter activity task.
//! Includes refactored context/result types for `process_candidate()`.

use crate::config::TwitterActivityConfig;
use crate::prelude::TaskContext;
use crate::utils::timing::duration_with_variance;
use crate::utils::twitter::{
    twitteractivity_dive::ThreadCache,
    twitteractivity_limits::{EngagementCounters, EngagementLimits},
    twitteractivity_persona::PersonaWeights,
};
use log::info;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Validation errors for task payload.
#[derive(Debug)]
pub enum TaskValidationError {
    InvalidDuration { field: String, value: i64 },
    InvalidCandidateCount { field: String, value: i64 },
    InvalidThreadDepth { field: String, value: i64 },
    InvalidMaxActionsPerScan { field: String, value: i64 },
    InvalidPositiveNumber { field: String, value: i64 },
}

impl std::fmt::Display for TaskValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskValidationError::InvalidDuration { field, value } => write!(
                f,
                "Invalid value for '{}': {} (must be positive)",
                field, value
            ),
            TaskValidationError::InvalidCandidateCount { field, value } => {
                write!(f, "Invalid value for '{}': {} (must be u32)", field, value)
            }
            TaskValidationError::InvalidThreadDepth { field, value } => {
                write!(f, "Invalid value for '{}': {} (must be u32)", field, value)
            }
            TaskValidationError::InvalidMaxActionsPerScan { field, value } => write!(
                f,
                "Invalid value for '{}': {} (must be u32, min 1)",
                field, value
            ),
            TaskValidationError::InvalidPositiveNumber { field, value } => write!(
                f,
                "Invalid value for '{}': {} (must be positive)",
                field, value
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

/// Task configuration parsed from JSON payload.
#[derive(Debug, Clone)]
pub struct TaskConfig {
    pub duration_ms: u64,
    pub candidate_count: u32,
    pub thread_depth: u32,
    pub max_actions_per_scan: u32,
    pub weights: Option<Value>,
    pub llm_enabled: bool,
    pub smart_decision_enabled: bool,
    pub sentiment_templates: SentimentTemplates,
    pub enhanced_sentiment_enabled: bool,
    pub dry_run_actions: bool,
}

impl TaskConfig {
    /// Parse task configuration from JSON payload with defaults
    pub fn from_payload(
        payload: &Value,
        config: &TwitterActivityConfig,
    ) -> Result<Self, TaskValidationError> {
        let duration_ms = duration_with_variance(read_u64(payload, "duration_ms", 300_000)?, 20);
        let candidate_count = read_u32(
            payload,
            "candidate_count",
            config.engagement_candidate_count,
        )?;
        let thread_depth = read_u32(payload, "thread_depth", 3)?;
        let max_actions_per_scan = read_u32(
            payload,
            "max_actions_per_scan",
            config.engagement_candidate_count,
        )?
        .max(1);
        let weights = payload.get("weights").cloned();

        // Parse LLM config (V2 feature)
        let llm_enabled = payload
            .get("llm_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(config.llm.enabled);

        // Parse smart decision config (V3 feature - rule-based)
        let smart_decision_enabled = payload
            .get("smart_decision_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Sentiment templates use defaults for now
        let sentiment_templates = SentimentTemplates::default();

        // Parse enhanced sentiment config
        let enhanced_sentiment_enabled = payload
            .get("enhanced_sentiment_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true); // Enable by default for better analysis

        let dry_run_actions = payload
            .get("dry_run_actions")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(Self {
            duration_ms,
            candidate_count,
            thread_depth,
            max_actions_per_scan,
            weights,
            llm_enabled,
            smart_decision_enabled,
            sentiment_templates,
            enhanced_sentiment_enabled,
            dry_run_actions,
        })
    }
}

/// Tracks the last action type and timestamp for each tweet to prevent unrealistic action chains.
#[derive(Debug, Clone, Default)]
pub struct TweetActionTracker {
    /// Maps tweet ID to (last_action_type, timestamp)
    pub last_action: HashMap<String, (&'static str, Instant)>,
    /// Minimum delay between actions on the same tweet in milliseconds
    pub min_delay_ms: u64,
}

impl TweetActionTracker {
    pub fn new(min_delay_ms: u64) -> Self {
        Self {
            last_action: HashMap::new(),
            min_delay_ms,
        }
    }

    /// Check if an action is allowed on this tweet (prevents rapid action chains).
    pub fn can_perform_action(&self, tweet_id: &str, _action_type: &str) -> bool {
        if let Some((_, last_time)) = self.last_action.get(tweet_id) {
            let elapsed = last_time.elapsed();
            // Enforce minimum delay between actions on same tweet
            if elapsed.as_millis() < self.min_delay_ms as u128 {
                return false;
            }
        }
        true
    }

    /// Record that an action was performed on a tweet.
    pub fn record_action(&mut self, tweet_id: String, action_type: &'static str) {
        let tweet_id_for_log = tweet_id.clone();
        self.last_action
            .insert(tweet_id, (action_type, Instant::now()));
        info!(
            "Recorded {} action on tweet {} (cooldown: {}ms)",
            action_type, tweet_id_for_log, self.min_delay_ms
        );
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
    pub thread_cache: Option<ThreadCache>,
}

/// Result of processing a single candidate tweet.
/// Replaces the 5-tuple return type `(bool, Instant, u32, u32, Option<ThreadCache>)`
pub struct CandidateResult {
    pub should_break: bool,
    pub next_scroll: Instant,
    pub actions_this_scan: u32,
    pub actions_taken: u32,
    pub thread_cache: Option<ThreadCache>,
}

/// Helper: read numeric fields from payload with validation (u64)
pub fn read_u64(payload: &Value, key: &str, default: u64) -> Result<u64, TaskValidationError> {
    payload
        .get(key)
        .and_then(|v| v.as_u64())
        .map(|v| {
            if v > 0 {
                Ok(v)
            } else {
                Err(TaskValidationError::InvalidPositiveNumber {
                    field: key.to_string(),
                    value: v as i64,
                })
            }
        })
        .unwrap_or(Ok(default))
}

/// Helper: read numeric fields from payload with validation (u32)
pub fn read_u32(payload: &Value, key: &str, default: u32) -> Result<u32, TaskValidationError> {
    payload
        .get(key)
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .map(|v| {
            if v > 0 {
                Ok(v)
            } else {
                Err(TaskValidationError::InvalidPositiveNumber {
                    field: key.to_string(),
                    value: v as i64,
                })
            }
        })
        .unwrap_or(Ok(default))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_task_validation_error_display() {
        let err = TaskValidationError::InvalidDuration {
            field: "duration_ms".to_string(),
            value: -100,
        };
        let display = format!("{}", err);
        assert!(display.contains("duration_ms"));
        assert!(display.contains("must be positive"));
    }

    #[test]
    fn test_read_u64_valid() {
        let payload = json!({"duration_ms": 120000});
        let result = read_u64(&payload, "duration_ms", 300000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 120000);
    }

    #[test]
    fn test_read_u64_invalid() {
        let payload = json!({"duration_ms": -100});
        let result = read_u64(&payload, "duration_ms", 300000);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let display = format!("{}", err);
        assert!(display.contains("duration_ms"));
    }

    #[test]
    fn test_read_u64_default() {
        let payload = json!({});
        let result = read_u64(&payload, "duration_ms", 300000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 300000);
    }

    #[test]
    fn test_read_u32_valid() {
        let payload = json!({"candidate_count": 10});
        let result = read_u32(&payload, "candidate_count", 5);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn test_read_u32_invalid() {
        let payload = json!({"max_actions_per_scan": -5});
        let result = read_u32(&payload, "max_actions_per_scan", 5);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let display = format!("{}", err);
        assert!(display.contains("max_actions_per_scan"));
    }

    #[test]
    fn test_from_payload_valid() {
        let payload = json!({
            "duration_ms": 120000,
            "candidate_count": 10,
            "thread_depth": 15,
            "max_actions_per_scan": 5
        });
        let config = crate::config::TwitterActivityConfig::default();
        let result = TaskConfig::from_payload(&payload, &config);
        assert!(result.is_ok());
        let task_config = result.unwrap();
        assert_eq!(task_config.duration_ms, 120000);
        assert_eq!(task_config.candidate_count, 10);
    }

    #[test]
    fn test_from_payload_invalid_duration() {
        let payload = json!({"duration_ms": -100});
        let config = crate::config::TwitterActivityConfig::default();
        let result = TaskConfig::from_payload(&payload, &config);
        assert!(result.is_err());
    }
}
