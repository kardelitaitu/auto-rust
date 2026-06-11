//! Twitter/X activity automation task orchestrator.
//!
//! This is a thin orchestrator (~300 lines) that delegates to modules in
//! `src/utils/twitter/`. The actual implementation lives in:
//! - `twitteractivity_navigation.rs` - Entry point selection, navigation
//! - `twitteractivity_engagement.rs` - Tweet processing and engagement
//! - `twitteractivity_feed.rs` - Feed scanning and candidate identification
//! - `twitteractivity_state.rs` - `TaskConfig`, `CandidateContext`, `CandidateResult`

use anyhow::Result;
use log::{error, info, warn};
use serde_json::Value;
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::prelude::TaskContext;
use crate::utils::profile::BrowserProfile;
use crate::utils::timing::run_with_timeout;
// Runtime thresholds for consecutive failures/empty scans are driven
// by config.twitter_activity fields. The MAX_CONSECUTIVE_* constants
// in twitteractivity_constants.rs serve as defaults for new configs.
use crate::utils::twitter::{
    twitteractivity_engagement::process_candidate,
    twitteractivity_feed::identify_engagement_candidates,
    twitteractivity_limits::EngagementLimits,
    twitteractivity_navigation::phase1_navigation,
    twitteractivity_persona::{apply_behavior_profile, select_persona_weights},
    twitteractivity_simulation::run_simulation,
    twitteractivity_state::{CandidateContext, SessionState, TaskConfig},
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
/// The timeout wrapper ensures the task cannot exceed `duration_ms` milliseconds. This is the correct boundary for timeout enforcement.
pub async fn run(api: &TaskContext, payload: Value, config: &Config) -> Result<()> {
    let task_config = TaskConfig::from_payload(&payload, &config.twitter_activity)
        .map_err(|e| anyhow::anyhow!("Payload validation failed: {e}"))?;
    if task_config.simulate_only {
        return run_simulation(&task_config, config);
    }
    let duration_ms = task_config.duration_ms;
    run_with_timeout(
        duration_ms,
        "twitteractivity",
        run_inner(api, config, task_config),
    )
    .await
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
/// * `config` - Application configuration
/// * `task_config` - Pre-parsed task configuration
///
/// Build persona weights from config and behavior profile.
async fn build_persona(
    profile: &BrowserProfile,
    task_config: &TaskConfig,
    config: &Config,
) -> crate::utils::twitter::twitteractivity_persona::PersonaWeights {
    let mut persona = select_persona_weights(
        task_config.weights.as_ref(),
        &config.twitter_activity.probabilities,
    );
    persona = apply_behavior_profile(persona, profile, 0.0);

    info!(
        "Persona weights: like={:.2}, rt={:.2}, follow={:.2}, reply={:.2}",
        persona.like_prob, persona.retweet_prob, persona.follow_prob, persona.reply_prob
    );
    persona
}

/// Initialize session state with engagement limits and deadline.
fn init_session(config: &Config, task_config: &TaskConfig) -> SessionState {
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
    let session = SessionState::new(
        limits,
        task_config.duration_ms,
        crate::utils::twitter::twitteractivity_constants::MIN_ACTION_CHAIN_DELAY_MS,
    );

    info!(
        "Engagement limits: likes={}/{}, retweets={}/{}, follows={}/{}, total={}/{}",
        session.counters.likes,
        session.limits.max_likes,
        session.counters.retweets,
        session.limits.max_retweets,
        session.counters.follows,
        session.limits.max_follows,
        session.counters.total_actions(),
        session.limits.max_total_actions
    );
    session
}

fn should_continue_feed_loop(
    session: &SessionState,
    scrolls_performed: u32,
    task_config: &TaskConfig,
) -> bool {
    !session.is_expired() && scrolls_performed < task_config.scroll_count
}

/// Scroll the feed and track consecutive failures.
/// Returns `true` if the scroll succeeded or is a retryable failure.
/// Returns `false` if too many consecutive failures — caller should stop.
async fn scroll_feed(
    api: &TaskContext,
    scroll_amount: i32,
    smooth: bool,
    back_scroll: bool,
    scroll_pause_ms: u64,
    consecutive_failures: &mut u32,
    max_consecutive_failures: u32,
) -> bool {
    match api.scroll_read(1, scroll_amount, smooth, back_scroll).await {
        Ok(()) => {
            *consecutive_failures = 0;
            api.pause(scroll_pause_ms).await;
        }
        Err(err) => {
            *consecutive_failures += 1;
            warn!(
                "[twitter] Scroll failed (attempt {}): {}",
                *consecutive_failures, err
            );
            if *consecutive_failures >= max_consecutive_failures {
                error!("[twitter] Too many consecutive scroll failures, stopping task");
                return false;
            }
            api.pause(scroll_pause_ms).await;
        }
    }
    true
}

/// Identify and process candidate tweets, returning whether to continue.
async fn scan_and_process_candidates(
    api: &TaskContext,
    persona: &crate::utils::twitter::twitteractivity_persona::PersonaWeights,
    task_config: &TaskConfig,
    session: &mut SessionState,
    scroll_interval: Duration,
    next_scroll: &mut Instant,
    next_candidate_scan: &mut Instant,
) -> Result<bool> {
    let candidates = identify_engagement_candidates(api).await?;
    info!("Candidate scan | candidates={}", candidates.len());

    if candidates.is_empty() {
        return Ok(false);
    }

    let to_consider = candidates
        .iter()
        .take(task_config.candidate_count as usize)
        .collect::<Vec<_>>();

    let mut actions_this_scan = 0u32;

    for tweet in to_consider {
        let ctx = CandidateContext {
            tweet,
            persona,
            task_config,
            api,
            limits: &session.limits,
            scroll_interval,
            action_tracker: &mut session.action_tracker,
            counters: &mut session.counters,
        };

        let result =
            process_candidate(ctx, actions_this_scan, *next_scroll, *next_candidate_scan).await?;
        let crate::utils::twitter::twitteractivity_state::CandidateResult {
            should_break,
            next_scroll: new_next_scroll,
            next_candidate_scan: new_next_candidate_scan,
            actions_this_scan: new_actions_this_scan,
        } = result;

        *next_scroll = new_next_scroll;
        *next_candidate_scan = new_next_candidate_scan;
        actions_this_scan = new_actions_this_scan;

        if should_break {
            break;
        }
    }

    Ok(true)
}

/// Main task logic — thin orchestrator that delegates to utility modules.
async fn run_inner(api: &TaskContext, config: &Config, task_config: TaskConfig) -> Result<()> {
    info!("Task started");

    // Build persona weights from behavior profile
    let persona = build_persona(api.behavior_profile(), &task_config, config).await;

    // Initialize session state
    let mut session = init_session(config, &task_config);

    // Phase 1: Navigation & authentication check
    phase1_navigation(api).await?;

    // Phase 2: Feed scanning and engagement
    info!("Phase 2: Scanning feed for {} ms", task_config.duration_ms);
    let mut consecutive_scroll_failures = 0u32;
    let mut consecutive_empty_scans = 0u32;
    let mut scrolls_performed = 0u32;

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
    let mut next_scroll = Instant::now() + scroll_interval;
    let mut next_candidate_scan = Instant::now();

    while should_continue_feed_loop(&session, scrolls_performed, &task_config) {
        let now = Instant::now();

        // Sleep if not yet time for a candidate scan
        if now < next_candidate_scan {
            let sleep_duration = next_candidate_scan - now;
            let max_sleep = Duration::from_millis(250);
            let remaining = session.remaining_time();
            let chunk = sleep_duration.min(max_sleep).min(remaining);
            if chunk > Duration::from_millis(0) {
                tokio::time::sleep(chunk).await;
            }
            continue;
        }

        // Scroll to load new content
        if now >= next_scroll {
            if !scroll_feed(
                api,
                scroll_amount,
                smooth,
                profile.scroll.back_scroll,
                scroll_pause_ms,
                &mut consecutive_scroll_failures,
                config.twitter_activity.max_consecutive_scroll_failures,
            )
            .await
            {
                break;
            }
            scrolls_performed += 1;
            next_scroll = Instant::now() + scroll_interval;
        }

        // Identify and process candidate tweets
        next_candidate_scan = Instant::now() + candidate_scan_interval;
        if scan_and_process_candidates(
            api,
            &persona,
            &task_config,
            &mut session,
            scroll_interval,
            &mut next_scroll,
            &mut next_candidate_scan,
        )
        .await?
        {
            consecutive_empty_scans = 0;
        } else {
            consecutive_empty_scans += 1;
            warn!("[twitter] No candidates found (attempt {consecutive_empty_scans})");
            if consecutive_empty_scans >= config.twitter_activity.max_consecutive_empty_scans {
                error!("[twitter] Too many empty scans, stopping task");
                break;
            }
        }

        if session.remaining_time().as_millis() < 500 {
            break;
        }
    }

    // Final summary
    log_summary(&session, &task_config, config);
    Ok(())
}

/// Log final engagement summary including guard threshold values.
fn log_summary(session: &SessionState, task_config: &TaskConfig, config: &Config) {
    let (summary_line, remaining_limits_line) =
        session.build_summary_lines(task_config.duration_ms);
    info!("{summary_line}");
    info!("{remaining_limits_line}");
    info!(
        "[twitter] Guard thresholds | max_scroll_failures={} max_empty_scans={}",
        config.twitter_activity.max_consecutive_scroll_failures,
        config.twitter_activity.max_consecutive_empty_scans,
    );
}

#[cfg(test)]
mod tdd_tests {
    use crate::tests::twitter_helpers::*;
    use crate::utils::twitter::twitteractivity_limits::{EngagementCounters, EngagementLimits};

    // ====================================================================
    // RED Tests — describe desired behavior (expected to fail on first run)
    // These tests demonstrate desired behavior that may not yet pass.
    // Run with: .\run-twitter-tests.ps1 -Red
    // ====================================================================

    #[test]
    fn tdd_red_session_limits_block_after_exact_count() {
        // RED: Verifies that limits block at precise boundary
        let limits = EngagementLimits::with_limits(3, 5, 2, 1, 3, 2, 2, 20);
        let mut counters = EngagementCounters::new();

        for _ in 0..3 {
            counters.increment_like();
        }

        assert!(
            !limits.can_like(&counters),
            "4th like should be blocked when max_likes = 3"
        );
        assert!(
            limits.can_retweet(&counters),
            "retweet should still be allowed when only like limit reached"
        );
    }

    // ====================================================================
    // GREEN Tests — validate working behavior
    // Run with: .\run-twitter-tests.ps1 -Green
    // ====================================================================

    #[test]
    fn tdd_green_session_state_creation() {
        let session = test_session_state();
        assert_session_valid(&session, 10);
    }

    #[test]
    fn tdd_green_session_tracks_actions() {
        let mut session = test_session_state();

        session.record_action("tweet_1", "like");
        session.record_action("tweet_2", "retweet");
        session.record_action("tweet_3", "follow");

        assert_eq!(session.counters.total_actions(), 3);
        assert_eq!(session.counters.likes, 1);
        assert_eq!(session.counters.retweets, 1);
        assert_eq!(session.counters.follows, 1);
    }

    #[test]
    fn tdd_green_session_blocked_after_per_action_limit_reached() {
        // is_action_allowed checks per-action limits (not total limit)
        // Set max_likes = 2 to test per-action boundary
        let mut session = test_session_state_with_limits(2, 5, 5, 5, 5, 5, 5, 10, 60_000);

        session.record_action("t1", "like");
        session.record_action("t2", "like");

        assert_eq!(session.counters.total_actions(), 2);
        assert_eq!(session.counters.likes, 2);

        // Per-action limit reached — is_action_allowed should block
        assert_action_blocked(&session, "like");

        // But other actions should still be allowed
        assert!(
            session.is_action_allowed("retweet"),
            "Retweet should still be allowed"
        );
        assert!(
            session.is_action_allowed("follow"),
            "Follow should still be allowed"
        );
    }

    #[test]
    fn tdd_green_persona_weights_have_expected_defaults() {
        let weights = test_persona_weights();

        assert!((0.0..=1.0).contains(&weights.like_prob));
        assert!((0.0..=1.0).contains(&weights.retweet_prob));
    }

    #[test]
    fn tdd_green_session_action_summary_format() {
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

    // ====================================================================
    // EDGE Case Tests
    // ====================================================================

    #[test]
    fn tdd_edge_action_tracker_zero_delay() {
        let mut tracker = test_action_tracker(0);
        tracker.record_action("tweet_id".to_string(), "like");
        assert!(
            tracker.can_perform_action("tweet_id"),
            "Zero delay should allow immediate second action"
        );
    }

    #[test]
    fn tdd_edge_counters_no_actions() {
        let counters = test_counters_with_actions(0, 0, 0, 0);
        assert_eq!(counters.total_actions(), 0);
        let limits = EngagementLimits::default();
        assert_all_actions_allowed(&limits, &counters);
    }

    #[test]
    fn tdd_edge_session_is_action_allowed_unknown() {
        let session = test_session_state();
        assert!(!session.is_action_allowed("unknown_action"));
    }

    #[test]
    fn tdd_edge_action_tracker_unknown_tweet() {
        let tracker = test_action_tracker(1000);
        assert!(tracker.can_perform_action("unknown_tweet"));
    }

    // ====================================================================
    // REGRESSION Tests
    // ====================================================================

    #[test]
    fn tdd_regression_session_state_not_expired_on_creation() {
        let session = test_session_state();
        assert_session_valid(&session, 10);
    }

    #[test]
    fn tdd_regression_limit_saturation_no_panic() {
        let limits = EngagementLimits::default();
        let mut counters = EngagementCounters::new();

        for _ in 0..1000 {
            counters.increment_like();
        }

        let remaining = limits.remaining(&counters);
        assert_eq!(
            remaining.get("likes").copied().unwrap_or(0),
            0,
            "Remaining should saturate at 0"
        );
        assert!(
            !limits.can_like(&counters),
            "Should not allow like when saturated"
        );
    }
}

#[cfg(test)]
mod test_support {
    use serde_json::{json, Value};

    pub fn twitter_config() -> crate::config::TwitterActivityConfig {
        crate::config::TwitterActivityConfig::default()
    }

    pub fn payload_with_all_fields() -> Value {
        json!({
            "duration_ms": 120000,
            "candidate_count": 10,
            "thread_depth": 15,
            "max_actions_per_scan": 5,
            "weights": { "like_prob": 0.5 },
            "llm_enabled": true,
            "smart_decision_enabled": true,
            "enhanced_sentiment_enabled": false,
            "dry_run_actions": true
        })
    }
}

#[cfg(test)]
mod config_tests {
    use super::test_support::{payload_with_all_fields, twitter_config};
    use super::TaskConfig;

    #[test]
    fn task_config_from_payload_with_all_fields() {
        let config =
            TaskConfig::from_payload(&payload_with_all_fields(), &twitter_config()).unwrap();
        assert!(config.duration_ms >= 96_000 && config.duration_ms <= 144_000);
        assert_eq!(config.candidate_count, 10);
        assert_eq!(config.thread_depth, 15);
        assert_eq!(config.max_actions_per_scan, 5);
        assert!(config.weights.is_some());
        assert!(config.llm_enabled);
        assert!(config.smart_decision_enabled);
        assert!(!config.enhanced_sentiment_enabled);
        assert!(config.dry_run_actions);
        assert!(!config.simulate_only);
    }
}

#[cfg(test)]
mod navigation_tests {
    use crate::utils::twitter::twitteractivity_navigation::{select_entry_point, ENTRY_POINTS};

    #[test]
    fn select_entry_point_returns_valid_url() {
        let url = select_entry_point();
        let valid_urls: Vec<&str> = ENTRY_POINTS.iter().map(|ep| ep.url).collect();
        assert!(valid_urls.contains(&url));
    }
}

#[cfg(test)]
mod summary_tests {
    use super::{should_continue_feed_loop, TaskConfig};
    use crate::utils::twitter::twitteractivity_limits::EngagementLimits;
    use crate::utils::twitter::twitteractivity_state::SessionState;

    #[test]
    fn feed_loop_stops_when_scroll_count_is_reached() {
        let session = SessionState::new(EngagementLimits::default(), 60_000, 100);
        let task_config = TaskConfig {
            scroll_count: 2,
            ..Default::default()
        };

        assert!(should_continue_feed_loop(&session, 1, &task_config));
        assert!(!should_continue_feed_loop(&session, 2, &task_config));
    }
}

#[cfg(test)]
mod timeout_tests {
    use crate::utils::timing::run_with_timeout;
    use std::future::pending;

    #[tokio::test]
    async fn run_with_timeout_returns_inner_result() {
        let result = run_with_timeout(50, "test", async { Ok::<_, anyhow::Error>(()) }).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn run_with_timeout_reports_timeout() {
        let result = run_with_timeout(1, "test", async {
            pending::<()>().await;
            Ok::<_, anyhow::Error>(())
        })
        .await;

        let err = result.expect_err("expected timeout");
        assert!(err.to_string().contains("test"));
    }
}

#[cfg(test)]
mod gap_tests {
    use super::{should_continue_feed_loop, TaskConfig};
    use crate::utils::twitter::twitteractivity_limits::EngagementLimits;
    use crate::utils::twitter::twitteractivity_state::SessionState;

    #[test]
    fn should_continue_feed_loop_stops_at_scroll_limit() {
        let session = SessionState::new(EngagementLimits::default(), 60_000, 100);
        let config = TaskConfig {
            scroll_count: 5,
            ..Default::default()
        };

        assert!(should_continue_feed_loop(&session, 4, &config));
        assert!(!should_continue_feed_loop(&session, 5, &config));
        assert!(!should_continue_feed_loop(&session, 6, &config));
    }

    #[test]
    fn should_continue_feed_loop_stops_when_expired() {
        let session = SessionState::new(EngagementLimits::default(), 0, 100);
        let config = TaskConfig {
            scroll_count: 100,
            ..Default::default()
        };

        // Brief yield to ensure time passes past 0ms deadline
        std::thread::sleep(std::time::Duration::from_millis(2));

        assert!(!should_continue_feed_loop(&session, 0, &config));
    }

    #[test]
    fn should_continue_feed_loop_zero_scroll_count() {
        let session = SessionState::new(EngagementLimits::default(), 60_000, 100);
        let config = TaskConfig {
            scroll_count: 0,
            ..Default::default()
        };

        // With scroll_count=0, should never continue
        assert!(!should_continue_feed_loop(&session, 0, &config));
    }
}
