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
use std::time::{Duration, Instant};

use crate::prelude::TaskContext;
use crate::utils::twitter::{
    twitteractivity_dive::*, twitteractivity_decision::*, twitteractivity_feed::*,
    twitteractivity_humanized::*, twitteractivity_interact::*, twitteractivity_limits::*,
    twitteractivity_llm::*, twitteractivity_navigation::*, twitteractivity_persona::*,
    twitteractivity_popup::*, twitteractivity_sentiment::*,
};

/// Default feed scan duration (ms): 2 minutes
const DEFAULT_DURATION_MS: u64 = 120_000;
/// Default number of scroll actions per scan cycle
const DEFAULT_SCROLL_COUNT: u32 = 12;
/// Default number of engagement candidates to consider
const DEFAULT_CANDIDATES: u32 = 5;
/// Default thread depth when diving
const DEFAULT_THREAD_DEPTH: u32 = 5;

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
    let scroll_count = read_u32(&payload, "scroll_count", DEFAULT_SCROLL_COUNT);
    let candidate_count = read_u32(&payload, "candidate_count", DEFAULT_CANDIDATES);
    let thread_depth = read_u32(&payload, "thread_depth", DEFAULT_THREAD_DEPTH);
    let weights = payload.get("weights");
    
    // Parse LLM config (V2 feature)
    let llm_enabled = payload.get("llm_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
    let llm_reply_probability = payload.get("llm_reply_probability").and_then(|v| v.as_f64()).unwrap_or(0.05);
    let llm_quote_probability = payload.get("llm_quote_probability").and_then(|v| v.as_f64()).unwrap_or(0.15);
    
    // Parse smart decision config (V3 feature - rule-based)
    let smart_decision_enabled = payload.get("smart_decision_enabled").and_then(|v| v.as_bool()).unwrap_or(false);

    // Build persona weights
    let mut persona = select_persona_weights(weights);
    // Apply profile characteristics (variance, sentiment modulation will be applied per-tweet)
    let profile = api.behavior_profile();
    // For now we use the session's assigned profile; profile preset integration TBD

    persona = apply_behavior_profile(persona, profile, 0.0);

    info!(
        "Persona weights: like={:.2}, rt={:.2}, follow={:.2}, reply={:.2}, scroll_count={}",
        persona.like_prob,
        persona.retweet_prob,
        persona.follow_prob,
        persona.reply_prob,
        scroll_count
    );

    // Initialize engagement counters and limits
    let mut counters = EngagementCounters::new();
    let limits = EngagementLimits::default();
    
    info!(
        "Engagement limits: likes={}/{}, retweets={}/{}, follows={}/{}, total={}/{}",
        counters.likes, limits.max_likes,
        counters.retweets, limits.max_retweets,
        counters.follows, limits.max_follows,
        counters.total_actions(), limits.max_total_actions
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
    let mut actions_taken = 0u32;
    let mut last_remaining = Duration::from_millis(duration_ms);

    while Instant::now() < deadline {
        // Scroll to load new content
        scroll_feed(api, scroll_count, false).await?;
        human_pause(api, 800).await;

        // Check for popups periodically
        if !ensure_feed_populated(api).await? {
            warn!("Feed appears empty after scroll");
        }

        // Identify candidate tweets (in viewport)
        let candidates = identify_engagement_candidates(api).await?;
        let candidates_vec: Vec<Value> = candidates;

        if !candidates_vec.is_empty() {
            // Take up to `candidate_count` candidates from top
            let to_consider = candidates_vec
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
                    let tweet_text = tweet.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let replies: Vec<(String, String)> = vec![];  // Would need to extract from DOM
                    Some(decide_engagement(tweet_text, &replies))
                } else {
                    None
                };
                
                // Skip if smart decision says None
                if let Some(ref decision) = engagement_decision {
                    if decision.level == EngagementLevel::None {
                        info!("Skipping engagement: {} (score: {})", decision.reason, decision.score);
                        continue;  // Skip to next tweet
                    }
                }

                // Decision: Like?
                if should_like(&candidate_persona) {
                    // Check smart decision level
                    if let Some(ref decision) = engagement_decision {
                        if decision.level == EngagementLevel::None {
                            // Already skipped above
                        } else if !limits.can_like(&counters) {
                            info!("Skipping like: limit reached ({}/{})", counters.likes, limits.max_likes);
                        } else if let Some(pos) = tweet
                            .get("x")
                            .and_then(|v| v.as_f64())
                            .zip(tweet.get("y").and_then(|v| v.as_f64()))
                        {
                            // Move to tweet center (approximate from tweet x,y) and click like button
                            if let Some(btn_pos) = find_like_button_near(api, pos.0, pos.1).await? {
                                if like_at_position(api, btn_pos.0, btn_pos.1).await? {
                                    info!("Liked tweet");
                                    counters.increment_like();
                                    actions_taken += 1;
                                    human_pause(api, 1200).await;
                                }
                            }
                        }
                    } else if !limits.can_like(&counters) {
                        info!("Skipping like: limit reached ({}/{})", counters.likes, limits.max_likes);
                    } else if let Some(pos) = tweet
                        .get("x")
                        .and_then(|v| v.as_f64())
                        .zip(tweet.get("y").and_then(|v| v.as_f64()))
                    {
                        // Move to tweet center (approximate from tweet x,y) and click like button
                        if let Some(btn_pos) = find_like_button_near(api, pos.0, pos.1).await? {
                            if like_at_position(api, btn_pos.0, btn_pos.1).await? {
                                info!("Liked tweet");
                                counters.increment_like();
                                actions_taken += 1;
                                human_pause(api, 1200).await;
                            }
                        }
                    }
                }

                // Decision: Retweet or Quote Tweet?
                if should_retweet(&candidate_persona) {
                    // Check limits
                    if !limits.can_retweet(&counters) {
                        info!("Skipping retweet: limit reached ({}/{})", counters.retweets, limits.max_retweets);
                    } else {
                        // 15% chance to quote tweet instead of native retweet (V2 feature)
                        let do_quote_tweet = llm_enabled && rand::random::<f64>() < llm_quote_probability && limits.can_quote_tweet(&counters);
                        
                        if do_quote_tweet {
                            // Generate quote tweet commentary
                            match extract_tweet_context(api).await {
                                Ok((author, text, replies)) => {
                                    match generate_quote_commentary(api, &author, &text, replies).await {
                                        Ok(commentary) => {
                                            match quote_tweet(api, &commentary).await {
                                                Ok(true) => {
                                                    info!("Quote tweeted with commentary: {}", commentary);
                                                    counters.increment_quote_tweet();
                                                    actions_taken += 1;
                                                    human_pause(api, 2000).await;
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
                                                actions_taken += 1;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to extract tweet context for quote: {}", e);
                                    if retweet_tweet(api).await? {
                                        info!("Retweeted (fallback)");
                                        counters.increment_retweet();
                                        actions_taken += 1;
                                    }
                                }
                            }
                        } else {
                            // Native retweet
                            if retweet_tweet(api).await? {
                                info!("Retweeted");
                                counters.increment_retweet();
                                actions_taken += 1;
                                human_pause(api, 1500).await;
                            }
                        }
                    }
                }

                // Decision: Follow author?
                if should_follow(&candidate_persona) {
                    if !limits.can_follow(&counters) {
                        info!("Skipping follow: limit reached ({}/{})", counters.follows, limits.max_follows);
                    } else if follow_from_tweet(api).await? {
                        info!("Followed user");
                        counters.increment_follow();
                        actions_taken += 1;
                        human_pause(api, 1200).await;
                    }
                }

                // Decision: Reply?
                if should_reply(&candidate_persona) {
                    if !limits.can_reply(&counters) {
                        info!("Skipping reply: limit reached ({}/{})", counters.replies, limits.max_replies);
                    } else {
                        // Try LLM-powered reply if enabled, fallback to template
                        let reply_text = if llm_enabled && rand::random::<f64>() < llm_reply_probability {
                            // Extract tweet context for LLM
                            match extract_tweet_context(api).await {
                                Ok((author, text, replies)) => {
                                    match generate_reply(api, &author, &text, replies).await {
                                        Ok(reply) => {
                                            info!("Generated LLM reply: {}", reply);
                                            reply
                                        }
                                        Err(e) => {
                                            warn!("LLM reply generation failed, using template: {}", e);
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
                            actions_taken += 1;
                            human_pause(api, 2000).await;
                        }
                    }
                }

                // Decision: Dive into thread?
                if should_dive(&candidate_persona) {
                    if !limits.can_dive(&counters) {
                        info!("Skipping thread dive: limit reached ({}/{})", counters.thread_dives, limits.max_thread_dives);
                    } else if let Some(pos) = tweet
                        .get("x")
                        .and_then(|v| v.as_f64())
                        .zip(tweet.get("y").and_then(|v| v.as_f64()))
                    {
                        dive_into_thread(api, pos.0, pos.1).await?;
                        read_full_thread(api, thread_depth).await?;
                        human_pause(api, 1000).await;
                        counters.increment_thread_dive();
                        actions_taken += 1;
                        // Navigate back to home feed
                        goto_home(api).await?;
                        human_pause(api, 1000).await;
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
    let remaining_limits = limits.remaining(&counters);
    let duration_secs = (Duration::from_millis(duration_ms) - last_remaining).as_secs_f64();
    info!(
        "[twitter] Session complete: cycles={}, likes={}, retweets={}, follows={}, replies={}, dives={}, \
         total_actions={}, duration={:.1}s",
        actions_taken,
        counters.likes,
        counters.retweets,
        counters.follows,
        counters.replies,
        counters.thread_dives,
        counters.total_actions(),
        duration_secs
    );
    info!(
        "[twitter] Limits remaining: likes={}, retweets={}, follows={}, replies={}, dives={}, total={}",
        remaining_limits.get("likes").copied().unwrap_or(0),
        remaining_limits.get("retweets").copied().unwrap_or(0),
        remaining_limits.get("follows").copied().unwrap_or(0),
        remaining_limits.get("replies").copied().unwrap_or(0),
        remaining_limits.get("thread_dives").copied().unwrap_or(0),
        remaining_limits.get("total_actions").copied().unwrap_or(0)
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
// Searches for engagement buttons within a proximity of the tweet center
async fn find_like_button_near(
    api: &TaskContext,
    tweet_x: f64,
    tweet_y: f64,
) -> Result<Option<(f64, f64)>> {
    let buttons = get_tweet_engagement_buttons(api).await?;
    if let Some(like_obj) = buttons.get("like").and_then(|v| v.as_object()) {
        if let (Some(x), Some(y)) = (
            like_obj.get("x").and_then(|v| v.as_f64()),
            like_obj.get("y").and_then(|v| v.as_f64()),
        ) {
            // Basic proximity check: if button is within reasonable distance
            let dx = (x - tweet_x).abs();
            let dy = (y - tweet_y).abs();
            // Allow up to 400px horizontal, 600px vertical distance
            if dx < 400.0 && dy < 600.0 {
                return Ok(Some((x, y)));
            }
        }
    }
    Ok(None)
}

// Helper: click like at a specific coordinate
async fn like_at_position(api: &TaskContext, x: f64, y: f64) -> Result<bool> {
    api.move_mouse_to(x, y).await?;
    human_pause(api, 250).await;
    api.click_at(x, y).await?;
    human_pause(api, 600).await;

    // Verify like was registered by checking if button state changed
    let page = api.page();
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

    Ok(true) // Assume success if can't verify
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
