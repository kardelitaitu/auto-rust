//! Twitter/X activity automation task.
//!
//! Simulates human-like engagement on a Twitter/X timeline:
//! - Navigates to home feed
//! - Scrolls through content
//! - Analyzes tweet sentiment (optional)
//! - Engages: like, retweet, follow, reply, dive into threads
//!
//! The task uses a profile-based persona to modulate interaction probabilities.
//! Payload fields control timing, engagement counts, and optional persona overrides.
//!
//! ## Payload example
//! ```json
//! {
//!   "duration_ms": 120000,
//!   "weights": {
//!     "like_prob": 0.4,
//!     "retweet_prob": 0.15,
//!     "follow_prob": 0.05,
//!     "reply_prob": 0.02,
//!     "thread_dive_prob": 0.25
//!   },
//!   "profile": "Casual"
//! }
//! ```

use anyhow::Result;
use log::{info, warn};
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// Element selectors configuration
pub const HOME_LOGO_SELECTOR: &str = r#"a[aria-label="X"]"#;
pub const TWEET_LINK_SELECTOR: &str = r#"a[href*="/status/"]"#;
pub const TWEET_DETAIL_SELECTOR: &str = r#"div[role="dialog"]"#;
pub const TWEET_DETAIL_FALLBACK1: &str = r#"div[data-testid="tweetDetail"]"#;
pub const TWEET_DETAIL_FALLBACK2: &str = r#"div[data-testid="tweetThread"]"#;
pub const TWEET_DETAIL_FALLBACK3: &str = r#"[aria-label="Timeline: Thread"]"#;
pub const TWEET_DETAIL_FALLBACK4: &str = r#"article[data-testid="tweet"]"#;
pub const RETWEET_BUTTON_SELECTOR: &str = r#"button[data-testid="retweet"]"#;
pub const RETWEET_CONFIRM_SELECTOR: &str = r#"div[data-testid="retweetConfirm"]"#;
pub const LIKE_BUTTON_SELECTOR: &str = r#"button[data-testid*="like"]"#;
pub const FOLLOW_BUTTON_SELECTOR: &str = r#"button[data-testid$="-follow"]"#;
pub const BOOKMARK_BUTTON_SELECTOR: &str = r#"button[data-testid="bookmark"]"#;

use crate::metrics::{
    RUN_COUNTER_BOOKMARK_FAILURE, RUN_COUNTER_BOOKMARK_SUCCESS, RUN_COUNTER_BUTTON_MISSING,
    RUN_COUNTER_CANDIDATE_SCANNED, RUN_COUNTER_CLICK_VERIFY_FAILED,
    RUN_COUNTER_DIVE_FAILURE, RUN_COUNTER_DIVE_SUCCESS, RUN_COUNTER_DIVE_TARGET_FALLBACK_USED,
    RUN_COUNTER_FOLLOW_FAILURE, RUN_COUNTER_FOLLOW_SUCCESS, RUN_COUNTER_LIKE_FAILURE,
    RUN_COUNTER_LIKE_SUCCESS, RUN_COUNTER_QUOTE_FAILURE, RUN_COUNTER_QUOTE_SUCCESS,
    RUN_COUNTER_REPLY_FAILURE, RUN_COUNTER_REPLY_SUCCESS, RUN_COUNTER_RETWEET_FAILURE,
    RUN_COUNTER_RETWEET_SUCCESS,
};
use crate::prelude::TaskContext;
use crate::utils::mouse::hover_before_click;
use crate::utils::twitter::{
    twitteractivity_decision::*, twitteractivity_dive::*, twitteractivity_feed::*,
    twitteractivity_humanized::*, twitteractivity_interact::*, twitteractivity_limits::*,
    twitteractivity_llm::*, twitteractivity_navigation::*, twitteractivity_persona::*,
    twitteractivity_popup::*, twitteractivity_sentiment::*,
    twitteractivity_sentiment_enhanced::{EnhancedSentimentAnalyzer, EnhancedSentimentResult, extract_thread_context, extract_user_reputation, extract_temporal_factors},
};

/// Default feed scan duration range (ms): 5-9 minutes (300-540 seconds)
fn default_duration_ms(config: &crate::config::TwitterActivityConfig) -> u64 {
    rand::random::<u64>() % (config.duration_range_max_ms - config.duration_range_min_ms) + config.duration_range_min_ms
}
/// Minimum delay between feed candidate scans (ms)
const MIN_CANDIDATE_SCAN_INTERVAL_MS: u64 = 2500;

/// Entry point for navigation (URL and weight)
struct EntryPoint {
    url: &'static str,
    weight: u32,
}

/// Weighted entry points matching Node.js implementation
const ENTRY_POINTS: [EntryPoint; 15] = [
    // Primary Entry (59%)
    EntryPoint {
        url: "https://x.com/",
        weight: 59,
    },
    // 4% Weight Group (32% total)
    EntryPoint {
        url: "https://x.com/i/jf/global-trending/home",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/explore",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/explore/tabs/for-you",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/explore/tabs/trending",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/i/bookmarks",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/notifications",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/notifications/mentions",
        weight: 4,
    },
    EntryPoint {
        url: "https://x.com/i/chat/",
        weight: 4,
    },
    // 2% Weight Group (4% total)
    EntryPoint {
        url: "https://x.com/i/connect_people?show_topics=false",
        weight: 2,
    },
    EntryPoint {
        url: "https://x.com/i/connect_people?is_creator_only=true",
        weight: 2,
    },
    // Legacy/Supplementary Exploratory Points (5% total)
    EntryPoint {
        url: "https://x.com/explore/tabs/news",
        weight: 1,
    },
    EntryPoint {
        url: "https://x.com/explore/tabs/sports",
        weight: 1,
    },
    EntryPoint {
        url: "https://x.com/explore/tabs/entertainment",
        weight: 1,
    },
    EntryPoint {
        url: "https://x.com/explore/tabs/for_you",
        weight: 1,
    },
];

/// Select a weighted entry point randomly
pub fn select_entry_point() -> &'static str {
    let total_weight: u32 = ENTRY_POINTS.iter().map(|ep| ep.weight).sum();
    let mut random = rand::random::<u32>() % total_weight;

    for entry in ENTRY_POINTS.iter() {
        if random < entry.weight {
            return entry.url;
        }
        random -= entry.weight;
    }

    ENTRY_POINTS[0].url // fallback to home
}

/// Tracks the last action type and timestamp for each tweet to prevent unrealistic action chains.
///
// This tracker prevents the bot from performing multiple actions on the same tweet
// in rapid succession, which would appear unnatural. It enforces a minimum delay
// between different action types on the same tweet.
//
// # Fields
// * `last_action` - Maps tweet ID to (last_action_type, timestamp)
// * `min_delay_ms` - Minimum delay between actions on the same tweet
//
// # Behavior
// - Actions on the same tweet must be separated by at least `min_delay_ms`
// - This prevents patterns like like→retweet→reply in quick succession
// - Each action is recorded with its timestamp for cooldown tracking
//
// # Example
/// ```rust,no_run
/// # use rust_orchestrator::task::twitteractivity::TweetActionTracker;
/// // This is a simplified example for documentation purposes
/// let mut tracker = TweetActionTracker::new(3000);
/// // In actual use, you would call can_perform_action and record_action
/// ```
#[derive(Debug, Clone, Default)]
pub struct TweetActionTracker {
    /// Maps tweet ID to (last_action_type, timestamp)
    last_action: HashMap<String, (&'static str, Instant)>,
    /// Minimum delay between actions on the same tweet in milliseconds
    min_delay_ms: u64,
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
            // Enforce minimum delay between different action types on same tweet
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

/// Navigate to weighted entry point and simulate reading if not on home.
///
/// This function implements the Node.js navigateAndRead pattern:
/// 1. Navigate to a weighted random entry point (home, explore, notifications, etc.)
/// 2. If not on home feed, simulate reading by scrolling for 10-20 seconds
/// 3. Navigate to home feed after reading simulation
///
/// # Arguments
/// * `api` - Task context for browser operations
/// * `entry_url` - The URL to navigate to
///
/// # Returns
/// Result<()> - Ok if navigation and reading simulation completed successfully
///
/// # Behavior
/// - Logs the selected entry point with emoji indicators
/// - Pauses for 2 seconds after navigation
/// - Simulates human-like reading with random scroll amounts and pauses
/// - Navigates to home feed after reading on non-home pages
async fn navigate_and_read(api: &TaskContext, entry_url: &str) -> Result<()> {
    let entry_name = entry_url
        .replace("https://x.com/", "")
        .replace("https://x.com", "");
    let entry_name = if entry_name.is_empty() {
        "home"
    } else {
        &entry_name
    };

    info!("🎲 Rolled entry point: {} → {}", entry_name, entry_url);

    // Navigate to entry point
    api.navigate(entry_url, 60000).await?;
    human_pause(api, 2000).await;

    // Check if on home feed
    let on_home = is_on_home_feed(api).await.unwrap_or(false);

    if !on_home {
        // Simulate reading on non-home page
        let scroll_duration = rand::random::<u64>() % 10000 + 10000; // 10-20s
        info!(
            "📖 Simulating reading on {} for {}s",
            entry_name,
            scroll_duration / 1000
        );

        let scroll_start = Instant::now();
        let profile = api.behavior_runtime();
        while scroll_start.elapsed().as_millis() < scroll_duration as u128 {
            let scroll_amount = (rand::random::<u64>() % 400 + 200) as i32;
            let _ = api
                .scroll_read(
                    1,
                    scroll_amount,
                    profile.scroll.smooth,
                    profile.scroll.back_scroll,
                )
                .await;
            human_pause(api, rand::random::<u64>() % 300 + 200).await;
        }

        info!("✅ Finished reading, navigating to home...");
        goto_home(api).await?;
        human_pause(api, 500).await;
    }

    Ok(())
}

/// Configuration for reply and quote text templates by sentiment.
///
/// This struct holds template strings for generating human-like engagement text
/// based on tweet sentiment analysis. Each sentiment category (positive, neutral, negative)
/// has separate templates for replies and quotes.
///
/// # Fields
/// * `reply_positive` - Template strings for positive sentiment replies
/// * `reply_neutral` - Template strings for neutral sentiment replies
/// * `reply_negative` - Template strings for negative sentiment replies
/// * `quote_positive` - Template strings for positive sentiment quotes
/// * `quote_neutral` - Template strings for neutral sentiment quotes
/// * `quote_negative` - Template strings for negative sentiment quotes
#[derive(Debug, Clone)]
struct SentimentTemplates {
    reply_positive: Vec<String>,
    reply_neutral: Vec<String>,
    reply_negative: Vec<String>,
    quote_positive: Vec<String>,
    quote_neutral: Vec<String>,
    quote_negative: Vec<String>,
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
///
/// This struct contains all configuration parameters for the Twitter activity task,
/// including timing, engagement limits, persona weights, and feature flags.
///
/// # Fields
/// * `duration_ms` - Total duration of the feed scanning phase in milliseconds
/// * `candidate_count` - Maximum number of tweet candidates to consider per scan
/// * `thread_depth` - Maximum thread depth when diving into tweet threads
/// * `max_actions_per_scan` - Maximum successful actions allowed per candidate scan iteration
/// * `weights` - Optional persona weights for engagement probability modulation
/// * `llm_enabled` - Whether LLM-powered reply/quote generation is enabled
/// * `smart_decision_enabled` - Whether rule-based smart decision filtering is enabled
/// * `sentiment_templates` - Template strings for generating engagement text by sentiment
/// * `enhanced_sentiment_enabled` - Whether to use enhanced sentiment analysis with context
///
/// # Example
/// ```json
/// {
///   "duration_ms": 120000,
///   "candidate_count": 5,
///   "thread_depth": 10,
///   "max_actions_per_scan": 3,
///   "llm_enabled": true,
///   "smart_decision_enabled": true,
///   "enhanced_sentiment_enabled": true
/// }
/// ```
#[derive(Debug, Clone)]
struct TaskConfig {
    duration_ms: u64,
    candidate_count: u32,
    thread_depth: u32,
    max_actions_per_scan: u32,
    weights: Option<Value>,
    llm_enabled: bool,
    smart_decision_enabled: bool,
    sentiment_templates: SentimentTemplates,
    enhanced_sentiment_enabled: bool,
}

impl TaskConfig {
    /// Parse task configuration from JSON payload with defaults
    fn from_payload(payload: &Value, config: &crate::config::TwitterActivityConfig) -> Self {
        let duration_ms = read_u64(payload, "duration_ms", default_duration_ms(config));
        let candidate_count = read_u32(payload, "candidate_count", config.default_candidate_count);
        let thread_depth = read_u32(payload, "thread_depth", config.default_thread_depth);
        let max_actions_per_scan = read_u32(
            payload,
            "max_actions_per_scan",
            config.default_max_actions_per_scan,
        )
        .max(1);
        let weights = payload.get("weights").cloned();

        // Parse LLM config (V2 feature)
        let llm_enabled = payload
            .get("llm_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Parse smart decision config (V3 feature - rule-based)
        let smart_decision_enabled = payload
            .get("smart_decision_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Sentiment templates use defaults for now (could be parsed from payload in future)
        let sentiment_templates = SentimentTemplates::default();

        // Parse enhanced sentiment config
        let enhanced_sentiment_enabled = payload
            .get("enhanced_sentiment_enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true); // Enable by default for better analysis

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
        }
    }
}

/// Phase 4: Final summary and cleanup.
///
/// # Arguments
/// * `counters` - Engagement counters
/// * `limits` - Engagement limits
/// * `task_config` - Task configuration
/// * `last_remaining` - Remaining time at end of loop
fn phase4_cleanup(
    counters: &EngagementCounters,
    limits: &EngagementLimits,
    task_config: &TaskConfig,
    last_remaining: Duration,
    api: &TaskContext,
) {
    let _remaining_limits = limits.remaining(counters);
    let duration_secs = (Duration::from_millis(task_config.duration_ms) - last_remaining).as_secs_f64();
    info!(
        "[twitter] Engagement summary | likes={} retweets={} follows={} replies={} thread_dives={} bookmarks={} quote_tweets={} total_actions={} duration={:.1}s",
        counters.likes,
        counters.retweets,
        counters.follows,
        counters.replies,
        counters.thread_dives,
        counters.bookmarks,
        counters.quote_tweets,
        counters.total_actions(),
        duration_secs
    );

    // Calculate and log success rates
    let like_attempts = api.run_counter(RUN_COUNTER_LIKE_SUCCESS) + api.run_counter(RUN_COUNTER_LIKE_FAILURE);
    let retweet_attempts = api.run_counter(RUN_COUNTER_RETWEET_SUCCESS) + api.run_counter(RUN_COUNTER_RETWEET_FAILURE);
    let follow_attempts = api.run_counter(RUN_COUNTER_FOLLOW_SUCCESS) + api.run_counter(RUN_COUNTER_FOLLOW_FAILURE);
    let reply_attempts = api.run_counter(RUN_COUNTER_REPLY_SUCCESS) + api.run_counter(RUN_COUNTER_REPLY_FAILURE);
    let bookmark_attempts = api.run_counter(RUN_COUNTER_BOOKMARK_SUCCESS) + api.run_counter(RUN_COUNTER_BOOKMARK_FAILURE);
    let quote_attempts = api.run_counter(RUN_COUNTER_QUOTE_SUCCESS) + api.run_counter(RUN_COUNTER_QUOTE_FAILURE);
    let dive_attempts = api.run_counter(RUN_COUNTER_DIVE_SUCCESS) + api.run_counter(RUN_COUNTER_DIVE_FAILURE);

    fn calc_rate(success: usize, total: usize) -> f64 {
        if total == 0 { 0.0 } else { (success as f64 / total as f64) * 100.0 }
    }

    info!(
        "[twitter] Success rates | like={:.1}% ({}/{}) retweet={:.1}% ({}/{}) follow={:.1}% ({}/{}) reply={:.1}% ({}/{}) bookmark={:.1}% ({}/{}) quote={:.1}% ({}/{}) dive={:.1}% ({}/{})",
        calc_rate(api.run_counter(RUN_COUNTER_LIKE_SUCCESS), like_attempts), api.run_counter(RUN_COUNTER_LIKE_SUCCESS), like_attempts,
        calc_rate(api.run_counter(RUN_COUNTER_RETWEET_SUCCESS), retweet_attempts), api.run_counter(RUN_COUNTER_RETWEET_SUCCESS), retweet_attempts,
        calc_rate(api.run_counter(RUN_COUNTER_FOLLOW_SUCCESS), follow_attempts), api.run_counter(RUN_COUNTER_FOLLOW_SUCCESS), follow_attempts,
        calc_rate(api.run_counter(RUN_COUNTER_REPLY_SUCCESS), reply_attempts), api.run_counter(RUN_COUNTER_REPLY_SUCCESS), reply_attempts,
        calc_rate(api.run_counter(RUN_COUNTER_BOOKMARK_SUCCESS), bookmark_attempts), api.run_counter(RUN_COUNTER_BOOKMARK_SUCCESS), bookmark_attempts,
        calc_rate(api.run_counter(RUN_COUNTER_QUOTE_SUCCESS), quote_attempts), api.run_counter(RUN_COUNTER_QUOTE_SUCCESS), quote_attempts,
        calc_rate(api.run_counter(RUN_COUNTER_DIVE_SUCCESS), dive_attempts), api.run_counter(RUN_COUNTER_DIVE_SUCCESS), dive_attempts
    );
}

/// Phase 1: Navigation and authentication setup.
///
/// # Arguments
/// * `api` - Task context
///
/// # Returns
/// Result<()> - Ok if navigation and setup completed successfully
async fn phase1_navigation(api: &TaskContext) -> Result<()> {
    info!("Phase 1: Navigation to entry point");
    let entry_url = select_entry_point();
    navigate_and_read(api, entry_url).await?;

    if verify_login(api).await? {
        info!("User is logged in - proceeding");
    } else {
        warn!("User appears not logged in; task may fail");
    }

    // Dismiss initial popups
    match dismiss_cookie_banner(api).await {
        Ok(true) => info!("Cookie banner dismissed"),
        Ok(false) => {}
        Err(e) => warn!("Cookie banner dismissal failed: {}", e),
    }
    match dismiss_signup_nag(api).await {
        Ok(true) => info!("Signup nag dismissed"),
        Ok(false) => {}
        Err(e) => warn!("Signup nag dismissal failed: {}", e),
    }
    if let Err(e) = close_active_popup(api).await {
        warn!("Popup close failed: {}", e);
    }

    Ok(())
}

/// Handle smart decision check for engagement.
///
/// # Arguments
/// * `tweet` - Tweet data from candidate scan
/// * `task_config` - Task configuration
///
/// # Returns
/// Option<EngagementDecision> - None if smart decision disabled or should skip, Some(decision) otherwise
fn handle_engagement_decision(tweet: &Value, task_config: &TaskConfig) -> Option<EngagementDecision> {
    if !task_config.smart_decision_enabled {
        return None;
    }

    // Extract tweet text (already extracted by identify_engagement_candidates)
    let tweet_text = tweet.get("text").and_then(|v| v.as_str()).unwrap_or("");

    // Extract replies from tweet data (already parsed by identify_engagement_candidates)
    let mut replies: Vec<(String, String)> = Vec::new();
    if let Some(replies_array) = tweet.get("replies").and_then(|v| v.as_array()) {
        for reply_value in replies_array {
            if let Some(reply_obj) = reply_value.as_object() {
                if let (Some(author_value), Some(text_value)) =
                    (reply_obj.get("author"), reply_obj.get("text"))
                {
                    if let (Some(author_str), Some(text_str)) =
                        (author_value.as_str(), text_value.as_str())
                    {
                        replies
                            .push((author_str.to_string(), text_str.to_string()));
                    }
                }
            }
        }
    }

    info!(
        "Smart decision: tweet='{}' ({} chars), replies={}",
        tweet_text,
        tweet_text.len(),
        replies.len()
    );

    Some(decide_engagement(tweet_text, &replies))
}

/// Process a single candidate tweet for engagement.
///
/// # Arguments
/// * `tweet` - Tweet data from candidate scan
/// * `persona` - Base persona weights
/// * `task_config` - Task configuration
/// * `api` - Task context
/// * `limits` - Engagement limits
/// * `scroll_interval` - Scroll interval for resuming after dive
/// * `action_tracker` - Action tracking state (mutable)
/// * `counters` - Engagement counters (mutable)
/// * `current_thread_cache` - Thread cache from previous dive (mutable)
/// * `actions_this_scan` - Actions taken in current scan
/// * `next_scroll` - Next scroll time (mutable)
/// * `_actions_taken` - Total actions taken (mutable)
///
/// # Returns
/// Result<(bool, Instant, u32, u32, Option<ThreadCache>)> - (should_break, next_scroll, actions_this_scan, _actions_taken, thread_cache)
async fn process_candidate(
    tweet: &Value,
    persona: &PersonaWeights,
    task_config: &TaskConfig,
    api: &TaskContext,
    limits: &EngagementLimits,
    scroll_interval: Duration,
    action_tracker: &mut TweetActionTracker,
    counters: &mut EngagementCounters,
    // NOTE: current_thread_cache is intentionally NOT used here to avoid cross-tweet contamination.
    // Each tweet should start with a fresh cache. The cache is only populated if THIS tweet
    // performs a dive, and is only valid for subsequent quote/reply actions on THIS same tweet.
    _current_thread_cache_unused: Option<ThreadCache>,
    actions_this_scan: u32,
    next_scroll: Instant,
    _actions_taken: u32,
) -> Result<(bool, Instant, u32, u32, Option<ThreadCache>)> {
    let mut actions_this_scan = actions_this_scan;
    let mut next_scroll = next_scroll;
    let mut _actions_taken = _actions_taken;
    // Each tweet starts with a fresh cache. Only populated if this tweet dives.
    let mut current_thread_cache: Option<ThreadCache> = None;

    if actions_this_scan >= task_config.max_actions_per_scan {
        info!(
            "Per-scan action budget reached ({}/{}), deferring remaining candidates",
            actions_this_scan, task_config.max_actions_per_scan
        );
        return Ok((true, next_scroll, actions_this_scan, _actions_taken, current_thread_cache));
    }

    // Analyze sentiment with enhanced context when enabled
    let sentiment_result = if task_config.enhanced_sentiment_enabled {
        let analyzer = EnhancedSentimentAnalyzer::default();
        let tweet_text = extract_tweet_text(tweet);
        let thread_context = extract_thread_context(tweet);
        let user_reputation = extract_user_reputation(tweet);
        let temporal_factors = extract_temporal_factors(tweet);

        analyzer.analyze_enhanced(
            &tweet_text,
            thread_context.as_ref(),
            user_reputation.as_ref(),
            temporal_factors.as_ref(),
        )
    } else {
        // Fallback to basic sentiment analysis
        let sentiment = analyze_tweet_sentiment(tweet);
        EnhancedSentimentResult {
            base_sentiment: sentiment,
            final_sentiment: sentiment,
            base_score: sentiment_score(sentiment) as f32,
            final_score: sentiment_score(sentiment) as f32,
            confidence: 0.7, // Default confidence for basic analysis
            score_breakdown: crate::utils::twitter::twitteractivity_sentiment_enhanced::ScoreBreakdown {
                text_score: sentiment_score(sentiment) as f32,
                emoji_score: 0.0,
                domain_score: 0.0,
                context_score: 0.0,
                reputation_score: 0.0,
                temporal_score: 0.0,
            },
        }
    };

    let sentiment = sentiment_result.final_sentiment;
    let mut candidate_persona = persona.clone();
    // Modulate weights by sentiment with enhanced scoring
    candidate_persona.interest_multiplier = match sentiment {
        Sentiment::Negative => 0.3, // suppress engagement on negative tweets
        Sentiment::Positive => 1.4, // boost positive (slightly more than basic)
        Sentiment::Neutral => 1.0,
    };

    // Additional modulation based on confidence
    if sentiment_result.confidence > 0.8 {
        // High confidence - amplify the effect
        candidate_persona.interest_multiplier *= 1.1;
    } else if sentiment_result.confidence < 0.5 {
        // Low confidence - reduce the effect
        candidate_persona.interest_multiplier *= 0.9;
    }

    // Smart decision check (V3 feature - rule-based)
    let engagement_decision = handle_engagement_decision(tweet, task_config);

    // Skip if smart decision says None
    if let Some(ref decision) = engagement_decision {
        if decision.level == EngagementLevel::None {
            info!(
                "Skipping engagement: {} (score: {})",
                decision.reason, decision.score
            );
            return Ok((false, next_scroll, actions_this_scan, _actions_taken, current_thread_cache));
        }
    }

    // Determine actions to perform on this tweet
    let mut actions_to_do: Vec<&str> = Vec::new();
    let tweet_id = tweet
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Check each action type, respecting limits and chaining
    if should_like(&candidate_persona)
        && action_tracker.can_perform_action(tweet_id, "like")
        && limits.can_like(counters)
    {
        actions_to_do.push("like");
    }
    if should_retweet(&candidate_persona)
        && action_tracker.can_perform_action(tweet_id, "retweet")
        && limits.can_retweet(counters)
    {
        actions_to_do.push("retweet");
    }
    if should_quote(&candidate_persona)
        && action_tracker.can_perform_action(tweet_id, "quote_tweet")
        && limits.can_quote_tweet(counters)
    {
        actions_to_do.push("quote");
    }
    if should_follow(&candidate_persona)
        && action_tracker.can_perform_action(tweet_id, "follow")
        && limits.can_follow(counters)
    {
        actions_to_do.push("follow");
    }
    if should_reply(&candidate_persona)
        && action_tracker.can_perform_action(tweet_id, "reply")
        && limits.can_reply(counters)
    {
        actions_to_do.push("reply");
    }
    if should_bookmark(&candidate_persona)
        && action_tracker.can_perform_action(tweet_id, "bookmark")
        && limits.can_bookmark(counters)
    {
        actions_to_do.push("bookmark");
    }

    // Determine if we need to dive (retweet, quote, reply, follow, bookmark require dive; like does not)
    let need_dive = actions_to_do.iter().any(|&action| action != "like");
    let mut did_dive = false;

    if need_dive {
        // Dive into thread for non-like actions
        if actions_this_scan >= task_config.max_actions_per_scan {
            return Ok((true, next_scroll, actions_this_scan, _actions_taken, current_thread_cache));
        }
        if !limits.can_dive(counters) {
            info!(
                "Skipping dive: limit reached ({}/{})",
                counters.thread_dives, limits.max_thread_dives
            );
        } else if let Some(status_url) = tweet.get("status_url").and_then(|v| v.as_str()) {
            // Pause continuous scrolling before diving to avoid interference
            let original_next_scroll = next_scroll;
            next_scroll = Instant::now() + Duration::from_secs(300); // Pause for 5 minutes during dive
            info!("Paused continuous scrolling for thread dive");

            let dive_outcome = dive_into_thread(api, status_url).await?;
            if dive_outcome.used_fallback_target {
                api.increment_run_counter(RUN_COUNTER_DIVE_TARGET_FALLBACK_USED, 1);
            }
            if !dive_outcome.opened {
                info!("Thread dive failed: no valid target resolved");
                // Resume scrolling if dive failed
                next_scroll = original_next_scroll;
                api.increment_run_counter(RUN_COUNTER_DIVE_FAILURE, 1);
            } else {
                api.increment_run_counter(RUN_COUNTER_DIVE_SUCCESS, 1);
                // Use cache from dive outcome, initialize empty if none
                let mut thread_cache = dive_outcome.cache.unwrap_or_default();
                read_full_thread(api, task_config.thread_depth, &mut thread_cache).await?;
                scroll_pause(api).await;
                counters.increment_thread_dive();
                _actions_taken += 1;
                actions_this_scan += 1;
                // Record dive action
                action_tracker.record_action(tweet_id.to_string(), "dive");
                did_dive = true;
                // Store cache for later LLM use
                current_thread_cache = Some(thread_cache);
            }
        }
    }

    // Perform actions (only 1 per dive)
    for &action in actions_to_do.iter().take(1) {
        let success = match action {
            "like" => {
                // Like can be done on feed or in detail
                if did_dive {
                    // In detail view, use general like function
                    like_tweet(api).await?
                } else {
                    // On feed, use position from tweet data
                    if let Some(btn_pos) = extract_tweet_button_position(tweet, "like") {
                        like_at_position(api, btn_pos.0, btn_pos.1).await?
                    } else {
                        warn!("Like button not found in tweet payload for {}", tweet_id);
                        api.increment_run_counter(RUN_COUNTER_BUTTON_MISSING, 1);
                        false
                    }
                }
            }
            "retweet" => {
                // Validate we're in thread detail view before retweeting
                if !did_dive {
                    warn!("Skipping retweet: not in thread detail view for tweet {}", tweet_id);
                    false
                } else {
                    match crate::utils::twitter::twitteractivity_interact::is_on_tweet_page(api).await {
                        Ok(true) => retweet_tweet(api).await?,
                        Ok(false) => {
                            warn!("Skipping retweet: not on tweet page for tweet {}", tweet_id);
                            false
                        }
                        Err(e) => {
                            warn!("Failed to validate tweet page context for retweet: {}", e);
                            false
                        }
                    }
                }
            }
            "quote" => {
                // Validate we're in thread detail view before quoting
                if !did_dive {
                    warn!("Skipping quote: not in thread detail view for tweet {}", tweet_id);
                    false
                } else {
                    match crate::utils::twitter::twitteractivity_interact::is_on_tweet_page(api).await {
                        Ok(true) => {
                            let quote_text = if task_config.llm_enabled {
                                // Use cached data if available, fallback to extraction
                                let (author, text, replies) = if let Some(ref cache) = current_thread_cache {
                                    if cache.is_valid() {
                                        info!("Using cached thread data for quote ({} replies)", cache.replies.len());
                                        (cache.tweet_author.clone(), cache.tweet_text.clone(), cache.replies.clone())
                                    } else {
                                        match extract_tweet_context(api).await {
                                            Ok(data) => data,
                                            Err(e) => {
                                                warn!("Failed to extract tweet context for quote: {}", e);
                                                ("unknown".to_string(), String::new(), Vec::new())
                                            }
                                        }
                                    }
                                } else {
                                    match extract_tweet_context(api).await {
                                        Ok(data) => data,
                                        Err(e) => {
                                            warn!("Failed to extract tweet context for quote: {}", e);
                                            ("unknown".to_string(), String::new(), Vec::new())
                                        }
                                    }
                                };
                                match generate_quote_commentary(api, &author, &text, replies).await {
                                    Ok(commentary) => {
                                        info!("Generated LLM quote: {}", commentary);
                                        commentary
                                    }
                                    Err(e) => {
                                        warn!("LLM quote failed, using template: {}", e);
                                        generate_quote_text(sentiment, counters.quote_tweets, &task_config.sentiment_templates)
                                    }
                                }
                            } else {
                                generate_quote_text(sentiment, counters.quote_tweets, &task_config.sentiment_templates)
                            };
                            match quote_tweet(api, &quote_text).await {
                                Ok(success) => {
                                    if success {
                                        info!("Quote tweeted with commentary: {}", quote_text);
                                    }
                                    success
                                }
                                Err(e) => {
                                    warn!("Quote tweet error: {}", e);
                                    false
                                }
                            }
                        }
                        Ok(false) => {
                            warn!("Skipping quote: not on tweet page for tweet {}", tweet_id);
                            false
                        }
                        Err(e) => {
                            warn!("Failed to validate tweet page context for quote: {}", e);
                            false
                        }
                    }
                }
            }
            "follow" => {
                // Validate we're in thread detail view before following
                if !did_dive {
                    warn!("Skipping follow: not in thread detail view for tweet {}", tweet_id);
                    false
                } else {
                    match crate::utils::twitter::twitteractivity_interact::is_on_tweet_page(api).await {
                        Ok(true) => follow_from_tweet(api).await?,
                        Ok(false) => {
                            warn!("Skipping follow: not on tweet page for tweet {}", tweet_id);
                            false
                        }
                        Err(e) => {
                            warn!("Failed to validate tweet page context for follow: {}", e);
                            false
                        }
                    }
                }
            }
            "reply" => {
                // Validate we're in thread detail view before replying
                if !did_dive {
                    warn!("Skipping reply: not in thread detail view for tweet {}", tweet_id);
                    false
                } else {
                    match crate::utils::twitter::twitteractivity_interact::is_on_tweet_page(api).await {
                        Ok(true) => {
                            let reply_text = if task_config.llm_enabled {
                                // Use cached data if available, fallback to extraction
                                let (author, text, replies) = if let Some(ref cache) = current_thread_cache {
                                    if cache.is_valid() {
                                        info!("Using cached thread data for reply ({} replies)", cache.replies.len());
                                        (cache.tweet_author.clone(), cache.tweet_text.clone(), cache.replies.clone())
                                    } else {
                                        match extract_tweet_context(api).await {
                                            Ok(data) => data,
                                            Err(e) => {
                                                warn!("Failed to extract tweet context for reply: {}", e);
                                                ("unknown".to_string(), String::new(), Vec::new())
                                            }
                                        }
                                    }
                                } else {
                                    match extract_tweet_context(api).await {
                                        Ok(data) => data,
                                        Err(e) => {
                                            warn!("Failed to extract tweet context for reply: {}", e);
                                            ("unknown".to_string(), String::new(), Vec::new())
                                        }
                                    }
                                };
                                match generate_reply(api, &author, &text, replies).await {
                                    Ok(reply) => {
                                        info!("Generated LLM reply: {}", reply);
                                        reply
                                    }
                                    Err(e) => {
                                        warn!("LLM reply failed, using template: {}", e);
                                        generate_reply_text(sentiment, counters.replies, &task_config.sentiment_templates)
                                    }
                                }
                            } else {
                                generate_reply_text(sentiment, counters.replies, &task_config.sentiment_templates)
                            };
                            reply_to_tweet(api, &reply_text).await?
                        }
                        Ok(false) => {
                            warn!("Skipping reply: not on tweet page for tweet {}", tweet_id);
                            false
                        }
                        Err(e) => {
                            warn!("Failed to validate tweet page context for reply: {}", e);
                            false
                        }
                    }
                }
            }
            "bookmark" => {
                // Validate we're in thread detail view before bookmarking
                if !did_dive {
                    warn!("Skipping bookmark: not in thread detail view for tweet {}", tweet_id);
                    false
                } else {
                    match crate::utils::twitter::twitteractivity_interact::is_on_tweet_page(api).await {
                        Ok(true) => bookmark_tweet(api).await?,
                        Ok(false) => {
                            warn!("Skipping bookmark: not on tweet page for tweet {}", tweet_id);
                            false
                        }
                        Err(e) => {
                            warn!("Failed to validate tweet page context for bookmark: {}", e);
                            false
                        }
                    }
                }
            }
            _ => false,
        };

        if success {
            // Update counters and record action
            match action {
                "like" => {
                    info!("Liked tweet");
                    counters.increment_like();
                    api.increment_run_counter(RUN_COUNTER_LIKE_SUCCESS, 1);
                }
                "retweet" => {
                    info!("Retweeted");
                    counters.increment_retweet();
                    api.increment_run_counter(RUN_COUNTER_RETWEET_SUCCESS, 1);
                }
                "quote" => {
                    counters.increment_quote_tweet();
                    api.increment_run_counter(RUN_COUNTER_QUOTE_SUCCESS, 1);
                }
                "follow" => {
                    info!("Followed user");
                    counters.increment_follow();
                    api.increment_run_counter(RUN_COUNTER_FOLLOW_SUCCESS, 1);
                }
                "reply" => {
                    info!("Replied with sentiment {:?}", sentiment);
                    counters.increment_reply();
                    api.increment_run_counter(RUN_COUNTER_REPLY_SUCCESS, 1);
                }
                "bookmark" => {
                    info!("Bookmarked tweet");
                    counters.increment_bookmark();
                    api.increment_run_counter(RUN_COUNTER_BOOKMARK_SUCCESS, 1);
                }
                _ => {}
            }
            _actions_taken += 1;
            actions_this_scan += 1;
            action_tracker.record_action(tweet_id.to_string(), action);

            // Use appropriate pause
            if action == "reply" || action == "quote" {
                clustered_reply_pause(api).await;
            } else {
                clustered_engagement_pause(api).await;
            }
        } else {
            match action {
                "like" => {
                    api.increment_run_counter(RUN_COUNTER_CLICK_VERIFY_FAILED, 1);
                    api.increment_run_counter(RUN_COUNTER_LIKE_FAILURE, 1);
                }
                "retweet" => api.increment_run_counter(RUN_COUNTER_RETWEET_FAILURE, 1),
                "quote" => api.increment_run_counter(RUN_COUNTER_QUOTE_FAILURE, 1),
                "follow" => api.increment_run_counter(RUN_COUNTER_FOLLOW_FAILURE, 1),
                "reply" => api.increment_run_counter(RUN_COUNTER_REPLY_FAILURE, 1),
                "bookmark" => api.increment_run_counter(RUN_COUNTER_BOOKMARK_FAILURE, 1),
                _ => {}
            }
        }
    }

    // Navigate back to home after dive
    if did_dive {
        // Wait 3-5s after engagement before going home
        let home_wait_ms = rand::random::<u64>() % 2000 + 3000; // 3-5s
        human_pause(api, home_wait_ms).await;
        info!("Navigating back to home after thread dive and engagement");
        goto_home(api).await?;
        scroll_pause(api).await;
        // Resume continuous scrolling now that we're back on home feed
        next_scroll = Instant::now() + scroll_interval;
        info!("Resumed continuous scrolling after thread dive");
    }

    Ok((false, next_scroll, actions_this_scan, _actions_taken, current_thread_cache))
}

/// Task entry point called by orchestrator.
///
/// # Arguments
/// * `api` - Task context with page, profile, clipboard
/// * `payload` - JSON task configuration (see module docs)
/// * `config` - Application configuration for default probabilities
///
/// # Returns
/// Result<()> - Ok if completed successfully, Err on failure
pub async fn run(api: &TaskContext, payload: Value, config: &crate::config::Config) -> Result<()> {
    info!("Task started");

    // Parse task configuration from payload
    let task_config = TaskConfig::from_payload(&payload, &config.twitter_activity);

    // Build persona weights
    let mut persona = select_persona_weights(task_config.weights.as_ref(), &config.twitter_activity.probabilities);
    let profile = api.behavior_profile();

    persona = apply_behavior_profile(persona, profile, 0.0);

    info!(
        "Persona weights: like={:.2}, rt={:.2}, follow={:.2}, reply={:.2}",
        persona.like_prob, persona.retweet_prob, persona.follow_prob, persona.reply_prob
    );

    // Initialize engagement counters and limits
    let mut counters = EngagementCounters::new();
    let limits = EngagementLimits::default();
    let mut action_tracker = TweetActionTracker::new(config.twitter_activity.min_action_chain_delay_ms);
    let _current_thread_cache: Option<crate::utils::twitter::twitteractivity_dive::ThreadCache> = None;

    info!(
        "Engagement limits: likes={}/{}, retweets={}/{}, follows={}/{}, total={}/{}",
        counters.likes,
        limits.max_likes,
        counters.retweets,
        limits.max_retweets,
        counters.follows,
        limits.max_follows,
        counters.total_actions(),
        limits.max_total_actions
    );
    info!(
        "Per-scan action budget: {} successful actions",
        task_config.max_actions_per_scan
    );

    // Phase 1: Navigation & authentication check
    phase1_navigation(api).await?;

    // Phase 2: Feed analysis & scrolling
    info!("Phase 2: Scanning feed for {} ms", task_config.duration_ms);
    let deadline = Instant::now() + Duration::from_millis(task_config.duration_ms);
    let mut _actions_taken = 0u32;
    let mut last_remaining = Duration::from_millis(task_config.duration_ms);

    // Continuous scrolling like pageview
    let profile = api.behavior_runtime();
    let scroll_amount = if config.twitter_activity.scroll_amount_pixels > 0 {
        config.twitter_activity.scroll_amount_pixels
    } else {
        profile.scroll.amount
    };
    let scroll_pause_ms = profile.scroll.pause_ms;
    let smooth = profile.scroll.smooth;
    let scroll_interval = Duration::from_millis(scroll_pause_ms);
    let candidate_scan_interval = if config.twitter_activity.candidate_scan_interval_ms > 0 {
        Duration::from_millis(config.twitter_activity.candidate_scan_interval_ms)
    } else {
        Duration::from_millis(MIN_CANDIDATE_SCAN_INTERVAL_MS)
    };
    let mut next_scroll = Instant::now();
    let mut next_candidate_scan = Instant::now();

    // Session-level error recovery (following Node.js pattern)
    while Instant::now() < deadline {
        let now = Instant::now();

        if now < next_candidate_scan {
            tokio::time::sleep(next_candidate_scan - now).await;
            continue;
        }

        // Scroll to load new content (interval-based like pageview)
        if now >= next_scroll {
            let _ = api
                .scroll_read(
                    1, // single pause per scroll
                    scroll_amount,
                    smooth,
                    profile.scroll.back_scroll,
                )
                .await;
            next_scroll = now + scroll_interval;
        }

        // Check for popups periodically
        if !ensure_feed_populated(api).await? {
            warn!("Feed appears empty after scroll");
        }

        // Identify candidate tweets
        let scan_started = Instant::now();
        let candidates = identify_engagement_candidates(api).await?;
        api.increment_run_counter(RUN_COUNTER_CANDIDATE_SCANNED, candidates.len());
        info!(
            "candidate_scan | candidates={} duration_ms={}",
            candidates.len(),
            scan_started.elapsed().as_millis()
        );
        next_candidate_scan = scan_started + candidate_scan_interval;

        if !candidates.is_empty() {
            let to_consider = candidates
                .iter()
                .take(task_config.candidate_count as usize)
                .collect::<Vec<_>>();
            let mut actions_this_scan = 0u32;

            for tweet in to_consider {
                let (should_break, new_next_scroll, new_actions_this_scan, new_actions_taken, _new_thread_cache) = process_candidate(
                    tweet,
                    &persona,
                    &task_config,
                    api,
                    &limits,
                    scroll_interval,
                    &mut action_tracker,
                    &mut counters,
                    None,  // Each tweet starts with fresh cache (no cross-tweet contamination)
                    actions_this_scan,
                    next_scroll,
                    _actions_taken,
                ).await?;
                
                next_scroll = new_next_scroll;
                actions_this_scan = new_actions_this_scan;
                _actions_taken = new_actions_taken;
                // new_thread_cache is only populated if THIS tweet did a dive.
                // It's not used for subsequent tweets (each starts fresh).
                // We discard it here to prevent accidental reuse.
                // If we wanted to use it, we'd need to track it per-tweet, not globally.
                // current_thread_cache = new_thread_cache;  // REMOVED - prevents cross-tweet contamination
                
                if should_break {
                    break;
                }
            }
        }

        // Time check at end of loop
        last_remaining = deadline.saturating_duration_since(Instant::now());
        if last_remaining.as_millis() < 500 {
            break;
        }
    }

    // Final summary with engagement counters
    phase4_cleanup(&counters, &limits, &task_config, last_remaining, api);

    Ok(())
}

// Helper: read numeric fields from payload with defaults
fn read_u64(payload: &Value, key: &str, default: u64) -> u64 {
    payload.get(key).and_then(|v| v.as_u64()).unwrap_or(default)
}

fn read_u32(payload: &Value, key: &str, default: u32) -> u32 {
    payload
        .get(key)
        .and_then(|v| v.as_u64())
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(default)
}

// Helper: extract tweet text from tweet object
fn extract_tweet_text(tweet_obj: &Value) -> String {
    if let Some(text) = tweet_obj.get("text").or_else(|| tweet_obj.get("full_text")) {
        if let Some(text_str) = text.as_str() {
            return text_str.to_string();
        }
    }
    String::new()
}

// Helper: extract a per-tweet button center from candidate payload.
fn extract_tweet_button_position(tweet: &Value, button: &str) -> Option<(f64, f64)> {
    let button_obj = tweet
        .get("buttons")
        .and_then(|v| v.as_object())
        .and_then(|buttons| buttons.get(button))
        .and_then(|v| v.as_object())?;

    let x = button_obj.get("x").and_then(|v| v.as_f64())?;
    let y = button_obj.get("y").and_then(|v| v.as_f64())?;
    Some((x, y))
}

// Helper: click like at a specific coordinate with profile-aware timing and hover
async fn like_at_position(api: &TaskContext, x: f64, y: f64) -> Result<bool> {
    let page = api.page();
    let element_type = "button";
    hover_before_click(page, "", x, y, element_type).await?;
    click_prep_pause(api).await;
    api.click_at(x, y).await?;
    click_post_pause(api).await;

    // Verify like was registered by checking if button state changed
    let verify_js = format!(
        r#"
        (function() {{
            var x = {x};
            var y = {y};
            var controls = document.querySelectorAll('button[data-testid], a[data-testid]');
            var nearest = null;
            var best = Number.POSITIVE_INFINITY;
            for (var i = 0; i < controls.length; i++) {{
                var el = controls[i];
                var testId = (el.getAttribute('data-testid') || '').toLowerCase();
                if (!(testId.includes('like') || testId.includes('unlike'))) continue;
                var rect = el.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) continue;
                var cx = rect.x + rect.width / 2;
                var cy = rect.y + rect.height / 2;
                var dist = Math.hypot(cx - x, cy - y);
                if (dist < best) {{
                    best = dist;
                    nearest = el;
                }}
            }}

            if (!nearest || best > 120) return false;
            var nearestId = (nearest.getAttribute('data-testid') || '').toLowerCase();
            if (nearestId.includes('unlike')) return true;

            var svg = nearest.querySelector('svg');
            if (!svg) return false;
            var color = (svg.getAttribute('color') || svg.getAttribute('fill') || '').toLowerCase();
            return color.includes('rgb') || color.includes('#');
        }})()
        "#,
        x = x,
        y = y
    );

    let result = page.evaluate(verify_js).await?;

    let value = result.value();
    if let Some(v) = value {
        if let Some(liked) = v.as_bool() {
            return Ok(liked);
        }
    }

    Ok(true)
}

/// Generate a short reply string based on sentiment.
///
/// This function selects a reply template from the sentiment templates based on
/// the detected sentiment of the tweet. The reply index is used to cycle through
/// templates deterministically, allowing for variety while maintaining consistency.
///
/// # Arguments
/// * `sentiment` - The detected sentiment of the tweet (Positive, Neutral, Negative)
/// * `reply_idx` - Index for cycling through templates (typically reply count)
/// * `templates` - Sentiment templates containing reply strings
///
/// # Returns
/// String - A reply template string appropriate for the sentiment
///
/// # Behavior
/// - Uses modulo arithmetic to cycle through templates
/// - Positive sentiment → enthusiastic agreement templates
/// - Neutral sentiment → brief acknowledgment templates
/// - Negative sentiment → respectful disagreement templates
fn generate_reply_text(sentiment: Sentiment, reply_idx: u32, templates: &SentimentTemplates) -> String {
    let phrases = match sentiment {
        Sentiment::Positive => &templates.reply_positive,
        Sentiment::Neutral => &templates.reply_neutral,
        Sentiment::Negative => &templates.reply_negative,
    };
    phrases[(reply_idx as usize) % phrases.len()].clone()
}

/// Generate a short quote commentary string based on sentiment.
///
/// This function selects a quote template from the sentiment templates based on
/// the detected sentiment of the tweet. The quote index is used to cycle through
/// templates deterministically, allowing for variety while maintaining consistency.
///
/// # Arguments
/// * `sentiment` - The detected sentiment of the tweet (Positive, Neutral, Negative)
/// * `quote_idx` - Index for cycling through templates (typically quote count)
/// * `templates` - Sentiment templates containing quote strings
///
/// # Returns
/// String - A quote template string appropriate for the sentiment
///
/// # Behavior
/// - Uses modulo arithmetic to cycle through templates
/// - Positive sentiment → enthusiastic sharing templates
/// - Neutral sentiment → informative sharing templates
/// - Negative sentiment → thought-provoking sharing templates
fn generate_quote_text(sentiment: Sentiment, quote_idx: u32, templates: &SentimentTemplates) -> String {
    let phrases = match sentiment {
        Sentiment::Positive => &templates.quote_positive,
        Sentiment::Neutral => &templates.quote_neutral,
        Sentiment::Negative => &templates.quote_negative,
    };
    phrases[(quote_idx as usize) % phrases.len()].clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_read_u64_valid() {
        let payload = json!({ "duration_seconds": 123 });
        let result = read_u64(&payload, "duration_seconds", 0);
        assert_eq!(result, 123);
    }

    #[test]
    fn test_read_u64_missing() {
        let payload = json!({});
        let result = read_u64(&payload, "duration_seconds", 60);
        assert_eq!(result, 60); // default used
    }

    #[test]
    fn test_read_u64_wrong_type() {
        let payload = json!({ "duration_seconds": "not a number" });
        let result = read_u64(&payload, "duration_seconds", 0);
        // When type is wrong, as_u64 returns None, falls back to default
        assert_eq!(result, 0);
    }

    #[test]
    fn test_read_u32_valid() {
        let payload = json!({ "max_actions": 42 });
        let result = read_u32(&payload, "max_actions", 5);
        assert_eq!(result, 42);
    }

    #[test]
    fn test_read_u32_missing() {
        let payload = json!({});
        let result = read_u32(&payload, "max_actions", 5);
        assert_eq!(result, 5);
    }

    #[test]
    fn test_generate_reply_text_positive() {
        let templates = SentimentTemplates::default();
        let reply = generate_reply_text(Sentiment::Positive, 0, &templates);
        let positive_phrases = &[
            "Great point!",
            "Absolutely agree.",
            "Well said.",
            "Thanks for sharing!",
            "This is spot on.",
        ];
        assert!(positive_phrases.contains(&&*reply));
    }

    #[test]
    fn test_generate_reply_text_neutral() {
        let templates = SentimentTemplates::default();
        let reply = generate_reply_text(Sentiment::Neutral, 0, &templates);
        let neutral_phrases = &["Interesting.", "Thanks.", "Noted.", "Hmm.", "I see."];
        assert!(neutral_phrases.contains(&&*reply));
    }

    #[test]
    fn test_generate_reply_text_negative() {
        let templates = SentimentTemplates::default();
        let reply = generate_reply_text(Sentiment::Negative, 0, &templates);
        let negative_phrases = &[
            "I disagree, but good discussion.",
            "Different perspective, but thanks.",
            "I see your point, though I think otherwise.",
            "Respectfully, I have to differ.",
        ];
        assert!(negative_phrases.contains(&&*reply));
    }

    #[test]
    fn test_generate_reply_text_negative_wraps_without_panic() {
        // reply_idx=9 previously panicked because NEGATIVE had 4 entries and idx used mod 5.
        let templates = SentimentTemplates::default();
        let reply = generate_reply_text(Sentiment::Negative, 9, &templates);
        let negative_phrases = &[
            "I disagree, but good discussion.",
            "Different perspective, but thanks.",
            "I see your point, though I think otherwise.",
            "Respectfully, I have to differ.",
        ];
        assert!(negative_phrases.contains(&&*reply));
    }

    #[test]
    fn test_extract_tweet_button_position() {
        let tweet = json!({
            "buttons": {
                "like": { "x": 10.5, "y": 20.25 }
            }
        });
        let pos = extract_tweet_button_position(&tweet, "like");
        assert_eq!(pos, Some((10.5, 20.25)));
    }

    #[test]
    fn test_task_config_from_payload_with_all_fields() {
        let payload = json!({
            "duration_ms": 120000,
            "candidate_count": 10,
            "thread_depth": 15,
            "max_actions_per_scan": 5,
            "weights": { "like_prob": 0.5 },
            "llm_enabled": true,
            "smart_decision_enabled": true,
            "enhanced_sentiment_enabled": false
        });
        let twitter_config = crate::config::TwitterActivityConfig::default();
        let config = TaskConfig::from_payload(&payload, &twitter_config);
        assert_eq!(config.duration_ms, 120000);
        assert_eq!(config.candidate_count, 10);
        assert_eq!(config.thread_depth, 15);
        assert_eq!(config.max_actions_per_scan, 5);
        assert!(config.weights.is_some());
        assert!(config.llm_enabled);
        assert!(config.smart_decision_enabled);
        assert!(!config.enhanced_sentiment_enabled); // Explicitly set to false
    }

    #[test]
    fn test_task_config_from_payload_with_defaults() {
        let payload = json!({});
        let twitter_config = crate::config::TwitterActivityConfig::default();
        let config = TaskConfig::from_payload(&payload, &twitter_config);
        // duration_ms has random default, so just check it's non-zero
        assert!(config.duration_ms > 0);
        assert_eq!(config.candidate_count, twitter_config.default_candidate_count);
        assert_eq!(config.thread_depth, twitter_config.default_thread_depth);
        assert_eq!(config.max_actions_per_scan, twitter_config.default_max_actions_per_scan);
        assert!(config.weights.is_none());
        assert!(!config.llm_enabled);
        assert!(!config.smart_decision_enabled);
        assert!(config.enhanced_sentiment_enabled); // Default to true
    }

    #[test]
    fn test_task_config_max_actions_per_scan_minimum() {
        let payload = json!({ "max_actions_per_scan": 0 });
        let twitter_config = crate::config::TwitterActivityConfig::default();
        let config = TaskConfig::from_payload(&payload, &twitter_config);
        assert_eq!(config.max_actions_per_scan, 1); // should be max(1, 0)
    }

    #[test]
    fn test_select_entry_point_returns_valid_url() {
        let url = select_entry_point();
        // Verify the returned URL is one of the valid entry points
        let valid_urls: Vec<&str> = ENTRY_POINTS.iter().map(|ep| ep.url).collect();
        assert!(valid_urls.contains(&url));
    }

    #[test]
    fn test_select_entry_point_total_weight_calculation() {
        let total_weight: u32 = ENTRY_POINTS.iter().map(|ep| ep.weight).sum();
        // Verify total weight matches expected value (59 + 4*8 + 2*2 + 1*4 = 59 + 32 + 4 + 4 = 99)
        assert_eq!(total_weight, 99);
    }

    #[test]
    fn test_select_entry_point_multiple_calls() {
        // Call multiple times to ensure function works consistently
        for _ in 0..10 {
            let url = select_entry_point();
            let valid_urls: Vec<&str> = ENTRY_POINTS.iter().map(|ep| ep.url).collect();
            assert!(valid_urls.contains(&url));
        }
    }

    #[test]
    fn test_entry_points_structure() {
        // Verify ENTRY_POINTS has the expected structure
        assert_eq!(ENTRY_POINTS.len(), 15);
        
        // Verify all weights are positive
        for ep in ENTRY_POINTS.iter() {
            assert!(ep.weight > 0);
            assert!(!ep.url.is_empty());
        }
    }
}
