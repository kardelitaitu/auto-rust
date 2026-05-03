//! Engagement logic for Twitter activity task.
//! Contains process_candidate() and helper functions for tweet engagement.

use super::twitteractivity_state::*;
use crate::metrics::*;
use crate::prelude::TaskContext;
use crate::utils::mouse::hover_before_click;
use crate::utils::twitter::{
    twitteractivity_decision::*,
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
pub fn handle_engagement_decision(
    tweet: &Value,
    task_config: &TaskConfig,
) -> Option<EngagementDecision> {
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
                        replies.push((author_str.to_string(), text_str.to_string()));
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
    // Each tweet starts with a fresh cache. Only populated if this tweet dives.
    let mut current_thread_cache: Option<ThreadCache> = None;

    // Destructure ctx for easier access (preserve original variable names)
    let tweet = ctx.tweet;
    let persona = ctx.persona;
    let task_config = ctx.task_config;
    let api = ctx.api;
    let limits = ctx.limits;
    let scroll_interval = ctx.scroll_interval;
    let action_tracker = &mut ctx.action_tracker;
    let counters = &mut ctx.counters;
    let _current_thread_cache_unused = ctx.thread_cache;

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
            thread_cache: current_thread_cache,
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
    let engagement_decision = handle_engagement_decision(tweet, task_config);

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
                thread_cache: current_thread_cache,
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
            thread_cache: current_thread_cache,
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
                thread_cache: current_thread_cache,
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
                api.scroll_to_top().await?;
                human_pause(api, 800).await;
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

    // Perform the selected action.
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
                    warn!(
                        "Skipping retweet: not in thread detail view for tweet {}",
                        tweet_id
                    );
                    false
                } else {
                    match crate::utils::twitter::twitteractivity_interact::is_on_tweet_page(api)
                        .await
                    {
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
                                    get_tweet_context_for_llm(api, &current_thread_cache, "quote")
                                        .await;
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
                                    get_tweet_context_for_llm(api, &current_thread_cache, "reply")
                                        .await;
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
                    warn!(
                        "Skipping bookmark: not in thread detail view for tweet {}",
                        tweet_id
                    );
                    false
                } else {
                    match crate::utils::twitter::twitteractivity_interact::is_on_tweet_page(api)
                        .await
                    {
                        Ok(true) => bookmark_tweet(api).await?,
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

    Ok(CandidateResult {
        should_break: false,
        next_scroll,
        actions_this_scan,
        actions_taken: _actions_taken,
        thread_cache: current_thread_cache,
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

/// Helper: extract tweet context for LLM generation, using cache if available.
pub async fn get_tweet_context_for_llm(
    api: &TaskContext,
    cache: &Option<ThreadCache>,
    action_name: &str,
) -> (String, String, Vec<(String, String)>) {
    if let Some(ref cache) = cache {
        if cache.is_valid() {
            info!(
                "Using cached thread data for {} ({} replies)",
                action_name,
                cache.replies.len()
            );
            return (
                cache.tweet_author.clone(),
                cache.tweet_text.clone(),
                cache.replies.clone(),
            );
        }
    }
    match extract_tweet_context(api).await {
        Ok(data) => data,
        Err(e) => {
            warn!("Failed to extract tweet context for {}: {}", action_name, e);
            ("unknown".to_string(), String::new(), Vec::new())
        }
    }
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
