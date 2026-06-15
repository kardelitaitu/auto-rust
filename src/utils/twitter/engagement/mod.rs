//! Engagement logic for Twitter activity task.
//! Contains `process_candidate()` and helper functions for tweet engagement.

use super::twitteractivity_retry::{retry_with_backoff, RetryConfig};
use super::twitteractivity_state::{CandidateContext, CandidateResult, TaskConfig};
use crate::metrics::{
    RUN_COUNTER_DIVE_FAILURE, RUN_COUNTER_DIVE_SUCCESS, RUN_COUNTER_DIVE_TARGET_FALLBACK_USED,
    RUN_COUNTER_LIKE_SUCCESS, RUN_COUNTER_TRANSIENT_ERROR,
};
use crate::prelude::TaskContext;
use crate::utils::twitter::{
    decision::EngagementLevel,
    twitteractivity_dive::{dive_into_thread, identify_thread_replies},
    twitteractivity_humanized::{human_pause, scroll_pause},
    twitteractivity_limits::{EngagementCounters, EngagementLimits},
    twitteractivity_navigation::goto_home,
    twitteractivity_persona::{should_dive, PersonaWeights},
};
use anyhow::Result;
use log::{info, warn};
use rand::Rng;
use std::time::{Duration, Instant};

use super::twitteractivity_types::{EngagementOutcome, TweetId};

// Submodules
pub mod dispatch;
pub mod scoring;
#[cfg(test)]
mod tests;

// Re-exports from scoring
pub use crate::utils::twitter::sentiment::Sentiment;
pub use scoring::handle_engagement_decision;
pub(crate) use scoring::modulate_persona_by_sentiment;
// Re-exports from dispatch
pub use dispatch::dispatch_action;

pub use super::twitteractivity_actions::{
    extract_tweet_button_position, generate_quote_text, generate_reply_text, like_at_position,
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
                                Ok(EngagementOutcome::Completed) => {
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
    let (sentiment, candidate_persona) =
        modulate_persona_by_sentiment(tweet, task_config, persona).await;

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

    let raw_id = tweet
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("tweet missing 'id' field"))?;
    let tweet_id = TweetId::new(raw_id).map_err(anyhow::Error::msg)?;
    let mut actions_to_do = selected_candidate_actions(
        &candidate_persona,
        &tweet_id,
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
                counters.thread_dives(),
                limits.max_thread_dives
            );
        } else if task_config.dry_run_actions {
            info!("Dry-run: would dive into thread for tweet {tweet_id}");
            counters.increment_thread_dive();
            actions_this_scan += 1;
            action_tracker.record_action(tweet_id.clone(), "dive");
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
                action_tracker.record_action(tweet_id.clone(), "dive");
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

        match dispatch_action(
            api,
            action,
            tweet,
            &tweet_id,
            did_dive,
            sentiment,
            task_config,
            counters,
            action_tracker,
            &mut actions_this_scan,
        )
        .await
        {
            Ok(true) => {
                root_action_success = true;
            }
            _ => { /* Error or false returned */ }
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
