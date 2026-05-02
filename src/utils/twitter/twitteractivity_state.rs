//! State structs and context types for Twitter activity task.
//! Includes refactored context/result types for `process_candidate()`.

use crate::config::TwitterActivityConfig;
use crate::prelude::TaskContext;
use crate::utils::twitter::{
    twitteractivity_dive::ThreadCache,
    twitteractivity_limits::{EngagementLimits, EngagementCounters},
    twitteractivity_persona::PersonaWeights,
};
use crate::utils::timing::duration_with_variance;
use log::info;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Default feed scan duration budget (ms): 5 minutes.
pub const DEFAULT_TWITTERACTIVITY_DURATION_MS: u64 = 300_000;

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
    pub fn from_payload(payload: &Value, config: &TwitterActivityConfig) -> Self {
        let duration_ms =
            duration_with_variance(read_u64(payload, "duration_ms", DEFAULT_TWITTERACTIVITY_DURATION_MS), 20);
        let candidate_count = read_u32(
            payload,
            "candidate_count",
            config.engagement_candidate_count,
        );
        let thread_depth = read_u32(payload, "thread_depth", 3);
        let max_actions_per_scan = read_u32(
            payload,
            "max_actions_per_scan",
            config.engagement_candidate_count,
        )
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

        Self {
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
        }
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
/// Groups configuration and immutable state for candidate processing.
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

/// Helper: read numeric fields from payload with defaults (u64)
pub fn read_u64(payload: &Value, key: &str, default: u64) -> u64 {
    payload.get(key).and_then(|v| v.as_u64()).unwrap_or(default)
}

/// Helper: read numeric fields from payload with defaults (u32)
pub fn read_u32(payload: &Value, key: &str, default: u32) -> u32 {
    payload
        .get(key)
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(default)
}
