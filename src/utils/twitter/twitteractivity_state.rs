//! State structs and context types for Twitter activity task.
//! Includes refactored context/result types for `process_candidate()`.

use crate::config::TwitterActivityConfig;
use crate::prelude::TaskContext;
use crate::utils::payload as payload_util;
use crate::utils::payload::PayloadError;
use crate::utils::timing::duration_with_variance;
use crate::utils::twitter::{
    twitteractivity_limits::{EngagementCounters, EngagementLimits},
    twitteractivity_persona::PersonaWeights,
};
use log::info;
use rand::Rng;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

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

/// Task configuration parsed from JSON payload.
#[derive(Debug, Clone, Default)]
pub struct TaskConfig {
    pub duration_ms: u64,
    pub candidate_count: u32,
    pub thread_depth: u32,
    pub max_actions_per_scan: u32,
    pub scroll_count: u32,
    pub weights: Option<Value>,
    pub llm_enabled: bool,
    pub llm_api_key: Option<String>,
    pub smart_decision_enabled: bool,
    pub sentiment_templates: SentimentTemplates,
    pub enhanced_sentiment_enabled: bool,
    pub dry_run_actions: bool,
    pub simulate_only: bool,
    pub seed: u64,
}

impl TaskConfig {
    /// Parse task configuration from JSON payload with defaults
    pub fn from_payload(
        payload: &Value,
        config: &TwitterActivityConfig,
    ) -> Result<Self, TaskValidationError> {
        let duration_ms = match read_u64(payload, "duration_ms", config.feed_scan_duration_ms)? {
            value if payload.get("duration_ms").is_some() => duration_with_variance(value, 20),
            value => value,
        };
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
        let scroll_count = read_u32(payload, "scroll_count", config.feed_scroll_count)?;
        let weights = payload.get("weights").cloned();

        // Parse LLM config (V2 feature)
        let llm_enabled = payload
            .get("llm_enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(config.llm.enabled);

        // Parse smart decision config (V3 feature - rule-based)
        let smart_decision_enabled = payload
            .get("smart_decision_enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        // Sentiment templates use defaults for now
        let sentiment_templates = SentimentTemplates::default();

        // Parse enhanced sentiment config
        let enhanced_sentiment_enabled = payload
            .get("enhanced_sentiment_enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true); // Enable by default for better analysis

        let dry_run_actions = payload
            .get("dry_run_actions")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let simulate_only = payload
            .get("simulate_only")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let llm_api_key = decision_llm_api_key();

        let seed = rand::thread_rng().gen::<u64>();

        Ok(Self {
            duration_ms,
            candidate_count,
            thread_depth,
            max_actions_per_scan,
            scroll_count,
            weights,
            llm_enabled,
            llm_api_key,
            smart_decision_enabled,
            sentiment_templates,
            enhanced_sentiment_enabled,
            dry_run_actions,
            simulate_only,
            seed,
        })
    }
}

fn decision_llm_api_key() -> Option<String> {
    std::env::var("DASHSCOPE_API_KEY")
        .or_else(|_| std::env::var("QWEN_API_KEY"))
        .ok()
}

/// Tracks the last action type and timestamp for each tweet to prevent unrealistic action chains.
#[derive(Debug, Clone, Default)]
pub struct TweetActionTracker {
    /// Maps tweet ID to (`last_action_type`, timestamp)
    pub last_action: HashMap<String, (&'static str, Instant)>,
    /// Minimum delay between actions on the same tweet in milliseconds
    pub min_delay_ms: u64,
}

impl TweetActionTracker {
    #[must_use]
    pub fn new(min_delay_ms: u64) -> Self {
        Self {
            last_action: HashMap::new(),
            min_delay_ms,
        }
    }

    /// Check if an action is allowed on this tweet (prevents rapid action chains).
    #[must_use]
    pub fn can_perform_action(&self, tweet_id: &str, _action_type: &str) -> bool {
        if let Some((_, last_time)) = self.last_action.get(tweet_id) {
            let elapsed = last_time.elapsed();
            // Enforce minimum delay between actions on same tweet
            if elapsed.as_millis() < u128::from(self.min_delay_ms) {
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
}

/// Result of processing a single candidate tweet.
/// Replaces the 5-tuple return type `(bool, Instant, u32, u32, Option<ThreadCache>)`
pub struct CandidateResult {
    pub should_break: bool,
    pub next_scroll: Instant,
    pub next_candidate_scan: Instant,
    pub actions_this_scan: u32,
}

/// Consolidated session state for Twitter activity task.
/// Groups engagement counters, limits, action tracking, and deadline into a single unit.
/// Simplifies passing state through the call chain and reduces parameter count.
#[derive(Debug)]
pub struct SessionState {
    /// Engagement action counters (likes, retweets, follows, etc.)
    pub counters: EngagementCounters,
    /// Maximum allowed actions per session
    pub limits: EngagementLimits,
    /// Tracks per-tweet action timing to prevent rapid chains
    pub action_tracker: TweetActionTracker,
    /// Session deadline for timeout checking
    pub deadline: Instant,
}

impl SessionState {
    /// Creates a new `SessionState` with the given limits and duration.
    #[must_use]
    pub fn new(limits: EngagementLimits, duration_ms: u64, min_action_delay_ms: u64) -> Self {
        Self {
            counters: EngagementCounters::new(),
            limits,
            action_tracker: TweetActionTracker::new(min_action_delay_ms),
            deadline: Instant::now() + Duration::from_millis(duration_ms),
        }
    }

    /// Checks if the session has exceeded its deadline.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.deadline
    }

    /// Returns remaining time until deadline.
    #[must_use]
    pub fn remaining_time(&self) -> Duration {
        let now = Instant::now();
        if now >= self.deadline {
            Duration::from_millis(0)
        } else {
            self.deadline.duration_since(now)
        }
    }

    /// Checks if a specific action is allowed by limits.
    #[must_use]
    pub fn is_action_allowed(&self, action: &str) -> bool {
        match action {
            "like" => self.counters.likes < self.limits.max_likes,
            "retweet" => self.counters.retweets < self.limits.max_retweets,
            "follow" => self.counters.follows < self.limits.max_follows,
            "reply" => self.counters.replies < self.limits.max_replies,
            "bookmark" => self.counters.bookmarks < self.limits.max_bookmarks,
            "quote" => self.counters.quote_tweets < self.limits.max_quote_tweets,
            "dive" => self.counters.thread_dives < self.limits.max_thread_dives,
            _ => false,
        }
    }

    /// Returns total actions taken vs max allowed.
    #[must_use]
    pub fn action_summary(&self) -> (u32, u32) {
        (self.counters.total_actions(), self.limits.max_total_actions)
    }

    /// Checks if total action limit is reached.
    #[must_use]
    pub fn is_total_limit_reached(&self) -> bool {
        self.counters.total_actions() >= self.limits.max_total_actions
    }

    /// Records an action in both counters and tracker.
    pub fn record_action(&mut self, tweet_id: &str, action_type: &'static str) {
        self.counters.increment(action_type);
        self.action_tracker
            .record_action(tweet_id.to_string(), action_type);
    }

    /// Returns a formatted summary of session progress.
    #[must_use]
    pub fn progress_summary(&self) -> String {
        format!(
            "Session: {}/{} actions | L:{}/{} R:{}/{} F:{}/{} Re:{}/{} | Time left: {:?}",
            self.counters.total_actions(),
            self.limits.max_total_actions,
            self.counters.likes,
            self.limits.max_likes,
            self.counters.retweets,
            self.limits.max_retweets,
            self.counters.follows,
            self.limits.max_follows,
            self.counters.replies,
            self.limits.max_replies,
            self.remaining_time()
        )
    }
}

/// Tracks rate-limit backoff state for session-level pacing.
///
/// When a rate-limit error is detected from Twitter/X, this records the event
/// and enforces a cooldown period before any further engagement actions
/// are attempted. The cooldown increases exponentially with consecutive
/// rate-limit hits and resets on successful actions.
#[derive(Debug, Clone)]
pub struct RateLimitBackoff {
    /// Number of consecutive rate-limit hits
    consecutive_hits: u32,
    /// System time until which backoff is active
    cooldown_until: Instant,
    /// Base delay in ms for the first backoff
    base_delay_ms: u64,
    /// Maximum delay cap to prevent unbounded waiting
    max_delay_ms: u64,
}

impl RateLimitBackoff {
    /// Create a new `RateLimitBackoff` with the given timing parameters.
    ///
    /// # Arguments
    ///
    /// * `base_delay_ms` - Delay in ms for the first rate-limit hit (doubles each consecutive hit)
    /// * `max_delay_ms` - Maximum delay cap to prevent unbounded waiting
    #[must_use]
    pub fn new(base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            consecutive_hits: 0,
            cooldown_until: Instant::now(),
            base_delay_ms,
            max_delay_ms,
        }
    }

    /// Record a rate-limit hit, extending the cooldown period.
    /// Each consecutive hit doubles the cooldown duration.
    pub fn record_rate_limit(&mut self) {
        self.consecutive_hits = self.consecutive_hits.saturating_add(1);
        let delay = self.calculate_delay();
        self.cooldown_until = Instant::now() + Duration::from_millis(delay);
    }

    /// Record a successful action, clearing the cooldown state.
    /// The consecutive-hit counter is reset so the next rate-limit starts fresh.
    pub fn record_success(&mut self) {
        self.consecutive_hits = 0;
        self.cooldown_until = Instant::now();
    }

    /// Returns `true` if we are currently in a cooldown period.
    #[must_use]
    pub fn is_in_cooldown(&self) -> bool {
        Instant::now() < self.cooldown_until
    }

    /// Returns milliseconds remaining until cooldown expires.
    /// Returns 0 if not currently in cooldown.
    #[must_use]
    pub fn remaining_cooldown_ms(&self) -> u64 {
        let now = Instant::now();
        if now < self.cooldown_until {
            self.cooldown_until.duration_since(now).as_millis() as u64
        } else {
            0
        }
    }

    /// Reset backoff to initial state, clearing all state.
    pub fn reset(&mut self) {
        self.consecutive_hits = 0;
        self.cooldown_until = Instant::now();
    }

    /// Calculate the delay for the current number of consecutive hits
    /// using exponential backoff: base * 2^(hits-1), capped at `max_delay`.
    fn calculate_delay(&self) -> u64 {
        if self.consecutive_hits == 0 {
            return 0;
        }
        // Safe exponential: base * 2^(hits-1), capped to avoid overflow
        let exponent = (self.consecutive_hits - 1).min(63);
        let multiplier = 2u64.saturating_pow(exponent);
        let delay = self.base_delay_ms.saturating_mul(multiplier);
        delay.min(self.max_delay_ms)
    }
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Helper: read numeric fields from payload with validation (u64)
pub fn read_u64(payload: &Value, key: &str, default: u64) -> Result<u64, TaskValidationError> {
    let raw = match payload_util::read_u64(payload, key) {
        Ok(v) => v,
        Err(PayloadError::Missing) => return Ok(default),
        Err(PayloadError::Invalid(_)) => {
            let kind = payload.get(key).map(value_kind).unwrap_or("unknown");
            return Err(TaskValidationError::InvalidFieldType {
                field: key.to_string(),
                expected: "positive integer",
                actual: kind,
            });
        }
    };
    if raw == 0 {
        Err(TaskValidationError::InvalidPositiveNumber {
            field: key.to_string(),
            value: 0,
        })
    } else {
        Ok(raw)
    }
}

/// Helper: read numeric fields from payload with validation (u32)
pub fn read_u32(payload: &Value, key: &str, default: u32) -> Result<u32, TaskValidationError> {
    let raw = match payload_util::read_u32(payload, key) {
        Ok(v) => v,
        Err(PayloadError::Missing) => return Ok(default),
        Err(PayloadError::Invalid(_)) => {
            let kind = payload.get(key).map(value_kind).unwrap_or("unknown");
            return Err(TaskValidationError::InvalidFieldType {
                field: key.to_string(),
                expected: "positive u32",
                actual: kind,
            });
        }
    };
    if raw == 0 {
        Err(TaskValidationError::InvalidPositiveNumber {
            field: key.to_string(),
            value: 0,
        })
    } else {
        Ok(raw)
    }
}

#[cfg(test)]
mod tdd_tests {
    use super::{RateLimitBackoff, SessionState};
    use crate::tests::twitter_helpers::*;
    use crate::utils::twitter::twitteractivity_limits::EngagementLimits;

    // ====================================================================
    // RED Tests — describe desired behavior (expected to fail on first run)
    // ====================================================================

    #[test]
    fn tdd_red_session_expiry_reports_zero_remaining() {
        // RED: A session with 0ms duration should report 0 remaining time
        // This test describes expected behavior for edge-case expiry.

        // Create a session that expired in the past
        let limits = EngagementLimits::default();
        let session = SessionState::new(limits, 0, 100);

        // Brief yield to ensure time passes
        std::thread::sleep(std::time::Duration::from_millis(1));

        // VERIFY: is_expired() returns true
        assert!(
            session.is_expired(),
            "Session with 0ms duration should be expired"
        );

        // VERIFY: remaining_time() returns 0
        assert_eq!(
            session.remaining_time().as_millis(),
            0,
            "Remaining time should be 0 for expired session"
        );
    }

    // ====================================================================
    // GREEN Tests — validate working behavior
    // ====================================================================

    #[test]
    fn tdd_green_session_progress_summary_format() {
        // GREEN: Verify progress_summary() returns expected format string
        let mut session = test_session_state();

        session.record_action("tweet_1", "like");
        let summary = session.progress_summary();

        assert!(summary.contains("1/10"), "Summary should show 1/10 actions");
        assert!(summary.contains("L:1"), "Summary should show L:1");
        assert!(
            summary.contains("Time left:"),
            "Summary should show Time left"
        );
    }

    #[test]
    fn tdd_green_session_records_multiple_action_types() {
        // GREEN: Verify all action types can be recorded
        let mut session = test_session_state_with_limits(5, 3, 2, 1, 3, 2, 2, 20, 60000);

        session.record_action("t1", "like");
        session.record_action("t2", "retweet");
        session.record_action("t3", "follow");
        session.record_action("t4", "reply");
        session.record_action("t5", "bookmark");
        session.record_action("t6", "quote");
        session.record_action("t7", "dive");

        assert_eq!(session.counters.likes, 1);
        assert_eq!(session.counters.retweets, 1);
        assert_eq!(session.counters.follows, 1);
        assert_eq!(session.counters.replies, 1);
        assert_eq!(session.counters.bookmarks, 1);
        assert_eq!(session.counters.quote_tweets, 1);
        assert_eq!(session.counters.thread_dives, 1);
        assert_eq!(session.counters.total_actions(), 7);
    }

    #[test]
    fn tdd_green_session_is_total_limit_reached_detection() {
        // GREEN: Verify is_total_limit_reached() works
        let mut session = test_session_state_with_limits(5, 3, 2, 1, 3, 2, 2, 3, 60000);

        assert!(!session.is_total_limit_reached());

        session.record_action("t1", "like");
        session.record_action("t2", "like");
        session.record_action("t3", "like");

        assert!(session.is_total_limit_reached());
    }

    #[test]
    fn tdd_green_action_tracker_cooldown_expires() {
        // GREEN: Verify cooldown expires after minimum delay
        let mut tracker = test_action_tracker(50);

        tracker.record_action("tweet_1".to_string(), "like");
        assert!(!tracker.can_perform_action("tweet_1", "retweet"));

        std::thread::sleep(std::time::Duration::from_millis(60));

        assert!(tracker.can_perform_action("tweet_1", "retweet"));
    }

    // ====================================================================
    // EDGE Case Tests
    // ====================================================================

    #[test]
    fn tdd_edge_action_tracker_unknown_tweet_allowed() {
        // EDGE: Unknown tweet should always be allowed
        let tracker = test_action_tracker(1000);
        assert!(tracker.can_perform_action("unknown_tweet", "like"));
    }

    #[test]
    fn tdd_edge_session_is_action_allowed_for_unknown_action() {
        // EDGE: Unknown action type should return false
        let session = test_session_state();
        assert!(!session.is_action_allowed("unknown_action"));
    }

    #[test]
    fn tdd_edge_session_action_summary_empty() {
        // EDGE: Empty session action summary
        let session = test_session_state();
        assert_eq!(session.action_summary(), (0, 10));
    }

    // ====================================================================
    // RED Tests for RateLimitBackoff
    // ====================================================================

    #[test]
    fn tdd_red_rate_limit_backoff_blocks_after_record() {
        // RED: After recording a rate-limit hit, backoff should block further actions
        let mut backoff = RateLimitBackoff::new(100, 5000);

        assert!(
            !backoff.is_in_cooldown(),
            "Fresh backoff should not be in cooldown"
        );

        backoff.record_rate_limit();

        assert!(
            backoff.is_in_cooldown(),
            "Backoff should block after rate-limit hit"
        );
        assert!(
            backoff.remaining_cooldown_ms() > 0,
            "Remaining cooldown should be positive"
        );
    }

    #[test]
    fn tdd_red_rate_limit_backoff_increases_with_consecutive_hits() {
        // RED: Each consecutive rate-limit hit should increase cooldown exponentially
        let mut backoff = RateLimitBackoff::new(100, 5000);

        backoff.record_rate_limit();
        let cooldown_1 = backoff.remaining_cooldown_ms();

        backoff.record_rate_limit();
        let cooldown_2 = backoff.remaining_cooldown_ms();

        backoff.record_rate_limit();
        let cooldown_3 = backoff.remaining_cooldown_ms();

        // First cooldown should be ≈ base_delay (100ms)
        assert!(
            cooldown_1 >= 90,
            "First cooldown {} should be >= 90ms (base=100ms)",
            cooldown_1
        );

        // Each subsequent cooldown should be longer (exponential: 2x, 4x)
        assert!(
            cooldown_2 > cooldown_1,
            "Second cooldown {} should be longer than first {}",
            cooldown_2,
            cooldown_1
        );
        assert!(
            cooldown_3 > cooldown_2,
            "Third cooldown {} should be longer than second {}",
            cooldown_3,
            cooldown_2
        );
    }

    #[test]
    fn tdd_red_rate_limit_backoff_success_clears_state() {
        // RED: Recording a successful action should clear cooldown state
        let mut backoff = RateLimitBackoff::new(100, 5000);

        backoff.record_rate_limit();
        assert!(backoff.is_in_cooldown(), "Should be in cooldown after hit");

        backoff.record_success();
        assert!(!backoff.is_in_cooldown(), "Success should clear cooldown");
        assert_eq!(
            backoff.remaining_cooldown_ms(),
            0,
            "Remaining cooldown should be 0 after success"
        );
    }

    // ====================================================================
    // GREEN Tests for RateLimitBackoff
    // ====================================================================

    #[test]
    fn tdd_green_rate_limit_backoff_base_delay_default() {
        // GREEN: Default RateLimitBackoff creates without error
        let backoff = RateLimitBackoff::new(1000, 30000);
        assert!(!backoff.is_in_cooldown());
        assert_eq!(backoff.remaining_cooldown_ms(), 0);
    }

    #[test]
    fn tdd_green_rate_limit_backoff_capped_at_max() {
        // GREEN: Cooldown should not exceed max_delay_ms
        // Use small base with many consecutive hits to hit the cap
        let mut backoff = RateLimitBackoff::new(10, 500);

        for _ in 0..10 {
            backoff.record_rate_limit();
        }

        let cooldown = backoff.remaining_cooldown_ms();
        assert!(
            cooldown <= 500,
            "Cooldown {} should not exceed max_delay of 500",
            cooldown
        );
    }

    #[test]
    fn tdd_green_rate_limit_backoff_reset_clears_hit_counter() {
        // GREEN: After reset, consecutive_hits should be 0
        let mut backoff = RateLimitBackoff::new(100, 5000);

        backoff.record_rate_limit();
        assert!(backoff.is_in_cooldown());

        backoff.reset();
        assert!(!backoff.is_in_cooldown());

        // After reset, cooldown delay should be back to base (not escalated)
        backoff.record_rate_limit();
        let cooldown_after_reset = backoff.remaining_cooldown_ms();
        assert!(
            cooldown_after_reset >= 90,
            "After reset, cooldown {} should be ≈ base delay (100ms)",
            cooldown_after_reset
        );
    }

    #[test]
    fn tdd_green_rate_limit_backoff_records_success_clears_consecutive_hits() {
        // GREEN: Success resets consecutive hits, so next rate-limit starts fresh
        let mut backoff = RateLimitBackoff::new(100, 5000);

        backoff.record_rate_limit(); // hit 1 → 100ms cooldown
        backoff.record_success(); // reset

        backoff.record_rate_limit(); // should be hit 1 again (100ms), not hit 2 (200ms)
        let cooldown_after_success = backoff.remaining_cooldown_ms();

        assert!(
            (90..=150).contains(&cooldown_after_success),
            "After success+re-hit, cooldown {} should be ≈ base (100ms), not escalated (200ms)",
            cooldown_after_success
        );
    }
}

#[cfg(test)]
mod test_support {
    use serde_json::{json, Value};

    pub fn twitter_config() -> crate::config::TwitterActivityConfig {
        crate::config::TwitterActivityConfig::default()
    }

    pub fn duration_payload(value: i64) -> Value {
        json!({"duration_ms": value})
    }

    pub fn candidate_count_payload(value: i64) -> Value {
        json!({"candidate_count": value})
    }

    pub fn empty_payload() -> Value {
        json!({})
    }

    pub fn full_payload() -> Value {
        json!({
            "duration_ms": 120000,
            "candidate_count": 10,
            "thread_depth": 15,
            "max_actions_per_scan": 5
        })
    }
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
mod read_u64_tests {
    use super::{read_u64, test_support::*};

    #[test]
    fn read_u64_returns_value_when_present() {
        let result = read_u64(&duration_payload(120000), "duration_ms", 300000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 120000);
    }

    #[test]
    fn read_u64_rejects_invalid() {
        let result = read_u64(&duration_payload(-100), "duration_ms", 300000);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("duration_ms"));
    }

    #[test]
    fn read_u64_defaults_when_missing() {
        let result = read_u64(&empty_payload(), "duration_ms", 300000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 300000);
    }
}

#[cfg(test)]
mod read_u32_tests {
    use super::{read_u32, test_support::candidate_count_payload};

    #[test]
    fn read_u32_returns_value_when_present() {
        let result = read_u32(&candidate_count_payload(10), "candidate_count", 5);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 10);
    }

    #[test]
    fn read_u32_rejects_invalid() {
        let result = read_u32(&candidate_count_payload(-5), "candidate_count", 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("candidate_count"));
    }
}

#[cfg(test)]
mod payload_tests {
    use super::{test_support::*, TaskConfig};
    use serde_json::json;

    #[test]
    fn from_payload_parses_core_fields() {
        let result = TaskConfig::from_payload(&full_payload(), &twitter_config());
        assert!(result.is_ok());
        let task_config = result.unwrap();
        assert!((96_000..=144_000).contains(&task_config.duration_ms));
        assert_eq!(task_config.candidate_count, 10);
        assert_eq!(task_config.thread_depth, 15);
        assert_eq!(task_config.max_actions_per_scan, 5);
        assert!(!task_config.simulate_only);
    }

    #[test]
    fn from_payload_rejects_invalid_duration() {
        let result = TaskConfig::from_payload(&duration_payload(-100), &twitter_config());
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("duration_ms"),
            "Error should mention the field name: got {err}"
        );
        assert!(
            err.contains("positive"),
            "Error should mention 'positive': got {err}"
        );
    }

    #[test]
    fn from_payload_rejects_invalid_candidate_count_type() {
        let payload = json!({"candidate_count": "ten"});
        let result = TaskConfig::from_payload(&payload, &twitter_config());
        assert!(result.is_err());
        let err = result.err().unwrap().to_string();
        assert!(err.contains("candidate_count"));
        assert_eq!(
            err,
            "Invalid value for 'candidate_count': string (must be positive u32)"
        );
    }

    #[test]
    fn from_payload_parses_simulation_fields() {
        let payload = json!({
            "simulate_only": true
        });
        let result = TaskConfig::from_payload(&payload, &twitter_config());
        assert!(result.is_ok());
        let task_config = result.unwrap();
        assert!(task_config.simulate_only);
    }
}
