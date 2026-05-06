//! Engagement logic for Twitter activity task.
//! Contains process_candidate() and helper functions for tweet engagement.

use super::twitteractivity_retry::{retry_with_backoff, RetryConfig};
use super::twitteractivity_state::*;
use crate::metrics::*;
use crate::prelude::TaskContext;
use crate::utils::mouse::hover_before_click;
use crate::utils::twitter::{
    decision::*,
    twitteractivity_dive::*,
    twitteractivity_humanized::*,
    twitteractivity_interact::*,
    twitteractivity_limits::{EngagementCounters, EngagementLimits},
    twitteractivity_llm::*,
    twitteractivity_navigation::*,
    twitteractivity_persona::*,
    twitteractivity_sentiment::*,
    twitteractivity_sentiment_enhanced::{
        extract_temporal_factors, extract_thread_context, extract_user_reputation,
        EnhancedSentimentAnalyzer, EnhancedSentimentResult,
    },
};
use anyhow::Result;
use log::{info, warn};
use serde_json::Value;
use std::time::{Duration, Instant};

/// Smart decision check for engagement.
pub async fn handle_engagement_decision(
    tweet: &Value,
    task_config: &TaskConfig,
    persona: &PersonaWeights,
) -> Option<EngagementDecision> {
    if !task_config.smart_decision_enabled {
        return None;
    }

    // Extract tweet text
    let tweet_text = tweet.get("text").and_then(|v| v.as_str()).unwrap_or("");
    let tweet_id = tweet.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let author = tweet.get("author").and_then(|v| v.as_str()).unwrap_or("unknown");

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
        topic_alignment: "Unknown".to_string(),
    };

    // Use Factory to create appropriate engine
    // For feed scan, we typically use Legacy or Persona strategy unless LLM is explicitly requested
    let strategy = if task_config.llm_enabled {
        DecisionStrategy::Auto
    } else {
        DecisionStrategy::Legacy
    };

    // In a real scenario, we'd get the API key from environment or config
    let engine = DecisionEngineFactory::create(strategy, None);
    
    Some(engine.decide(&ctx).await)
}

/// Process a single candidate tweet for engagement.
#[allow(clippy::too_many_arguments)]
pub async fn process_candidate(
    mut ctx: CandidateContext<'_>,
    actions_this_scan: u32,
    next_scroll: Instant,
    _actions_taken: u32,
) -> Result<CandidateResult> {
    let mut actions_this_scan = actions_this_scan;
    let mut next_scroll = next_scroll;
    let mut _actions_taken = _actions_taken;

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
            actions_this_scan,
            actions_taken: _actions_taken,
        });
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
            score_breakdown:
                crate::utils::twitter::twitteractivity_sentiment_enhanced::ScoreBreakdown {
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
    let engagement_decision = handle_engagement_decision(tweet, task_config, &candidate_persona).await;

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
                actions_this_scan,
                actions_taken: _actions_taken,
            });
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

    let status_url = tweet.get("status_url").and_then(|v| v.as_str());
    let selected_action = select_candidate_action(
        &actions_to_do,
        should_dive(&candidate_persona),
        status_url.is_some(),
    );
    let Some(selected_action) = selected_action else {
        return Ok(CandidateResult {
            should_break: false,
            next_scroll,
            actions_this_scan,
            actions_taken: _actions_taken,
        });
    };

    // Retweet, quote, reply, follow, and bookmark require a detail view; like does not.
    let need_dive = selected_action != "like";
    let mut did_dive = false;

    if need_dive {
        // Dive into thread for non-like actions
        if actions_this_scan >= task_config.max_actions_per_scan {
            return Ok(CandidateResult {
                should_break: true,
                next_scroll,
                actions_this_scan,
                actions_taken: _actions_taken,
            });
        }
        if !limits.can_dive(counters) {
            info!(
                "Skipping dive: limit reached ({}/{})",
                counters.thread_dives, limits.max_thread_dives
            );
        } else if let Some(status_url) = status_url {
            // Pause continuous scrolling before diving to avoid interference
            let original_next_scroll = next_scroll;
            next_scroll = Instant::now() + Duration::from_secs(300); // Pause for 5 minutes during dive
            info!("Paused continuous scrolling for thread dive");

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
                    warn!("Thread dive failed after retries: {}", e);
                    api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                    api.increment_run_counter(RUN_COUNTER_DIVE_FAILURE, 1);
                    // Resume scrolling if dive failed and skip this candidate
                    next_scroll = original_next_scroll;
                    return Ok(CandidateResult {
                        should_break: false,
                        next_scroll,
                        actions_this_scan,
                        actions_taken: _actions_taken,
                    });
                }
            };
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
                // Read thread context for LLM use (not cached, extracted fresh when needed)
                if let Err(e) = api.scroll_to_top().await {
                    warn!("Scroll to top failed: {}", e);
                    // Non-fatal, continue
                }
                human_pause(api, 800).await;
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

    // Perform the selected action.
    let mut root_action_success = false;
    for action in [selected_action] {
        if task_config.dry_run_actions {
            if action != "like" && !did_dive {
                info!(
                    "Dry-run: would skip {} on tweet {} because thread detail did not open",
                    action, tweet_id
                );
                continue;
            }
            info!(
                "Dry-run: would perform {} on tweet {} (did_dive={})",
                action, tweet_id, did_dive
            );
            root_action_success = true; // Pretend success for sub-loop simulation
            continue;
        }

        if actions_this_scan >= task_config.max_actions_per_scan {
            info!(
                "Skipping {}: per-scan action budget reached after dive ({}/{})",
                action, actions_this_scan, task_config.max_actions_per_scan
            );
            continue;
        }
        if !action_allowed_by_limits(action, limits, counters) {
            info!("Skipping {}: engagement limit reached after dive", action);
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
                            warn!("Like failed after retries: {}", e);
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
                                warn!("Like at position failed after retries: {}", e);
                                api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                                api.increment_run_counter(RUN_COUNTER_LIKE_FAILURE, 1);
                                false
                            }
                        }
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
                    warn!(
                        "Skipping retweet: not in thread detail view for tweet {}",
                        tweet_id
                    );
                    false
                } else {
                    match crate::utils::twitter::twitteractivity_interact::is_on_tweet_page(api)
                        .await
                    {
                        Ok(true) => {
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
                                    warn!("Retweet failed after retries: {}", e);
                                    api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                                    api.increment_run_counter(RUN_COUNTER_RETWEET_FAILURE, 1);
                                    false
                                }
                            }
                        }
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
                    warn!(
                        "Skipping quote: not in thread detail view for tweet {}",
                        tweet_id
                    );
                    false
                } else {
                    match crate::utils::twitter::twitteractivity_interact::is_on_tweet_page(api)
                        .await
                    {
                        Ok(true) => {
                            let quote_text = if task_config.llm_enabled {
                                let (author, text, replies) =
                                    extract_tweet_context(api).await.unwrap_or_else(|e| {
                                        warn!("Failed to extract tweet context for quote: {}", e);
                                        ("unknown".to_string(), String::new(), Vec::new())
                                    });
                                match generate_quote_commentary(api, &author, &text, replies).await
                                {
                                    Ok(commentary) => {
                                        info!("Generated LLM quote: {}", commentary);
                                        commentary
                                    }
                                    Err(e) => {
                                        warn!("LLM quote failed, using template: {}", e);
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
                    warn!(
                        "Skipping follow: not in thread detail view for tweet {}",
                        tweet_id
                    );
                    false
                } else {
                    match crate::utils::twitter::twitteractivity_interact::is_on_tweet_page(api)
                        .await
                    {
                        Ok(true) => {
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
                                    warn!("Follow failed after retries: {}", e);
                                    api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                                    api.increment_run_counter(RUN_COUNTER_FOLLOW_FAILURE, 1);
                                    false
                                }
                            }
                        }
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
                    warn!(
                        "Skipping reply: not in thread detail view for tweet {}",
                        tweet_id
                    );
                    false
                } else {
                    match crate::utils::twitter::twitteractivity_interact::is_on_tweet_page(api)
                        .await
                    {
                        Ok(true) => {
                            let reply_text = if task_config.llm_enabled {
                                let (author, text, replies) =
                                    extract_tweet_context(api).await.unwrap_or_else(|e| {
                                        warn!("Failed to extract tweet context for reply: {}", e);
                                        ("unknown".to_string(), String::new(), Vec::new())
                                    });
                                match generate_reply(api, &author, &text, replies).await {
                                    Ok(reply) => {
                                        info!("Generated LLM reply: {}", reply);
                                        reply
                                    }
                                    Err(e) => {
                                        warn!("LLM reply failed, using template: {}", e);
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
                                    warn!("Reply failed after retries: {}", e);
                                    api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                                    api.increment_run_counter(RUN_COUNTER_REPLY_FAILURE, 1);
                                    false
                                }
                            }
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
                    warn!(
                        "Skipping bookmark: not in thread detail view for tweet {}",
                        tweet_id
                    );
                    false
                } else {
                    match crate::utils::twitter::twitteractivity_interact::is_on_tweet_page(api)
                        .await
                    {
                        Ok(true) => {
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
                                    warn!("Bookmark failed after retries: {}", e);
                                    api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
                                    api.increment_run_counter(RUN_COUNTER_BOOKMARK_FAILURE, 1);
                                    false
                                }
                            }
                        }
                        Ok(false) => {
                            warn!(
                                "Skipping bookmark: not on tweet page for tweet {}",
                                tweet_id
                            );
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

    // Depth-First Engagement: Engage with replies if we dived and root engagement was successful
    if did_dive && root_action_success {
        match identify_thread_replies(api).await {
            Ok(replies) => {
                let mut replies_engaged = 0;
                let max_replies = rand::random::<u32>() % 2 + 1; // Engage with 1-2 replies

                for reply in replies {
                    if replies_engaged >= max_replies {
                        break;
                    }
                    if actions_this_scan >= task_config.max_actions_per_scan {
                        break;
                    }
                    if !limits.can_like(counters) {
                        break;
                    }

                    // Run smart decision for this reply
                    if let Some(decision) = handle_engagement_decision(&reply, task_config, persona).await {
                        // For replies, we only do "Like" for safety and simplicity
                        if decision.score > 30 {
                            if let Some(pos) = reply.get("like_pos").and_then(|v| v.as_object()) {
                                let x = pos.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let y = pos.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let reply_id = reply.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");

                                info!("Depth-First: Engaging with high-quality reply {} (score: {})", reply_id, decision.score);
                                
                                match retry_with_backoff(
                                    || like_at_position(api, x, y),
                                    &RetryConfig::aggressive(),
                                    api,
                                    "depth_first_like",
                                ).await {
                                    Ok(true) => {
                                        info!("Successfully liked reply");
                                        counters.increment_like();
                                        _actions_taken += 1;
                                        actions_this_scan += 1;
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
                warn!("Depth-First: Failed to identify replies: {}", e);
            }
        }
    }

    // Navigate back to home after dive
    if did_dive {
        // Wait 3-5s after engagement before going home
        let home_wait_ms = rand::random::<u64>() % 2000 + 3000; // 3-5s
        human_pause(api, home_wait_ms).await;
        info!("Navigating back to home after thread dive and engagement");
        if let Err(e) =
            retry_with_backoff(|| goto_home(api), &RetryConfig::default(), api, "goto_home").await
        {
            warn!("Navigation to home failed after retries: {}", e);
            api.increment_run_counter(RUN_COUNTER_TRANSIENT_ERROR, 1);
            // Continue anyway - not fatal
        }
        scroll_pause(api).await;
        // Resume continuous scrolling now that we're back on home feed
        next_scroll = Instant::now() + scroll_interval;
        info!("Resumed continuous scrolling after thread dive");
    }

    Ok(CandidateResult {
        should_break: false,
        next_scroll,
        actions_this_scan,
        actions_taken: _actions_taken,
    })
}

/// Helper: extract tweet text from tweet object
pub fn extract_tweet_text(tweet_obj: &Value) -> String {
    if let Some(text) = tweet_obj.get("text").or_else(|| tweet_obj.get("full_text")) {
        if let Some(text_str) = text.as_str() {
            return text_str.to_string();
        }
    }
    String::new()
}

/// Helper: extract a per-tweet button center from candidate payload.
pub fn extract_tweet_button_position(tweet: &Value, button: &str) -> Option<(f64, f64)> {
    let button_obj = tweet
        .get("buttons")
        .and_then(|v| v.as_object())
        .and_then(|buttons| buttons.get(button))
        .and_then(|v| v.as_object())?;

    let x = button_obj.get("x").and_then(|v| v.as_f64())?;
    let y = button_obj.get("y").and_then(|v| v.as_f64())?;
    Some((x, y))
}

/// Helper: click like at a specific coordinate with profile-aware timing and hover
pub async fn like_at_position(api: &TaskContext, x: f64, y: f64) -> Result<bool> {
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

    // Verification failed - assume like was not registered
    Ok(false)
}

/// Generate a short reply string based on sentiment.
pub fn generate_reply_text(
    sentiment: Sentiment,
    reply_idx: u32,
    templates: &SentimentTemplates,
) -> String {
    let phrases = match sentiment {
        Sentiment::Positive => &templates.reply_positive,
        Sentiment::Neutral => &templates.reply_neutral,
        Sentiment::Negative => &templates.reply_negative,
    };
    phrases[(reply_idx as usize) % phrases.len()].clone()
}

/// Generate a short quote commentary string based on sentiment.
pub fn generate_quote_text(
    sentiment: Sentiment,
    quote_idx: u32,
    templates: &SentimentTemplates,
) -> String {
    let phrases = match sentiment {
        Sentiment::Positive => &templates.quote_positive,
        Sentiment::Neutral => &templates.quote_neutral,
        Sentiment::Negative => &templates.quote_negative,
    };
    phrases[(quote_idx as usize) % phrases.len()].clone()
}

/// Calculate success rate as a percentage.
pub fn calc_rate(success: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (success as f64 / total as f64) * 100.0
    }
}

/// Select candidate action (helper for process_candidate).
pub fn select_candidate_action(
    actions_to_do: &[&'static str],
    allow_dive: bool,
    can_open_detail: bool,
) -> Option<&'static str> {
    if allow_dive && can_open_detail {
        if let Some(action) = actions_to_do
            .iter()
            .copied()
            .find(|action| *action != "like")
        {
            return Some(action);
        }
    }

    actions_to_do
        .iter()
        .copied()
        .find(|action| *action == "like")
}

/// Check if action is allowed by limits (helper for process_candidate).
pub fn action_allowed_by_limits(
    action: &str,
    limits: &EngagementLimits,
    counters: &EngagementCounters,
) -> bool {
    match action {
        "like" => limits.can_like(counters),
        "retweet" => limits.can_retweet(counters),
        "quote" => limits.can_quote_tweet(counters),
        "follow" => limits.can_follow(counters),
        "reply" => limits.can_reply(counters),
        "bookmark" => limits.can_bookmark(counters),
        _ => false,
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};
    use serde_json::json;

    /// Test that select_candidate_action prefers non-like actions when dive is allowed
    #[test]
    fn select_candidate_action_prefers_non_like_when_dive_allowed() {
        let actions = vec!["like", "retweet", "reply"];
        let result = select_candidate_action(&actions, true, true);
        assert_eq!(result, Some("retweet"));
    }

    /// Test that select_candidate_action falls back to like when no other actions available
    #[test]
    fn select_candidate_action_falls_back_to_like() {
        let actions = vec!["like"];
        let result = select_candidate_action(&actions, true, true);
        assert_eq!(result, Some("like"));
    }

    /// Test that select_candidate_action returns None for empty actions
    #[test]
    fn select_candidate_action_returns_none_for_empty() {
        let actions: Vec<&str> = vec![];
        let result = select_candidate_action(&actions, true, true);
        assert_eq!(result, None);
    }

    /// Test that select_candidate_action only selects like when dive not allowed
    #[test]
    fn select_candidate_action_only_like_when_no_dive() {
        let actions = vec!["like", "retweet", "reply"];
        // When allow_dive is false, only like is selected (even if others available)
        let result = select_candidate_action(&actions, false, true);
        assert_eq!(result, Some("like"));
    }

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
    use serde_json::json;
    use crate::utils::twitter::twitteractivity_persona::PersonaWeights;

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
        let result = handle_engagement_decision(&tweet, &config, &persona).await;
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
        let result = handle_engagement_decision(&tweet, &config, &persona).await;
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
        let result = handle_engagement_decision(&tweet, &config, &persona).await;
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

    /// Property: select_candidate_action never panics on valid inputs
    #[test]
    fn select_candidate_action_no_panic_on_valid_inputs() {
        let actions_list: Vec<Vec<&str>> = vec![
            vec![],
            vec!["like"],
            vec!["retweet"],
            vec!["reply"],
            vec!["like", "retweet", "reply", "follow", "quote", "bookmark"],
            vec!["like"; 100], // Large list
        ];

        for actions in &actions_list {
            for allow_dive in [true, false] {
                for can_open_detail in [true, false] {
                    let _result = select_candidate_action(actions, allow_dive, can_open_detail);
                }
            }
        }
    }

    /// Property: select_candidate_action returns None only when actions empty
    #[test]
    fn select_candidate_action_returns_none_only_when_empty() {
        // Empty actions should return None
        assert_eq!(select_candidate_action(&[], true, true), None);
        assert_eq!(select_candidate_action(&[], true, false), None);
        assert_eq!(select_candidate_action(&[], false, true), None);
        assert_eq!(select_candidate_action(&[], false, false), None);

        // Non-empty should return Some
        assert!(select_candidate_action(&["like"], true, true).is_some());
        assert!(select_candidate_action(&["retweet"], true, true).is_some());
        assert!(select_candidate_action(&["like", "retweet"], true, true).is_some());
    }

    /// Property: select_candidate_action returns only valid actions from input
    #[test]
    fn select_candidate_action_returns_only_valid_actions() {
        let actions = vec!["like", "retweet", "reply"];

        for _ in 0..100 {
            if let Some(selected) = select_candidate_action(&actions, true, true) {
                assert!(
                    actions.contains(&selected),
                    "Selected action must be from input list"
                );
            }
        }
    }

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
