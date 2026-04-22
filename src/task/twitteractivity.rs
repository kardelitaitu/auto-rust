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

use crate::prelude::TaskContext;
use crate::utils::mouse::hover_before_click;
use crate::utils::twitter::{
    twitteractivity_decision::*, twitteractivity_dive::*, twitteractivity_feed::*,
    twitteractivity_humanized::*, twitteractivity_interact::*, twitteractivity_limits::*,
    twitteractivity_llm::*, twitteractivity_navigation::*, twitteractivity_persona::*,
    twitteractivity_popup::*, twitteractivity_sentiment::*,
};

/// Default feed scan duration (ms): 2 minutes
const DEFAULT_DURATION_MS: u64 = 120_000;
/// Default number of engagement candidates to consider
const DEFAULT_CANDIDATES: u32 = 5;
/// Default thread depth when diving
const DEFAULT_THREAD_DEPTH: u32 = 5;
/// Minimum delay between different action types on same tweet (ms)
const MIN_ACTION_CHAIN_DELAY_MS: u64 = 3000;
/// Minimum delay between feed candidate scans (ms)
const MIN_CANDIDATE_SCAN_INTERVAL_MS: u64 = 2500;

/// Tracks the last action type and timestamp for each tweet to prevent unrealistic action chains.
#[derive(Debug, Clone, Default)]
struct TweetActionTracker {
    /// Maps tweet ID to (last_action_type, timestamp)
    last_action: HashMap<String, (&'static str, Instant)>,
}

impl TweetActionTracker {
    fn new() -> Self {
        Self::default()
    }

    /// Check if an action is allowed on this tweet (prevents rapid action chains).
    fn can_perform_action(&self, tweet_id: &str, _action_type: &str) -> bool {
        if let Some((_, last_time)) = self.last_action.get(tweet_id) {
            let elapsed = last_time.elapsed();
            // Enforce minimum delay between different action types on same tweet
            if elapsed.as_millis() < MIN_ACTION_CHAIN_DELAY_MS as u128 {
                return false;
            }
        }
        true
    }

    /// Record that an action was performed on a tweet.
    fn record_action(&mut self, tweet_id: String, action_type: &'static str) {
        let tweet_id_for_log = tweet_id.clone();
        self.last_action
            .insert(tweet_id, (action_type, Instant::now()));
        info!(
            "Recorded {} action on tweet {} (cooldown: {}ms)",
            action_type, tweet_id_for_log, MIN_ACTION_CHAIN_DELAY_MS
        );
    }
}

/// Task entry point called by orchestrator.
///
/// # Arguments
/// * `api` - Task context with page, profile, clipboard
/// * `payload` - JSON task configuration (see module docs)
///
/// # Returns
/// Result<()> - Ok if completed successfully, Err on failure
pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    info!("Task started");

    // Parse task configuration from payload
    let duration_ms = read_u64(&payload, "duration_ms", DEFAULT_DURATION_MS);
    let candidate_count = read_u32(&payload, "candidate_count", DEFAULT_CANDIDATES);
    let thread_depth = read_u32(&payload, "thread_depth", DEFAULT_THREAD_DEPTH);
    let weights = payload.get("weights");

    // Parse LLM config (V2 feature)
    let llm_enabled = payload
        .get("llm_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let llm_reply_probability = payload
        .get("llm_reply_probability")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.05);
    let llm_quote_probability = payload
        .get("llm_quote_probability")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.15);

    // Parse smart decision config (V3 feature - rule-based)
    let smart_decision_enabled = payload
        .get("smart_decision_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Build persona weights
    let mut persona = select_persona_weights(weights);
    let profile = api.behavior_profile();

    persona = apply_behavior_profile(persona, profile, 0.0);

    info!(
        "Persona weights: like={:.2}, rt={:.2}, follow={:.2}, reply={:.2}",
        persona.like_prob,
        persona.retweet_prob,
        persona.follow_prob,
        persona.reply_prob
    );

    // Initialize engagement counters and limits
    let mut counters = EngagementCounters::new();
    let limits = EngagementLimits::default();
    let mut action_tracker = TweetActionTracker::new();

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

    // Phase 1: Navigation & authentication check
    info!("Phase 1: Navigation to home feed");
    goto_home(api).await?;

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

    // Phase 2: Feed analysis & scrolling
    info!("Phase 2: Scanning feed for {} ms", duration_ms);
    let deadline = Instant::now() + Duration::from_millis(duration_ms);
    let mut _actions_taken = 0u32;
    let mut last_remaining = Duration::from_millis(duration_ms);

    // Continuous scrolling like pageview
    let profile = api.behavior_runtime();
    let scroll_amount = profile.scroll.amount;
    let scroll_pause_ms = profile.scroll.pause_ms;
    let smooth = profile.scroll.smooth;
    let scroll_interval = Duration::from_millis(scroll_pause_ms);
    let candidate_scan_interval = Duration::from_millis(MIN_CANDIDATE_SCAN_INTERVAL_MS);
    let mut next_scroll = Instant::now();
    let mut next_candidate_scan = Instant::now();

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

        // Identify candidate tweets and cache engagement button positions
        let scan_started = Instant::now();
        let candidates = identify_engagement_candidates(api).await?;
        let buttons = if !candidates.is_empty() {
            get_tweet_engagement_buttons(api).await?
        } else {
            serde_json::Value::Null
        };
        info!(
            "candidate_scan | candidates={} duration_ms={}",
            candidates.len(),
            scan_started.elapsed().as_millis()
        );
        next_candidate_scan = scan_started + candidate_scan_interval;

        if !candidates.is_empty() {
            let to_consider = candidates
                .iter()
                .take(candidate_count as usize)
                .collect::<Vec<_>>();

            for tweet in to_consider {
                // Analyze sentiment (lightweight)
                let sentiment = analyze_tweet_sentiment(tweet);
                let mut candidate_persona = persona.clone();
                // Modulate weights by sentiment
                candidate_persona.interest_multiplier = if sentiment == Sentiment::Negative {
                    0.3 // suppress engagement on negative tweets
                } else if sentiment == Sentiment::Positive {
                    1.3 // boost positive
                } else {
                    1.0
                };

                // Smart decision check (V3 feature - rule-based)
                let engagement_decision = if smart_decision_enabled {
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
                } else {
                    None
                };

                // Skip if smart decision says None
                if let Some(ref decision) = engagement_decision {
                    if decision.level == EngagementLevel::None {
                        info!(
                            "Skipping engagement: {} (score: {})",
                            decision.reason, decision.score
                        );
                        continue; // Skip to next tweet
                    }
                }

                // Decision: Like?
                if should_like(&candidate_persona) {
                    // Check action chaining prevention
                    let tweet_id = tweet
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    if !action_tracker.can_perform_action(tweet_id, "like") {
                        info!(
                            "Skipping like on tweet {}: action chain cooldown active",
                            tweet_id
                        );
                    }
                    // Check limits
                    else if !limits.can_like(&counters) {
                        info!(
                            "Skipping like: limit reached ({}/{})",
                            counters.likes, limits.max_likes
                        );
                    } else if let Some(pos) = tweet
                        .get("x")
                        .and_then(|v| v.as_f64())
                        .zip(tweet.get("y").and_then(|v| v.as_f64()))
                    {
                        // Move to tweet center and click like button (using cached buttons)
                        if let Some(btn_pos) = find_like_button_near(&buttons, pos.0, pos.1)? {
                            if like_at_position(api, btn_pos.0, btn_pos.1).await? {
                                info!("Liked tweet");
                                counters.increment_like();
                                _actions_taken += 1;
                                // Record action for chain prevention
                                action_tracker.record_action(tweet_id.to_string(), "like");
                                // Use clustered pause for micro-movements between actions
                                clustered_engagement_pause(api).await;
                            }
                        } else {
                            warn!("Like button not found near ({}, {})", pos.0, pos.1);
                        }
                    }
                }

                // Decision: Retweet or Quote Tweet?
                if should_retweet(&candidate_persona) {
                    // Check action chaining prevention
                    let tweet_id = tweet
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    if !action_tracker.can_perform_action(tweet_id, "retweet") {
                        info!(
                            "Skipping retweet on tweet {}: action chain cooldown active",
                            tweet_id
                        );
                    }
                    // Check limits
                    else if !limits.can_retweet(&counters) {
                        info!(
                            "Skipping retweet: limit reached ({}/{})",
                            counters.retweets, limits.max_retweets
                        );
                    } else {
                        // 15% chance to quote tweet instead of native retweet (V2 feature)
                        let do_quote_tweet = llm_enabled
                            && rand::random::<f64>() < llm_quote_probability
                            && limits.can_quote_tweet(&counters);

                        if do_quote_tweet {
                            // Generate quote tweet commentary
                            match extract_tweet_context(api).await {
                                Ok((author, text, replies)) => {
                                    match generate_quote_commentary(api, &author, &text, replies)
                                        .await
                                    {
                                        Ok(commentary) => {
                                            match quote_tweet(api, &commentary).await {
                                                Ok(true) => {
                                                    info!(
                                                        "Quote tweeted with commentary: {}",
                                                        commentary
                                                    );
                                                    counters.increment_quote_tweet();
                                                    _actions_taken += 1;
                                                    // Record action for chain prevention
                                                    action_tracker.record_action(
                                                        tweet_id.to_string(),
                                                        "quote_tweet",
                                                    );
                                                    // Use clustered reply pause for quote tweets (thinking time)
                                                    clustered_reply_pause(api).await;
                                                }
                                                Ok(false) => {
                                                    warn!("Quote tweet failed");
                                                }
                                                Err(e) => {
                                                    warn!("Quote tweet error: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Failed to generate quote commentary, falling back to retweet: {}", e);
                                            if retweet_tweet(api).await? {
                                                info!("Retweeted (fallback)");
                                                counters.increment_retweet();
                                                _actions_taken += 1;
                                                // Record action for chain prevention
                                                action_tracker
                                                    .record_action(tweet_id.to_string(), "retweet");
                                                // Use clustered pause for micro-movements
                                                clustered_engagement_pause(api).await;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to extract tweet context for quote: {}", e);
                                    if retweet_tweet(api).await? {
                                        info!("Retweeted (fallback)");
                                        counters.increment_retweet();
                                        _actions_taken += 1;
                                        engagement_pause(api).await;
                                    }
                                }
                            }
                        } else {
                            // Native retweet
                            if retweet_tweet(api).await? {
                                info!("Retweeted");
                                counters.increment_retweet();
                                _actions_taken += 1;
                                // Record action for chain prevention
                                action_tracker.record_action(tweet_id.to_string(), "retweet");
                                // Use clustered pause for micro-movements
                                clustered_engagement_pause(api).await;
                            }
                        }
                    }
                }

                // Decision: Follow author?
                if should_follow(&candidate_persona) {
                    // Check action chaining prevention
                    let tweet_id = tweet
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    if !action_tracker.can_perform_action(tweet_id, "follow") {
                        info!(
                            "Skipping follow on tweet {}: action chain cooldown active",
                            tweet_id
                        );
                    }
                    // Check limits
                    else if !limits.can_follow(&counters) {
                        info!(
                            "Skipping follow: limit reached ({}/{})",
                            counters.follows, limits.max_follows
                        );
                    } else if follow_from_tweet(api).await? {
                        info!("Followed user");
                        counters.increment_follow();
                        _actions_taken += 1;
                        // Record action for chain prevention
                        action_tracker.record_action(tweet_id.to_string(), "follow");
                        // Use clustered pause for micro-movements
                        clustered_engagement_pause(api).await;
                    }
                }

                // Decision: Reply?
                if should_reply(&candidate_persona) {
                    // Check action chaining prevention
                    let tweet_id = tweet
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    if !action_tracker.can_perform_action(tweet_id, "reply") {
                        info!(
                            "Skipping reply on tweet {}: action chain cooldown active",
                            tweet_id
                        );
                    }
                    // Check limits
                    else if !limits.can_reply(&counters) {
                        info!(
                            "Skipping reply: limit reached ({}/{})",
                            counters.replies, limits.max_replies
                        );
                    } else {
                        // Try LLM-powered reply if enabled, fallback to template
                        let reply_text = if llm_enabled
                            && rand::random::<f64>() < llm_reply_probability
                        {
                            // Extract tweet context for LLM
                            match extract_tweet_context(api).await {
                                Ok((author, text, replies)) => {
                                    match generate_reply(api, &author, &text, replies).await {
                                        Ok(reply) => {
                                            info!("Generated LLM reply: {}", reply);
                                            reply
                                        }
                                        Err(e) => {
                                            warn!(
                                                "LLM reply generation failed, using template: {}",
                                                e
                                            );
                                            generate_reply_text(sentiment, counters.replies)
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to extract tweet context, using template: {}", e);
                                    generate_reply_text(sentiment, counters.replies)
                                }
                            }
                        } else {
                            // Use template reply
                            generate_reply_text(sentiment, counters.replies)
                        };

                        if reply_to_tweet(api, &reply_text).await? {
                            info!("Replied with sentiment {:?}", sentiment);
                            counters.increment_reply();
                            _actions_taken += 1;
                            // Record action for chain prevention
                            action_tracker.record_action(tweet_id.to_string(), "reply");
                            // Use clustered reply pause for thinking time after composing
                            clustered_reply_pause(api).await;
                        }
                    }
                }

                // Decision: Dive into thread?
                if should_dive(&candidate_persona) {
                    if !limits.can_dive(&counters) {
                        info!(
                            "Skipping thread dive: limit reached ({}/{})",
                            counters.thread_dives, limits.max_thread_dives
                        );
                    } else if let Some(pos) = tweet
                        .get("x")
                        .and_then(|v| v.as_f64())
                        .zip(tweet.get("y").and_then(|v| v.as_f64()))
                    {
                        dive_into_thread(api, pos.0, pos.1).await?;
                        read_full_thread(api, thread_depth).await?;
                        scroll_pause(api).await;
                        counters.increment_thread_dive();
                        _actions_taken += 1;
                        // Navigate back to home feed
                        goto_home(api).await?;
                        scroll_pause(api).await;
                    }
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
    let _remaining_limits = limits.remaining(&counters);
    let duration_secs = (Duration::from_millis(duration_ms) - last_remaining).as_secs_f64();
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

// Helper: find the like button near a given tweet center coordinate
// Uses cached buttons to avoid repeated DOM queries
fn find_like_button_near(
    buttons: &serde_json::Value,
    tweet_x: f64,
    tweet_y: f64,
) -> Result<Option<(f64, f64)>> {
    if let Some(like_obj) = buttons.get("like").and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            like_obj.get("x").and_then(|v| v.as_f64()),
            like_obj.get("y").and_then(|v| v.as_f64()),
        ) {
            let dx = (x - tweet_x).abs();
            let dy = (y - tweet_y).abs();
            if dx < 400.0 && dy < 600.0 {
                return Ok(Some((x, y)));
            }
        }
    }
    Ok(None)
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
    let result = page.evaluate(r#"
        (function() {
            var btn = document.querySelector('[data-testid="like"]');
            if (!btn) return false;
            var svg = btn.querySelector('svg');
            if (!svg) return false;
            var color = svg.getAttribute('color') || svg.getAttribute('fill') || '';
            return color.includes(' rgb') || color.includes('#') || svg.parentElement?.getAttribute('data-testid')?.includes('unlike');
        })()
    "#).await?;

    let value = result.value();
    if let Some(v) = value {
        if let Some(liked) = v.as_bool() {
            return Ok(liked);
        }
    }

    Ok(true)
}

// Helper: generate a short reply string based on sentiment
// Uses reply count as index for deterministic selection
fn generate_reply_text(sentiment: Sentiment, reply_idx: u32) -> String {
    let idx = (reply_idx as usize) % 5;
    const POSITIVE: &[&str] = &[
        "Great point!",
        "Absolutely agree.",
        "Well said.",
        "Thanks for sharing!",
        "This is spot on.",
    ];
    const NEUTRAL: &[&str] = &["Interesting.", "Thanks.", "Noted.", "Hmm.", "I see."];
    const NEGATIVE: &[&str] = &[
        "I disagree, but good discussion.",
        "Different perspective, but thanks.",
        "I see your point, though I think otherwise.",
        "Respectfully, I have to differ.",
    ];
    match sentiment {
        Sentiment::Positive => POSITIVE[idx],
        Sentiment::Neutral => NEUTRAL[idx],
        Sentiment::Negative => NEGATIVE[idx],
    }
    .to_string()
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
        let reply = generate_reply_text(Sentiment::Positive, 0);
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
        let reply = generate_reply_text(Sentiment::Neutral, 0);
        let neutral_phrases = &["Interesting.", "Thanks.", "Noted.", "Hmm.", "I see."];
        assert!(neutral_phrases.contains(&&*reply));
    }

    #[test]
    fn test_generate_reply_text_negative() {
        let reply = generate_reply_text(Sentiment::Negative, 0);
        let negative_phrases = &[
            "I disagree, but good discussion.",
            "Different perspective, but thanks.",
            "I see your point, though I think otherwise.",
            "Respectfully, I have to differ.",
        ];
        assert!(negative_phrases.contains(&&*reply));
    }
}
