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
    RUN_COUNTER_BUTTON_MISSING, RUN_COUNTER_CANDIDATE_SCANNED, RUN_COUNTER_CLICK_VERIFY_FAILED,
    RUN_COUNTER_DIVE_TARGET_FALLBACK_USED,
};
use crate::prelude::TaskContext;
use crate::utils::mouse::hover_before_click;
use crate::utils::twitter::{
    twitteractivity_decision::*, twitteractivity_dive::*, twitteractivity_feed::*,
    twitteractivity_humanized::*, twitteractivity_interact::*, twitteractivity_limits::*,
    twitteractivity_llm::*, twitteractivity_navigation::*, twitteractivity_persona::*,
    twitteractivity_popup::*, twitteractivity_sentiment::*,
};

/// Default feed scan duration range (ms): 5-9 minutes (300-540 seconds)
fn default_duration_ms() -> u64 {
    rand::random::<u64>() % (540_000 - 300_000) + 300_000 // 300-540 seconds
}
/// Default number of engagement candidates to consider
const DEFAULT_CANDIDATES: u32 = 5;
/// Default thread depth when diving
const DEFAULT_THREAD_DEPTH: u32 = 5;
/// Minimum delay between different action types on same tweet (ms)
const MIN_ACTION_CHAIN_DELAY_MS: u64 = 3000;
/// Minimum delay between feed candidate scans (ms)
const MIN_CANDIDATE_SCAN_INTERVAL_MS: u64 = 2500;
/// Maximum number of successful actions allowed per candidate scan iteration.
const DEFAULT_MAX_ACTIONS_PER_SCAN: u32 = 3;

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
#[derive(Debug, Clone, Default)]
pub struct TweetActionTracker {
    /// Maps tweet ID to (last_action_type, timestamp)
    last_action: HashMap<String, (&'static str, Instant)>,
}

impl TweetActionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if an action is allowed on this tweet (prevents rapid action chains).
    pub fn can_perform_action(&self, tweet_id: &str, _action_type: &str) -> bool {
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
    pub fn record_action(&mut self, tweet_id: String, action_type: &'static str) {
        let tweet_id_for_log = tweet_id.clone();
        self.last_action
            .insert(tweet_id, (action_type, Instant::now()));
        info!(
            "Recorded {} action on tweet {} (cooldown: {}ms)",
            action_type, tweet_id_for_log, MIN_ACTION_CHAIN_DELAY_MS
        );
    }
}

/// Navigate to weighted entry point and simulate reading if not on home.
/// Following Node.js navigateAndRead pattern.
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
    let duration_ms = read_u64(&payload, "duration_ms", default_duration_ms());
    let candidate_count = read_u32(&payload, "candidate_count", DEFAULT_CANDIDATES);
    let thread_depth = read_u32(&payload, "thread_depth", DEFAULT_THREAD_DEPTH);
    let max_actions_per_scan = read_u32(
        &payload,
        "max_actions_per_scan",
        DEFAULT_MAX_ACTIONS_PER_SCAN,
    )
    .max(1);
    let weights = payload.get("weights");

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

    // Build persona weights
    let mut persona = select_persona_weights(weights, &config.twitter_activity.probabilities);
    let profile = api.behavior_profile();

    persona = apply_behavior_profile(persona, profile, 0.0);

    info!(
        "Persona weights: like={:.2}, rt={:.2}, follow={:.2}, reply={:.2}",
        persona.like_prob, persona.retweet_prob, persona.follow_prob, persona.reply_prob
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
    info!(
        "Per-scan action budget: {} successful actions",
        max_actions_per_scan
    );

    // Phase 1: Navigation & authentication check
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

    // Phase 2: Feed analysis & scrolling
    info!("Phase 2: Scanning feed for {} ms", duration_ms);
    let deadline = Instant::now() + Duration::from_millis(duration_ms);
    let mut _actions_taken = 0u32;
    let mut last_remaining = Duration::from_millis(duration_ms);

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
                .take(candidate_count as usize)
                .collect::<Vec<_>>();
            let mut actions_this_scan = 0u32;

            for tweet in to_consider {
                if actions_this_scan >= max_actions_per_scan {
                    info!(
                        "Per-scan action budget reached ({}/{}), deferring remaining candidates",
                        actions_this_scan, max_actions_per_scan
                    );
                    break;
                }

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

                // Determine actions to perform on this tweet
                let mut actions_to_do: Vec<&str> = Vec::new();
                let tweet_id = tweet
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                // Check each action type, respecting limits and chaining
                if should_like(&candidate_persona)
                    && action_tracker.can_perform_action(tweet_id, "like")
                    && limits.can_like(&counters)
                {
                    actions_to_do.push("like");
                }
                if should_retweet(&candidate_persona)
                    && action_tracker.can_perform_action(tweet_id, "retweet")
                    && limits.can_retweet(&counters)
                {
                    actions_to_do.push("retweet");
                }
                if should_quote(&candidate_persona)
                    && action_tracker.can_perform_action(tweet_id, "quote_tweet")
                    && limits.can_quote_tweet(&counters)
                {
                    actions_to_do.push("quote");
                }
                if should_follow(&candidate_persona)
                    && action_tracker.can_perform_action(tweet_id, "follow")
                    && limits.can_follow(&counters)
                {
                    actions_to_do.push("follow");
                }
                if should_reply(&candidate_persona)
                    && action_tracker.can_perform_action(tweet_id, "reply")
                    && limits.can_reply(&counters)
                {
                    actions_to_do.push("reply");
                }
                if should_bookmark(&candidate_persona)
                    && action_tracker.can_perform_action(tweet_id, "bookmark")
                    && limits.can_bookmark(&counters)
                {
                    actions_to_do.push("bookmark");
                }

                // Determine if we need to dive (retweet, quote, reply, follow, bookmark require dive; like does not)
                let need_dive = actions_to_do.iter().any(|&action| action != "like");
                let mut did_dive = false;

                if need_dive {
                    // Dive into thread for non-like actions
                    if actions_this_scan >= max_actions_per_scan {
                        break;
                    }
                    if !limits.can_dive(&counters) {
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
                        } else {
                            read_full_thread(api, thread_depth).await?;
                            scroll_pause(api).await;
                            counters.increment_thread_dive();
                            _actions_taken += 1;
                            actions_this_scan += 1;
                            // Record dive action
                            action_tracker.record_action(tweet_id.to_string(), "dive");
                            did_dive = true;
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
                                        if llm_enabled {
                                            match extract_tweet_context(api).await {
                                                Ok((author, text, replies)) => {
                                                    match generate_quote_commentary(api, &author, &text, replies).await {
                                                        Ok(commentary) => {
                                                            match quote_tweet(api, &commentary).await {
                                                                Ok(success) => {
                                                                    if success {
                                                                        info!("Quote tweeted with commentary: {}", commentary);
                                                                    }
                                                                    success
                                                                }
                                                                Err(e) => {
                                                                    warn!("Quote tweet error: {}", e);
                                                                    false
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            warn!("Failed to generate quote commentary: {}", e);
                                                            false
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    warn!("Failed to extract tweet context for quote: {}", e);
                                                    false
                                                }
                                            }
                                        } else {
                                            false // Skip if no LLM
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
                                        let reply_text = if llm_enabled {
                                            match extract_tweet_context(api).await {
                                                Ok((author, text, replies)) => {
                                                    match generate_reply(api, &author, &text, replies).await {
                                                        Ok(reply) => {
                                                            info!("Generated LLM reply: {}", reply);
                                                            reply
                                                        }
                                                        Err(e) => {
                                                            warn!("LLM reply failed, using template: {}", e);
                                                            generate_reply_text(sentiment, counters.replies)
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    warn!("Failed to extract context, using template: {}", e);
                                                    generate_reply_text(sentiment, counters.replies)
                                                }
                                            }
                                        } else {
                                            generate_reply_text(sentiment, counters.replies)
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
                            }
                            "retweet" => {
                                info!("Retweeted");
                                counters.increment_retweet();
                            }
                            "quote" => counters.increment_quote_tweet(),
                            "follow" => {
                                info!("Followed user");
                                counters.increment_follow();
                            }
                            "reply" => {
                                info!("Replied with sentiment {:?}", sentiment);
                                counters.increment_reply();
                            }
                            "bookmark" => {
                                info!("Bookmarked tweet");
                                counters.increment_bookmark();
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
                        if action == "like" {
                            api.increment_run_counter(RUN_COUNTER_CLICK_VERIFY_FAILED, 1);
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

// Helper: generate a short reply string based on sentiment
// Uses reply count as index for deterministic selection
fn generate_reply_text(sentiment: Sentiment, reply_idx: u32) -> String {
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
        Sentiment::Positive => POSITIVE[(reply_idx as usize) % POSITIVE.len()],
        Sentiment::Neutral => NEUTRAL[(reply_idx as usize) % NEUTRAL.len()],
        Sentiment::Negative => NEGATIVE[(reply_idx as usize) % NEGATIVE.len()],
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

    #[test]
    fn test_generate_reply_text_negative_wraps_without_panic() {
        // reply_idx=9 previously panicked because NEGATIVE had 4 entries and idx used mod 5.
        let reply = generate_reply_text(Sentiment::Negative, 9);
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
}
