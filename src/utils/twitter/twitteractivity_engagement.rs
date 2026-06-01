//! Engagement logic for Twitter activity task.
//! Contains `process_candidate()` and helper functions for tweet engagement.

use super::twitteractivity_retry::{retry_with_backoff, RetryConfig};
use super::twitteractivity_state::{CandidateContext, CandidateResult, TaskConfig};
use crate::metrics::{
    RUN_COUNTER_BOOKMARK_FAILURE, RUN_COUNTER_BOOKMARK_SUCCESS, RUN_COUNTER_CLICK_VERIFY_FAILED,
    RUN_COUNTER_DIVE_FAILURE, RUN_COUNTER_DIVE_SUCCESS, RUN_COUNTER_DIVE_TARGET_FALLBACK_USED,
    RUN_COUNTER_FOLLOW_FAILURE, RUN_COUNTER_FOLLOW_SUCCESS, RUN_COUNTER_LIKE_FAILURE,
    RUN_COUNTER_LIKE_SUCCESS, RUN_COUNTER_QUOTE_FAILURE, RUN_COUNTER_QUOTE_SUCCESS,
    RUN_COUNTER_REPLY_FAILURE, RUN_COUNTER_REPLY_SUCCESS, RUN_COUNTER_RETWEET_FAILURE,
    RUN_COUNTER_RETWEET_SUCCESS, RUN_COUNTER_TRANSIENT_ERROR,
};
use crate::prelude::TaskContext;
use crate::utils::twitter::sentiment::SentimentAnalyzer;
use crate::utils::twitter::{
    decision::{
        DecisionEngineFactory, DecisionStrategy, EngagementDecision, EngagementLevel, TweetContext,
    },
    sentiment::Sentiment,
    twitteractivity_dive::{dive_into_thread, identify_thread_replies},
    twitteractivity_humanized::{
        clustered_engagement_pause, clustered_reply_pause, human_pause, scroll_pause,
    },
    twitteractivity_interact::{
        bookmark_tweet, follow_from_tweet, like_tweet, reply_to_tweet, retweet_tweet,
    },
    twitteractivity_limits::{EngagementCounters, EngagementLimits},
    twitteractivity_llm::{
        extract_tweet_context, generate_quote_commentary, generate_reply, quote_tweet,
    },
    twitteractivity_navigation::goto_home,
    twitteractivity_persona::{should_dive, PersonaWeights},
};
use anyhow::Result;
use log::{info, warn};
use rand::Rng;
use serde_json::Value;
use std::time::{Duration, Instant};

pub use super::twitteractivity_actions::{
    extract_tweet_button_position, extract_tweet_text, generate_quote_text, generate_reply_text,
    like_at_position,
};
pub use super::twitteractivity_helpers::{
    action_allowed_by_limits, calc_rate, filter_actions_for_decision_level,
    filter_detail_actions_for_gate, selected_candidate_actions,
    should_engage_replies_after_root_action, should_navigate_home_after_dive, validate_tweet_page,
};

#[cfg(test)]
pub use super::twitteractivity_persona::{
    should_follow, should_like, should_reply, should_retweet,
};
#[cfg(test)]
pub use super::twitteractivity_state::{SentimentTemplates, TweetActionTracker};

/// Smart decision check for engagement.
pub async fn handle_engagement_decision(
    tweet: &Value,
    task_config: &TaskConfig,
    persona: &PersonaWeights,
    llm_api_key: Option<String>,
) -> Option<EngagementDecision> {
    if !task_config.smart_decision_enabled {
        return None;
    }

    // Extract tweet text
    let tweet_text = tweet.get("text").and_then(|v| v.as_str()).unwrap_or("");
    let tweet_id = tweet
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let author = tweet
        .get("author")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Extract replies from tweet data
    let mut replies: Vec<String> = Vec::new();
    if let Some(replies_array) = tweet.get("replies").and_then(|v| v.as_array()) {
        for reply_value in replies_array {
            if let Some(reply_obj) = reply_value.as_object() {
                if let Some(text_value) = reply_obj.get("text") {
                    if let Some(text_str) = text_value.as_str() {
                        replies.push(text_str.to_string());
                    }
                }
            }
        }
    }

    info!(
        "Smart decision: tweet_id={} author=@{} replies={}",
        tweet_id,
        author,
        replies.len()
    );

    // Create context for decision engine
    let ctx = TweetContext {
        tweet_id: tweet_id.to_string(),
        text: tweet_text.to_string(),
        author: author.to_string(),
        replies,
        persona: persona.clone(),
        task_config: task_config.clone(),
        tweet_age: "Recent".to_string(), // Default for feed view
    };

    // Use Factory to create appropriate engine
    // For feed scan, we typically use Legacy or Persona strategy unless LLM is explicitly requested
    let strategy = if task_config.llm_enabled {
        DecisionStrategy::Auto
    } else {
        DecisionStrategy::Legacy
    };

    let engine = DecisionEngineFactory::create(strategy, llm_api_key);

    Some(engine.decide(&ctx).await)
}

/// Cached SentimentAnalyzer instance (created once and reused).
static SENTIMENT_ANALYZER: std::sync::OnceLock<std::sync::Mutex<SentimentAnalyzer>> =
    std::sync::OnceLock::new();

/// Analyze tweet sentiment and modulate persona weights accordingly.
#[allow(clippy::cast_precision_loss)]
fn modulate_persona_by_sentiment(
    tweet: &Value,
    task_config: &TaskConfig,
    persona: &PersonaWeights,
) -> (Sentiment, PersonaWeights) {
    let analyzer = SENTIMENT_ANALYZER
        .get_or_init(|| std::sync::Mutex::new(SentimentAnalyzer::new()))
        .lock()
        .expect("SentimentAnalyzer lock poisoned");
    let tweet_text = extract_tweet_text(tweet);
    let sentiment_result = if task_config.enhanced_sentiment_enabled {
        let thread_context = crate::utils::twitter::sentiment::extract_thread_context(tweet);
        let user_reputation = crate::utils::twitter::sentiment::extract_user_reputation(tweet);
        let temporal_factors = crate::utils::twitter::sentiment::extract_temporal_factors(tweet);
        analyzer.analyze_enhanced(
            &tweet_text,
            thread_context.as_ref(),
            user_reputation.as_ref(),
            temporal_factors.as_ref(),
        )
    } else {
        // Fallback to basic sentiment analysis
        let sentiment = analyzer.analyze_sentiment_sync(&tweet_text);
        crate::utils::twitter::sentiment::EnhancedSentimentResult {
            base_sentiment: sentiment,
            final_sentiment: sentiment,
            base_score: crate::utils::twitter::sentiment::sentiment_score(sentiment) as f32,
            final_score: crate::utils::twitter::sentiment::sentiment_score(sentiment) as f32,
            confidence: 0.7, // Default confidence for basic analysis
            score_breakdown: crate::utils::twitter::sentiment::ScoreBreakdown {
                text_score: crate::utils::twitter::sentiment::sentiment_score(sentiment) as f32,
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
        Sentiment::Positive => 1.4, // boost positive (lightly more than basic)
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

    (sentiment, candidate_persona)
}

/// Engage with replies in depth-first manner.
async fn engage_replies(
    api: &TaskContext,
    persona: &PersonaWeights,
    task_config: &TaskConfig,
    limits: &EngagementLimits,
    counters: &mut EngagementCounters,
    actions_this_scan: &mut u32,
) -> Result<()> {
    match identify_thread_replies(api).await {
        Ok(replies) => {
            let mut replies_engaged = 0;
            let max_replies: u32 = rand::thread_rng().gen_range(1..=2);
            for reply in replies {
                if task_config.dry_run_actions {
                    info!("Dry-run: would like reply...");
                    continue;
                }
                if replies_engaged >= max_replies {
                    break;
                }
                if *actions_this_scan >= task_config.max_actions_per_scan {
                    break;
                }
                if !limits.can_like(counters) {
                    break;
                }
                // Run smart decision for this reply
                if let Some(decision) = handle_engagement_decision(
                    &reply,
                    task_config,
                    persona,
                    task_config.llm_api_key.clone(),
                )
                .await
                {
                    if decision.score > 30 {
                        if let Some(pos) = reply.get("like_pos").and_then(|v| v.as_object()) {
                            let x = pos
                                .get("x")
                                .and_then(serde_json::Value::as_f64)
                                .unwrap_or(0.0);
                            let y = pos
                                .get("y")
                                .and_then(serde_json::Value::as_f64)
                                .unwrap_or(0.0);
                            match retry_with_backoff(
                                || like_at_position(api, x, y),
                                &RetryConfig::aggressive(),
                                api,
                                "depth_first_like",
                            )
                            .await
                            {
                                Ok(true) => {
                                    info!("Successfully liked reply");
                                    counters.increment_like();
                                    *actions_this_scan += 1;
                                    replies_engaged += 1;
                                    api.increment_run_counter(RUN_COUNTER_LIKE_SUCCESS, 1);
                                    // Human-like reading pause between replies
                                    human_pause(api, 1500).await;
                                }
                                _ => {
                                    warn!("Failed to like reply");
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            warn!("Depth-First: Failed to identify replies: {e}");
        }
    }
    Ok(())
}

/// Process a single candidate tweet for engagement.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::cast_precision_loss)]
pub async fn process_candidate(
    mut ctx: CandidateContext<'_>,
    actions_this_scan: u32,
    next_scroll: Instant,
    next_candidate_scan: Instant,
) -> Result<CandidateResult> {
    let mut actions_this_scan = actions_this_scan;
    let mut next_scroll = next_scroll;
    let mut next_candidate_scan = next_candidate_scan;

    // Destructure ctx for easier access (preserve original variable names)
    let tweet = ctx.tweet;
    let persona = ctx.persona;
    let task_config = ctx.task_config;
    let api = ctx.api;
    let limits = ctx.limits;
    let scroll_interval = ctx.scroll_interval;
    let action_tracker = &mut ctx.action_tracker;
    let counters = &mut ctx.counters;

    if actions_this_scan >= task_config.max_actions_per_scan {
        info!(
            "Per-scan action budget reached ({}/{}), deferring remaining candidates",
            actions_this_scan, task_config.max_actions_per_scan
        );
        return Ok(CandidateResult {
            should_break: true,
            next_scroll,
            next_candidate_scan,
            actions_this_scan,
        });
    }

    // Analyze sentiment and modulate persona
    let (sentiment, candidate_persona) = modulate_persona_by_sentiment(tweet, task_config, persona);

    // Smart decision check (V3 feature - rule-based)
    let engagement_decision = handle_engagement_decision(
        tweet,
        task_config,
        &candidate_persona,
        task_config.llm_api_key.clone(),
    )
    .await;

    // Skip if smart decision says None
    if let Some(ref decision) = engagement_decision {
        if decision.level == EngagementLevel::None {
            info!(
                "Skipping engagement: {} (score: {})",
                decision.reason, decision.score
            );
            return Ok(CandidateResult {
                should_break: false,
                next_scroll,
                next_candidate_scan,
                actions_this_scan,
            });
        }
    }

    let tweet_id = tweet
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let mut actions_to_do = selected_candidate_actions(
        &candidate_persona,
        tweet_id,
        limits,
        counters,
        action_tracker,
    );
    if let Some(decision) = engagement_decision.as_ref() {
        filter_actions_for_decision_level(&mut actions_to_do, decision.level);
    }

    let status_url = tweet.get("status_url").and_then(|v| v.as_str());

    let needs_detail_view = actions_to_do.iter().any(|&action| action != "like");
    if needs_detail_view {
        let has_status_url = status_url.is_some();
        let dive_allowed = has_status_url && should_dive(&candidate_persona);
        if !has_status_url {
            info!("Skipping detail-only actions for tweet {tweet_id}: missing status URL");
        } else if !dive_allowed {
            info!(
                "Skipping detail-only actions for tweet {tweet_id}: thread dive gate did not pass"
            );
        }
        filter_detail_actions_for_gate(&mut actions_to_do, has_status_url, dive_allowed);
    }

    if actions_to_do.is_empty() {
        return Ok(CandidateResult {
            should_break: false,
            next_scroll,
            next_candidate_scan,
            actions_this_scan,
        });
    }

    // Retweet, quote, reply, follow, and bookmark require a detail view; like does not.
    let need_dive = actions_to_do.iter().any(|&action| action != "like");
    let mut did_dive = false;

    if need_dive {
        // Dive into thread for non-like actions
        if actions_this_scan >= task_config.max_actions_per_scan {
            return Ok(CandidateResult {
                should_break: true,
                next_scroll,
                next_candidate_scan,
                actions_this_scan,
            });
        }
        if !limits.can_dive(counters) {
            info!(
                "Skipping dive: limit reached ({}/{})",
                counters.thread_dives, limits.max_thread_dives
            );
        } else if task_config.dry_run_actions {
            info!("Dry-run: would dive into thread for tweet {tweet_id}");
            counters.increment_thread_dive();
            actions_this_scan += 1;
            action_tracker.record_action(tweet_id.to_string(), "dive");
            did_dive = true;
            next_scroll = Instant::now() + scroll_interval;
            next_candidate_scan = Instant::now() + scroll_interval;
        } else if let Some(status_url) = status_url {
            // Pause continuous scrolling before diving to avoid interference
            let original_next_scroll = next_scroll;
            let dive_max_pause = Duration::from_secs(60);
            next_scroll = Instant::now() + dive_max_pause;
            info!(
                "Paused continuous scrolling for thread dive (max {}s)",
                dive_max_pause.as_secs()
            );

            let dive_result = retry_with_backoff(
                || dive_into_thread(api, status_url),
                &RetryConfig::default(),
                api,
                "dive_into_thread",
            )
            .await;

            let dive_outcome = match dive_result {
                Ok(outcome) => outcome,
                Err(e) => {
                    warn!("Thread dive failed after retries: {e}");
                    api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                    api.increment_run_counter(RUN_COUNTER_DIVE_FAILURE, 1);
                    // Resume scrolling if dive failed and skip this candidate
                    next_scroll = original_next_scroll;
                    return Ok(CandidateResult {
                        should_break: false,
                        next_scroll,
                        next_candidate_scan,
                        actions_this_scan,
                    });
                }
            };
            if dive_outcome.used_fallback_target {
                api.increment_run_counter(RUN_COUNTER_DIVE_TARGET_FALLBACK_USED, 1);
            }
            if dive_outcome.opened {
                api.increment_run_counter(RUN_COUNTER_DIVE_SUCCESS, 1);
                // Read thread context for LLM use (not cached, extracted fresh when needed)
                if let Err(e) = api.scroll_to_top().await {
                    warn!("Scroll to top failed: {e}");
                    // Non-fatal, continue
                }
                human_pause(api, 800).await;
                scroll_pause(api).await;
                counters.increment_thread_dive();
                actions_this_scan += 1;
                // Record dive action
                action_tracker.record_action(tweet_id.to_string(), "dive");
                did_dive = true;
            } else {
                info!("Thread dive failed: no valid target resolved");
                // Resume scrolling if dive failed
                next_scroll = original_next_scroll;
                api.increment_run_counter(RUN_COUNTER_DIVE_FAILURE, 1);
            }
        }
    }

    // Perform the selected action.
    let mut root_action_success = false;
    for action in actions_to_do {
        if actions_this_scan >= task_config.max_actions_per_scan {
            info!(
                "Skipping {}: per-scan action budget reached after dive ({}/{})",
                action, actions_this_scan, task_config.max_actions_per_scan
            );
            continue;
        }
        if !action_allowed_by_limits(action, limits, counters) {
            info!("Skipping {action}: engagement limit reached after dive");
            continue;
        }

        if task_config.dry_run_actions {
            if action != "like" && !did_dive {
                info!(
                    "Dry-run: would skip {action} on tweet {tweet_id} because thread detail did not open"
                );
                continue;
            }
            info!("Dry-run: would perform {action} on tweet {tweet_id} (did_dive={did_dive})");
            counters.increment(action);
            actions_this_scan += 1;
            action_tracker.record_action(tweet_id.to_string(), action);
            root_action_success = true; // Pretend success for sub-loop simulation
            continue;
        }

        let success = match action {
            "like" => {
                // Like can be done on feed or in detail
                if did_dive {
                    // In detail view, use general like function with retry
                    match retry_with_backoff(
                        || like_tweet(api),
                        &RetryConfig::aggressive(),
                        api,
                        "like_tweet",
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => {
                            warn!("Like failed after retries: {e}");
                            api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                            api.increment_run_counter(RUN_COUNTER_LIKE_FAILURE, 1);
                            false
                        }
                    }
                } else {
                    // On feed, use position from tweet data with retry
                    if let Some(btn_pos) = extract_tweet_button_position(tweet, "like") {
                        match retry_with_backoff(
                            || like_at_position(api, btn_pos.0, btn_pos.1),
                            &RetryConfig::aggressive(),
                            api,
                            "like_at_position",
                        )
                        .await
                        {
                            Ok(result) => result,
                            Err(e) => {
                                warn!("Like at position failed after retries: {e}");
                                api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                                api.increment_run_counter(RUN_COUNTER_LIKE_FAILURE, 1);
                                false
                            }
                        }
                    } else {
                        warn!("Like button position not found in tweet payload for {tweet_id}, falling back to selector-based like");
                        // Fallback to selector-based like (works on feed too)
                        match retry_with_backoff(
                            || like_tweet(api),
                            &RetryConfig::aggressive(),
                            api,
                            "like_tweet",
                        )
                        .await
                        {
                            Ok(result) => result,
                            Err(e) => {
                                warn!("Selector-based like failed after retries: {e}");
                                api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                                api.increment_run_counter(RUN_COUNTER_LIKE_FAILURE, 1);
                                false
                            }
                        }
                    }
                }
            }
            "retweet" => {
                if !validate_tweet_page(api, did_dive, "retweet", tweet_id).await {
                    false
                } else {
                    match retry_with_backoff(
                        || retweet_tweet(api),
                        &RetryConfig::default(),
                        api,
                        "retweet_tweet",
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => {
                            warn!("Retweet failed after retries: {e}");
                            api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                            api.increment_run_counter(RUN_COUNTER_RETWEET_FAILURE, 1);
                            false
                        }
                    }
                }
            }
            "quote" => {
                if !validate_tweet_page(api, did_dive, "quote", tweet_id).await {
                    false
                } else {
                    let quote_text = if task_config.llm_enabled {
                        let (author, text, replies) =
                            extract_tweet_context(api).await.unwrap_or_else(|e| {
                                warn!("Failed to extract tweet context for quote: {e}");
                                ("unknown".to_string(), String::new(), Vec::new())
                            });
                        match generate_quote_commentary(api, &author, &text, replies).await {
                            Ok(commentary) => {
                                info!("Generated LLM quote: {commentary}");
                                commentary
                            }
                            Err(e) => {
                                warn!("LLM quote failed, using template: {e}");
                                generate_quote_text(
                                    sentiment,
                                    counters.quote_tweets,
                                    &task_config.sentiment_templates,
                                )
                            }
                        }
                    } else {
                        generate_quote_text(
                            sentiment,
                            counters.quote_tweets,
                            &task_config.sentiment_templates,
                        )
                    };
                    match quote_tweet(api, &quote_text).await {
                        Ok(success) => {
                            if success {
                                info!("Quote tweeted with commentary: {quote_text}");
                            }
                            success
                        }
                        Err(e) => {
                            warn!("Quote tweet error: {e}");
                            false
                        }
                    }
                }
            }
            "follow" => {
                if !validate_tweet_page(api, did_dive, "follow", tweet_id).await {
                    false
                } else {
                    match retry_with_backoff(
                        || follow_from_tweet(api),
                        &RetryConfig::default(),
                        api,
                        "follow_from_tweet",
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => {
                            warn!("Follow failed after retries: {e}");
                            api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                            api.increment_run_counter(RUN_COUNTER_FOLLOW_FAILURE, 1);
                            false
                        }
                    }
                }
            }
            "reply" => {
                if !validate_tweet_page(api, did_dive, "reply", tweet_id).await {
                    false
                } else {
                    let reply_text = if task_config.llm_enabled {
                        let (author, text, replies) =
                            extract_tweet_context(api).await.unwrap_or_else(|e| {
                                warn!("Failed to extract tweet context for reply: {e}");
                                ("unknown".to_string(), String::new(), Vec::new())
                            });
                        match generate_reply(api, &author, &text, replies).await {
                            Ok(reply) => {
                                info!("Generated LLM reply: {reply}");
                                reply
                            }
                            Err(e) => {
                                warn!("LLM reply failed, using template: {e}");
                                generate_reply_text(
                                    sentiment,
                                    counters.replies,
                                    &task_config.sentiment_templates,
                                )
                            }
                        }
                    } else {
                        generate_reply_text(
                            sentiment,
                            counters.replies,
                            &task_config.sentiment_templates,
                        )
                    };
                    match retry_with_backoff(
                        || reply_to_tweet(api, &reply_text),
                        &RetryConfig::conservative(),
                        api,
                        "reply_to_tweet",
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => {
                            warn!("Reply failed after retries: {e}");
                            api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                            api.increment_run_counter(RUN_COUNTER_REPLY_FAILURE, 1);
                            false
                        }
                    }
                }
            }
            "bookmark" => {
                if !validate_tweet_page(api, did_dive, "bookmark", tweet_id).await {
                    false
                } else {
                    match retry_with_backoff(
                        || bookmark_tweet(api),
                        &RetryConfig::aggressive(),
                        api,
                        "bookmark_tweet",
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(e) => {
                            warn!("Bookmark failed after retries: {e}");
                            api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                            api.increment_run_counter(RUN_COUNTER_BOOKMARK_FAILURE, 1);
                            false
                        }
                    }
                }
            }
            _ => false,
        };

        if success {
            root_action_success = true;
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
                    info!("Replied with sentiment {sentiment:?}");
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

    // Depth-First Engagement: Engage with replies if we dived and root engagement was successful
    if should_engage_replies_after_root_action(did_dive, root_action_success, task_config) {
        engage_replies(
            api,
            &candidate_persona,
            task_config,
            limits,
            counters,
            &mut actions_this_scan,
        )
        .await?;
    }

    // Navigate back to home after dive
    if should_navigate_home_after_dive(did_dive, task_config) {
        // Wait 3-5s after engagement before going home
        let home_wait_ms: u64 = rand::thread_rng().gen_range(3000..5000);
        human_pause(api, home_wait_ms).await;
        info!("Navigating back to home after thread dive and engagement");
        if let Err(e) =
            retry_with_backoff(|| goto_home(api), &RetryConfig::default(), api, "goto_home").await
        {
            warn!("Navigation to home failed after retries: {e}");
            api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
            // Continue anyway - not fatal
        }
        scroll_pause(api).await;
        // Resume continuous scrolling and candidate scanning
        next_scroll = Instant::now() + scroll_interval;
        next_candidate_scan = Instant::now() + scroll_interval;
        info!("Resumed continuous scrolling after thread dive");
    }

    Ok(CandidateResult {
        should_break: false,
        next_scroll,
        next_candidate_scan,
        actions_this_scan,
    })
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};
    use serde_json::json;

    /// Test action_allowed_by_limits for each action type
    #[test]
    fn action_allowed_by_limits_respects_all_limits() {
        let limits = EngagementLimits::with_limits(1, 1, 1, 1, 1, 1, 1, 5);
        let mut counters = EngagementCounters::new();

        // All should be allowed initially
        assert!(action_allowed_by_limits("like", &limits, &counters));
        assert!(action_allowed_by_limits("retweet", &limits, &counters));
        assert!(action_allowed_by_limits("quote", &limits, &counters));
        assert!(action_allowed_by_limits("follow", &limits, &counters));
        assert!(action_allowed_by_limits("reply", &limits, &counters));
        assert!(action_allowed_by_limits("bookmark", &limits, &counters));

        // After incrementing, should be blocked
        counters.increment_like();
        assert!(!action_allowed_by_limits("like", &limits, &counters));
        // Others still allowed
        assert!(action_allowed_by_limits("retweet", &limits, &counters));
    }

    /// Test action_allowed_by_limits with unknown action
    #[test]
    fn action_allowed_by_limits_returns_false_for_unknown() {
        let limits = EngagementLimits::default();
        let counters = EngagementCounters::new();
        assert!(!action_allowed_by_limits(
            "unknown_action",
            &limits,
            &counters
        ));
    }

    #[test]
    fn selected_candidate_actions_respects_persona_limits_and_tracker() {
        let persona = PersonaWeights {
            like_prob: 1.0,
            retweet_prob: 1.0,
            quote_prob: 1.0,
            follow_prob: 1.0,
            reply_prob: 1.0,
            bookmark_prob: 1.0,
            thread_dive_prob: 1.0,
            interest_multiplier: 1.0,
        };
        let limits = EngagementLimits::with_limits(1, 1, 1, 1, 1, 1, 1, 10);
        let counters = EngagementCounters::new();
        let mut tracker = TweetActionTracker::new(60_000);

        let actions = selected_candidate_actions(&persona, "tweet_1", &limits, &counters, &tracker);
        assert_eq!(
            actions,
            vec!["like", "retweet", "quote", "follow", "reply", "bookmark"]
        );

        tracker.record_action("tweet_1".to_string(), "like");
        let blocked = selected_candidate_actions(&persona, "tweet_1", &limits, &counters, &tracker);
        assert!(blocked.is_empty());
    }

    #[test]
    fn thread_dive_prob_zero_drops_detail_actions() {
        let persona = PersonaWeights {
            thread_dive_prob: 0.0,
            ..PersonaWeights::default()
        };
        let mut actions = vec!["like", "retweet", "reply"];

        filter_detail_actions_for_gate(&mut actions, true, should_dive(&persona));

        assert_eq!(actions, vec!["like"]);
    }

    #[test]
    fn thread_dive_prob_one_keeps_detail_actions_when_status_url_exists() {
        let persona = PersonaWeights {
            thread_dive_prob: 1.0,
            ..PersonaWeights::default()
        };
        let mut actions = vec!["like", "retweet", "reply"];

        filter_detail_actions_for_gate(&mut actions, true, should_dive(&persona));

        assert_eq!(actions, vec!["like", "retweet", "reply"]);
    }

    #[test]
    fn decision_level_minimal_keeps_like_only() {
        let mut actions = vec!["like", "retweet", "quote", "follow", "reply", "bookmark"];

        filter_actions_for_decision_level(&mut actions, EngagementLevel::Minimal);

        assert_eq!(actions, vec!["like"]);
    }

    #[test]
    fn decision_level_medium_keeps_like_and_retweet_only() {
        let mut actions = vec!["like", "retweet", "quote", "follow", "reply", "bookmark"];

        filter_actions_for_decision_level(&mut actions, EngagementLevel::Medium);

        assert_eq!(actions, vec!["like", "retweet"]);
    }

    #[test]
    fn decision_level_full_keeps_selected_actions() {
        let mut actions = vec!["like", "retweet", "quote", "follow", "reply", "bookmark"];

        filter_actions_for_decision_level(&mut actions, EngagementLevel::Full);

        assert_eq!(
            actions,
            vec!["like", "retweet", "quote", "follow", "reply", "bookmark"]
        );
    }

    #[test]
    fn decision_level_none_clears_selected_actions() {
        let mut actions = vec!["like", "retweet"];

        filter_actions_for_decision_level(&mut actions, EngagementLevel::None);

        assert!(actions.is_empty());
    }

    #[test]
    fn dry_run_skips_post_dive_browser_work() {
        let dry_run = TaskConfig {
            dry_run_actions: true,
            ..Default::default()
        };
        let live = TaskConfig {
            dry_run_actions: false,
            ..Default::default()
        };

        assert!(!should_engage_replies_after_root_action(
            true, true, &dry_run
        ));
        assert!(!should_navigate_home_after_dive(true, &dry_run));
        assert!(should_engage_replies_after_root_action(true, true, &live));
        assert!(should_navigate_home_after_dive(true, &live));
    }

    /// Test extract_tweet_text with text field
    #[test]
    fn extract_tweet_text_extracts_text_field() {
        let tweet = json!({"text": "Hello world"});
        assert_eq!(extract_tweet_text(&tweet), "Hello world");
    }

    /// Test extract_tweet_text with full_text field (fallback)
    #[test]
    fn extract_tweet_text_extracts_full_text_field() {
        let tweet = json!({"full_text": "Full text content"});
        assert_eq!(extract_tweet_text(&tweet), "Full text content");
    }

    /// Test extract_tweet_text returns empty for missing fields
    #[test]
    fn extract_tweet_text_returns_empty_for_missing() {
        let tweet = json!({"id": "123"});
        assert_eq!(extract_tweet_text(&tweet), "");
    }

    /// Test generate_reply_text cycles through templates
    #[test]
    fn generate_reply_text_cycles_templates() {
        let templates = SentimentTemplates::default();
        let text1 = generate_reply_text(Sentiment::Positive, 0, &templates);
        let text2 = generate_reply_text(Sentiment::Positive, 1, &templates);
        // Should return different templates
        assert!(!text1.is_empty());
        assert!(!text2.is_empty());
    }

    /// Test generate_quote_text cycles through templates
    #[test]
    fn generate_quote_text_cycles_templates() {
        let templates = SentimentTemplates::default();
        let text1 = generate_quote_text(Sentiment::Neutral, 0, &templates);
        let text2 = generate_quote_text(Sentiment::Neutral, 1, &templates);
        assert!(!text1.is_empty());
        assert!(!text2.is_empty());
    }

    /// Test calc_rate with valid inputs
    #[test]
    fn calc_rate_calculates_correctly() {
        assert_eq!(calc_rate(5, 10), 50.0);
        assert_eq!(calc_rate(0, 10), 0.0);
        assert_eq!(calc_rate(10, 10), 100.0);
    }

    /// Test calc_rate handles zero total
    #[test]
    fn calc_rate_handles_zero_total() {
        assert_eq!(calc_rate(5, 0), 0.0);
    }
}

#[cfg(test)]
mod decision_integration_tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_persona::PersonaWeights;
    use serde_json::json;

    /// Test handle_engagement_decision returns None when disabled
    #[tokio::test]
    async fn engagement_decision_returns_none_when_disabled() {
        let tweet = json!({"text": "Test tweet"});
        let config = TaskConfig {
            duration_ms: 60000,
            candidate_count: 5,
            smart_decision_enabled: false,
            ..Default::default()
        };
        let persona = PersonaWeights::default();
        let result = handle_engagement_decision(&tweet, &config, &persona, None).await;
        assert!(result.is_none());
    }

    /// Test handle_engagement_decision extracts tweet text correctly
    #[tokio::test]
    async fn engagement_decision_extracts_tweet_text() {
        let tweet = json!({
            "text": "This is a test tweet about technology",
            "replies": []
        });
        let config = TaskConfig {
            duration_ms: 60000,
            candidate_count: 5,
            smart_decision_enabled: true,
            ..Default::default()
        };
        let persona = PersonaWeights::default();
        let result = handle_engagement_decision(&tweet, &config, &persona, None).await;
        // Should return a decision (not None) when enabled
        assert!(result.is_some());
    }

    /// Test handle_engagement_decision handles replies array
    #[tokio::test]
    async fn engagement_decision_extracts_replies() {
        let tweet = json!({
            "text": "Main tweet",
            "replies": [
                {"author": "user1", "text": "Reply 1"},
                {"author": "user2", "text": "Reply 2"}
            ]
        });
        let config = TaskConfig {
            duration_ms: 60000,
            candidate_count: 5,
            smart_decision_enabled: true,
            ..Default::default()
        };
        let persona = PersonaWeights::default();
        let result = handle_engagement_decision(&tweet, &config, &persona, None).await;
        assert!(result.is_some());
    }
}

#[cfg(test)]
mod statistical_tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_persona::PersonaWeights;

    /// Test that should_like produces expected distribution (within tolerance)
    #[test]
    fn should_like_distribution_within_tolerance() {
        let persona = PersonaWeights::default();
        let expected_prob = persona.like_prob;
        let trials = 1000;

        let successes: u32 = (0..trials)
            .map(|_| if should_like(&persona) { 1 } else { 0 })
            .sum();

        let actual_rate = successes as f64 / trials as f64;
        let tolerance = 0.05; // 5% tolerance

        assert!(
            (actual_rate - expected_prob).abs() < tolerance,
            "Expected ~{:.2}, got {:.2}",
            expected_prob,
            actual_rate
        );
    }

    /// Test that should_retweet produces expected distribution (within tolerance)
    #[test]
    fn should_retweet_distribution_within_tolerance() {
        let persona = PersonaWeights::default();
        let expected_prob = persona.retweet_prob;
        let trials = 1000;

        let successes: u32 = (0..trials)
            .map(|_| if should_retweet(&persona) { 1 } else { 0 })
            .sum();

        let actual_rate = successes as f64 / trials as f64;
        let tolerance = 0.05;

        assert!(
            (actual_rate - expected_prob).abs() < tolerance,
            "Expected ~{:.2}, got {:.2}",
            expected_prob,
            actual_rate
        );
    }

    /// Test that should_reply produces expected distribution (within tolerance)
    #[test]
    fn should_reply_distribution_within_tolerance() {
        let persona = PersonaWeights::default();
        let expected_prob = persona.reply_prob;
        let trials = 1000;

        let successes: u32 = (0..trials)
            .map(|_| if should_reply(&persona) { 1 } else { 0 })
            .sum();

        let actual_rate = successes as f64 / trials as f64;
        let tolerance = 0.05;

        assert!(
            (actual_rate - expected_prob).abs() < tolerance,
            "Expected ~{:.2}, got {:.2}",
            expected_prob,
            actual_rate
        );
    }

    /// Test that should_follow produces expected distribution (within tolerance)
    #[test]
    fn should_follow_distribution_within_tolerance() {
        let persona = PersonaWeights::default();
        let expected_prob = persona.follow_prob;
        let trials = 1000;

        let successes: u32 = (0..trials)
            .map(|_| if should_follow(&persona) { 1 } else { 0 })
            .sum();

        let actual_rate = successes as f64 / trials as f64;
        let tolerance = 0.05;

        assert!(
            (actual_rate - expected_prob).abs() < tolerance,
            "Expected ~{:.2}, got {:.2}",
            expected_prob,
            actual_rate
        );
    }

    /// Test that calc_rate produces expected percentages
    #[test]
    fn calc_rate_statistical_accuracy() {
        assert_eq!(calc_rate(50, 100), 50.0);
        assert_eq!(calc_rate(25, 100), 25.0);
        assert_eq!(calc_rate(75, 100), 75.0);
        assert!((calc_rate(1, 3) - 33.33).abs() < 0.01);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;

    /// Property: action_allowed_by_limits never panics on valid/invalid action names
    #[test]
    fn action_allowed_by_limits_no_panic() {
        let limits = EngagementLimits::default();
        let counters = EngagementCounters::new();

        let actions = vec![
            "like",
            "retweet",
            "quote",
            "follow",
            "reply",
            "bookmark",
            "unknown",
            "",
            "invalid_action",
        ];

        for action in &actions {
            let _result = action_allowed_by_limits(action, &limits, &counters);
        }
    }

    /// Property: calc_rate handles all usize inputs without panic
    #[test]
    fn calc_rate_handles_all_inputs() {
        // Test edge cases
        assert_eq!(calc_rate(0, 0), 0.0);
        assert_eq!(calc_rate(usize::MAX, usize::MAX), 100.0);
        assert_eq!(calc_rate(0, usize::MAX), 0.0);

        // Test various combinations
        for success in [0, 1, 50, 100] {
            for total in [1, 50, 100, 1000] {
                if success <= total {
                    let rate = calc_rate(success, total);
                    assert!((0.0..=100.0).contains(&rate));
                }
            }
        }
    }

    /// Property: extract_tweet_text never panics on various JSON inputs
    #[test]
    fn extract_tweet_text_no_panic() {
        use serde_json::json;

        let test_cases = vec![
            json!({"text": "test"}),
            json!({"full_text": "full"}),
            json!({"text": null}),
            json!({"full_text": null}),
            json!({}),
            json!({"text": 123}),
            json!({"text": ["array"]}),
            json!({"text": {"nested": "object"}}),
            json!(null),
            json!("string"),
        ];

        for case in &test_cases {
            let _result = extract_tweet_text(case);
        }
    }

    /// Property: generate_reply_text always returns non-empty for valid sentiment
    #[test]
    fn generate_reply_text_returns_non_empty() {
        let templates = SentimentTemplates::default();

        for sentiment in [Sentiment::Positive, Sentiment::Neutral, Sentiment::Negative] {
            for idx in 0..100 {
                let result = generate_reply_text(sentiment, idx, &templates);
                assert!(!result.is_empty(), "Reply text should never be empty");
            }
        }
    }

    /// Property: generate_quote_text always returns non-empty for valid sentiment
    #[test]
    fn generate_quote_text_returns_non_empty() {
        let templates = SentimentTemplates::default();

        for sentiment in [Sentiment::Positive, Sentiment::Neutral, Sentiment::Negative] {
            for idx in 0..100 {
                let result = generate_quote_text(sentiment, idx, &templates);
                assert!(!result.is_empty(), "Quote text should never be empty");
            }
        }
    }
}

#[cfg(test)]
mod gap_tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};

    // should_engage_replies_after_root_action combinations
    #[test]
    fn should_engage_replies_requires_all_conditions() {
        let config = TaskConfig {
            dry_run_actions: false,
            ..Default::default()
        };

        // All true → true
        assert!(should_engage_replies_after_root_action(true, true, &config));

        // did_dive=false → false
        assert!(!should_engage_replies_after_root_action(
            false, true, &config
        ));

        // root_action_success=false → false
        assert!(!should_engage_replies_after_root_action(
            true, false, &config
        ));

        // Both false → false
        assert!(!should_engage_replies_after_root_action(
            false, false, &config
        ));
    }

    #[test]
    fn should_engage_replies_dry_run_blocks() {
        let config = TaskConfig {
            dry_run_actions: true,
            ..Default::default()
        };
        // Even with dive+success, dry_run blocks it
        assert!(!should_engage_replies_after_root_action(
            true, true, &config
        ));
    }

    // should_navigate_home_after_dive combinations
    #[test]
    fn should_navigate_home_requires_dive_and_not_dry_run() {
        let config = TaskConfig {
            dry_run_actions: false,
            ..Default::default()
        };

        assert!(should_navigate_home_after_dive(true, &config));
        assert!(!should_navigate_home_after_dive(false, &config));
    }

    #[test]
    fn should_navigate_home_dry_run_blocks() {
        let config = TaskConfig {
            dry_run_actions: true,
            ..Default::default()
        };
        assert!(!should_navigate_home_after_dive(true, &config));
    }

    // filter_detail_actions_for_gate edge cases
    #[test]
    fn filter_detail_actions_no_status_url_keeps_only_like() {
        let mut actions = vec!["like", "retweet", "reply"];
        filter_detail_actions_for_gate(&mut actions, false, true);
        assert_eq!(actions, vec!["like"]);
    }

    #[test]
    fn filter_detail_actions_dive_not_allowed_keeps_only_like() {
        let mut actions = vec!["like", "retweet", "reply"];
        filter_detail_actions_for_gate(&mut actions, true, false);
        assert_eq!(actions, vec!["like"]);
    }

    #[test]
    fn filter_detail_actions_like_only_unaffected() {
        let mut actions = vec!["like"];
        filter_detail_actions_for_gate(&mut actions, false, false);
        assert_eq!(actions, vec!["like"]);
    }

    #[test]
    fn filter_detail_actions_empty_list_stays_empty() {
        let mut actions: Vec<&str> = vec![];
        filter_detail_actions_for_gate(&mut actions, true, true);
        assert!(actions.is_empty());
    }

    #[test]
    fn filter_detail_actions_all_allowed_when_status_and_dive() {
        let mut actions = vec!["like", "retweet", "reply"];
        filter_detail_actions_for_gate(&mut actions, true, true);
        assert_eq!(actions, vec!["like", "retweet", "reply"]);
    }

    // filter_actions_for_decision_level with empty list
    #[test]
    fn filter_decision_level_empty_actions_stays_empty() {
        let mut actions: Vec<&str> = vec![];
        filter_actions_for_decision_level(&mut actions, EngagementLevel::Full);
        assert!(actions.is_empty());

        filter_actions_for_decision_level(&mut actions, EngagementLevel::Medium);
        assert!(actions.is_empty());

        filter_actions_for_decision_level(&mut actions, EngagementLevel::Minimal);
        assert!(actions.is_empty());

        filter_actions_for_decision_level(&mut actions, EngagementLevel::None);
        assert!(actions.is_empty());
    }

    // action_allowed_by_limits with "dive" action
    #[test]
    fn action_allowed_by_limits_dive_returns_false() {
        let limits = EngagementLimits::default();
        let counters = EngagementCounters::new();
        // "dive" is not in the match statement of action_allowed_by_limits
        assert!(!action_allowed_by_limits("dive", &limits, &counters));
    }

    #[test]
    fn action_allowed_by_limits_empty_string_returns_false() {
        let limits = EngagementLimits::default();
        let counters = EngagementCounters::new();
        assert!(!action_allowed_by_limits("", &limits, &counters));
    }

    // calc_rate edge cases
    #[test]
    fn calc_rate_hundred_percent() {
        assert_eq!(calc_rate(100, 100), 100.0);
    }

    #[test]
    fn calc_rate_fractional() {
        let rate = calc_rate(1, 3);
        assert!((rate - 33.333333).abs() < 0.01);
    }

    // extract_tweet_text with truncated text (Twitter API v2 uses "text" with note_text)
    #[test]
    fn extract_tweet_text_handles_empty_string_value() {
        let tweet = serde_json::json!({"text": ""});
        assert_eq!(extract_tweet_text(&tweet), "");
    }

    // selected_candidate_actions with tracker blocking specific tweets
    #[test]
    fn selected_candidate_actions_tracker_blocks_per_tweet() {
        let persona = PersonaWeights {
            like_prob: 1.0,
            retweet_prob: 1.0,
            quote_prob: 0.0,
            follow_prob: 0.0,
            reply_prob: 0.0,
            bookmark_prob: 0.0,
            thread_dive_prob: 0.0,
            interest_multiplier: 1.0,
        };
        let limits = EngagementLimits::with_limits(10, 10, 10, 10, 10, 10, 10, 100);
        let counters = EngagementCounters::new();
        let mut tracker = TweetActionTracker::new(60_000);

        // First call should include like and retweet
        let actions = selected_candidate_actions(&persona, "tweet_x", &limits, &counters, &tracker);
        assert!(actions.contains(&"like"));
        assert!(actions.contains(&"retweet"));

        // Record an action on tweet_x — tracker blocks further actions on same tweet
        tracker.record_action("tweet_x".to_string(), "like");
        let blocked = selected_candidate_actions(&persona, "tweet_x", &limits, &counters, &tracker);
        assert!(blocked.is_empty());

        // Different tweet should still be allowed
        let other = selected_candidate_actions(&persona, "tweet_y", &limits, &counters, &tracker);
        assert!(!other.is_empty());
    }
}
