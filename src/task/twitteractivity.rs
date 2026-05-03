//! Twitter/X activity automation task orchestrator.
//!
//! This is a thin orchestrator (~100 lines) that delegates to modules in
//! `src/utils/twitter/`. The actual implementation lives in:
//! - `twitteractivity_navigation.rs` - Entry point selection, navigation
//! - `twitteractivity_engagement.rs` - Tweet processing and engagement
//! - `twitteractivity_feed.rs` - Feed scanning and candidate identification
//! - `twitteractivity_state.rs` - TaskConfig, CandidateContext, CandidateResult

use anyhow::Result;
use log::info;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::timeout;

use crate::config::Config;
use crate::prelude::TaskContext;
use crate::utils::twitter::{
    twitteractivity_engagement::process_candidate,
    twitteractivity_feed::identify_engagement_candidates,
    twitteractivity_limits::{EngagementCounters, EngagementLimits},
    twitteractivity_navigation::phase1_navigation,
    twitteractivity_persona::{apply_behavior_profile, select_persona_weights},
    twitteractivity_state::{CandidateContext, TaskConfig, TweetActionTracker},
};

/// Task entry point called by orchestrator.
///
/// # Responsibilities
/// - Extracts task configuration from JSON payload
/// - Applies timeout wrapper to prevent runaway tasks
/// - Delegates all implementation to `run_inner()`
///
/// # Arguments
/// * `api` - Task context with page, profile, clipboard
/// * `payload` - JSON task configuration
/// * `config` - Application configuration
///
/// # Timeout
/// The timeout wrapper ensures the task cannot exceed `duration_ms`
/// milliseconds. This is the correct boundary for timeout enforcement.
pub async fn run(api: &TaskContext, payload: Value, config: &Config) -> Result<()> {
    let task_config = TaskConfig::from_payload(&payload, &config.twitter_activity)
        .map_err(|e| anyhow::anyhow!("Payload validation failed: {}", e))?;
    let duration_ms = task_config.duration_ms;
    timeout(
        Duration::from_millis(duration_ms),
        run_inner(api, payload, config, task_config),
    )
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "twitteractivity exceeded task duration of {}ms",
            duration_ms
        )
    })?
}

/// Main task logic - thin orchestrator that delegates to utility modules.
///
/// # Responsibilities
/// - Phase 1: Navigation & authentication (via `twitteractivity_navigation::phase1_navigation`)
/// - Phase 2: Feed scanning loop with candidate identification
/// - Delegates engagement actions to `twitteractivity_engagement::process_candidate()`
///
/// # Architecture
/// This function is intentionally separate from `run()` to keep the timeout
/// boundary clean. The split allows `run()` to handle timeout enforcement
/// while `run_inner()` contains the actual task logic.
///
/// # Arguments
/// * `api` - Task context with page, profile, clipboard
/// * `_payload` - JSON task configuration (already parsed into `task_config`)
/// * `config` - Application configuration
/// * `task_config` - Pre-parsed task configuration
async fn run_inner(
    api: &TaskContext,
    _payload: Value,
    config: &Config,
    task_config: TaskConfig,
) -> Result<()> {
    info!("Task started");

    // Build persona weights
    let mut persona = select_persona_weights(
        task_config.weights.as_ref(),
        &config.twitter_activity.probabilities,
    );
    let profile = api.behavior_profile();
    persona = apply_behavior_profile(persona, profile, 0.0);

    info!(
        "Persona weights: like={:.2}, rt={:.2}, follow={:.2}, reply={:.2}",
        persona.like_prob, persona.retweet_prob, persona.follow_prob, persona.reply_prob
    );

    // Initialize engagement counters and limits
    let mut counters = EngagementCounters::new();
    let limits = EngagementLimits::with_limits(
        config.twitter_activity.engagement_limits.max_likes,
        config.twitter_activity.engagement_limits.max_retweets,
        config.twitter_activity.engagement_limits.max_follows,
        config.twitter_activity.engagement_limits.max_replies,
        config.twitter_activity.engagement_limits.max_thread_dives,
        config.twitter_activity.engagement_limits.max_bookmarks,
        config.twitter_activity.engagement_limits.max_quote_tweets,
        config.twitter_activity.engagement_limits.max_total_actions,
    );
    let mut action_tracker = TweetActionTracker::new(
        crate::utils::twitter::twitteractivity_constants::MIN_ACTION_CHAIN_DELAY_MS,
    );

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
    phase1_navigation(api).await?;

    // Phase 2: Feed scanning and engagement
    info!("Phase 2: Scanning feed for {} ms", task_config.duration_ms);
    let deadline = Instant::now() + Duration::from_millis(task_config.duration_ms);
    let mut actions_taken = 0u32;
    let mut last_remaining = Duration::from_millis(task_config.duration_ms);

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
        Duration::from_millis(
            crate::utils::twitter::twitteractivity_constants::MIN_CANDIDATE_SCAN_INTERVAL_MS,
        )
    };
    let mut next_scroll = Instant::now();
    let mut next_candidate_scan = Instant::now();

    while Instant::now() < deadline {
        let now = Instant::now();

        if now < next_candidate_scan {
            tokio::time::sleep(next_candidate_scan - now).await;
            continue;
        }

        // Scroll to load new content
        if now >= next_scroll {
            let _ = api
                .scroll_read(1, scroll_amount, smooth, profile.scroll.back_scroll)
                .await;
            next_scroll = now + scroll_interval;
        }

        // Identify candidate tweets
        let candidates = identify_engagement_candidates(api).await?;
        info!("Candidate scan | candidates={}", candidates.len());
        next_candidate_scan = Instant::now() + candidate_scan_interval;

        if !candidates.is_empty() {
            let to_consider = candidates
                .iter()
                .take(task_config.candidate_count as usize)
                .collect::<Vec<_>>();
            let mut actions_this_scan = 0u32;

            for tweet in to_consider {
                let ctx = CandidateContext {
                    tweet,
                    persona: &persona,
                    task_config: &task_config,
                    api,
                    limits: &limits,
                    scroll_interval,
                    action_tracker: &mut action_tracker,
                    counters: &mut counters,
                    thread_cache: None,
                };

                let result =
                    process_candidate(ctx, actions_this_scan, next_scroll, actions_taken).await?;
                let crate::utils::twitter::twitteractivity_state::CandidateResult {
                    should_break,
                    next_scroll: new_next_scroll,
                    actions_this_scan: new_actions_this_scan,
                    actions_taken: new_actions_taken,
                    thread_cache: _,
                } = result;

                next_scroll = new_next_scroll;
                actions_this_scan = new_actions_this_scan;
                actions_taken = new_actions_taken;

                if should_break {
                    break;
                }
            }
        }

        last_remaining = deadline.saturating_duration_since(Instant::now());
        if last_remaining.as_millis() < 500 {
            break;
        }
    }

    // Final summary
    log_summary(&counters, &limits, &task_config, last_remaining, api);
    Ok(())
}

/// Log final engagement summary.
fn log_summary(
    counters: &EngagementCounters,
    limits: &EngagementLimits,
    task_config: &TaskConfig,
    last_remaining: Duration,
    _api: &TaskContext,
) {
    let duration_secs =
        (Duration::from_millis(task_config.duration_ms) - last_remaining).as_secs_f64();
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

    let remaining_limits = limits.remaining(counters);
    info!(
        "[twitter] Remaining limits | likes={} retweets={} follows={} replies={} thread_dives={} bookmarks={} quote_tweets={} total_actions={}",
        remaining_limits.get("likes").unwrap_or(&0),
        remaining_limits.get("retweets").unwrap_or(&0),
        remaining_limits.get("follows").unwrap_or(&0),
        remaining_limits.get("replies").unwrap_or(&0),
        remaining_limits.get("thread_dives").unwrap_or(&0),
        remaining_limits.get("bookmarks").unwrap_or(&0),
        remaining_limits.get("quote_tweets").unwrap_or(&0),
        remaining_limits.get("total_actions").unwrap_or(&0)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::twitter::twitteractivity_navigation::select_entry_point;
    use serde_json::json;

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
            "enhanced_sentiment_enabled": false,
            "dry_run_actions": true
        });
        let twitter_config = crate::config::TwitterActivityConfig::default();
        let config = TaskConfig::from_payload(&payload, &twitter_config).unwrap();
        assert!(config.duration_ms >= 96_000 && config.duration_ms <= 144_000);
        assert_eq!(config.candidate_count, 10);
        assert_eq!(config.thread_depth, 15);
        assert_eq!(config.max_actions_per_scan, 5);
        assert!(config.weights.is_some());
        assert!(config.llm_enabled);
        assert!(config.smart_decision_enabled);
        assert!(!config.enhanced_sentiment_enabled);
        assert!(config.dry_run_actions);
    }

    #[test]
    fn test_select_entry_point_returns_valid_url() {
        let url = select_entry_point();
        let valid_urls: Vec<&str> = crate::utils::twitter::twitteractivity_navigation::ENTRY_POINTS
            .iter()
            .map(|ep| ep.url)
            .collect();
        assert!(valid_urls.contains(&url));
    }
}
