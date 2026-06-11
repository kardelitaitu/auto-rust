//! Small utility / filter functions extracted from `twitteractivity_engagement.rs`.

use crate::prelude::TaskContext;
use crate::utils::twitter::{
    decision::EngagementLevel,
    twitteractivity_interact::is_on_tweet_page,
    twitteractivity_limits::{EngagementCounters, EngagementLimits},
    twitteractivity_persona::{
        should_bookmark, should_follow, should_like, should_quote, should_reply, should_retweet,
        PersonaWeights,
    },
    twitteractivity_state::{TaskConfig, TweetActionTracker},
    twitteractivity_types::TweetId,
};
use log::warn;

/// Calculate success rate as a percentage.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn calc_rate(success: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        (success as f64 / total as f64) * 100.0
    }
}

/// Check if action is allowed by limits (helper for `process_candidate`).
#[must_use]
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

/// Validate that we're on a tweet detail page before performing an action.
pub async fn validate_tweet_page(
    api: &TaskContext,
    did_dive: bool,
    action_name: &str,
    tweet_id: &str,
) -> bool {
    if !did_dive {
        warn!("Skipping {action_name}: not in thread detail view for tweet {tweet_id}");
        return false;
    }
    match is_on_tweet_page(api).await {
        Ok(true) => true,
        Ok(false) => {
            warn!("Skipping {action_name}: not on tweet page for tweet {tweet_id}");
            false
        }
        Err(e) => {
            warn!("Failed to validate tweet page context for {action_name}: {e}");
            false
        }
    }
}

pub fn selected_candidate_actions(
    candidate_persona: &PersonaWeights,
    tweet_id: &str,
    limits: &EngagementLimits,
    counters: &EngagementCounters,
    action_tracker: &TweetActionTracker,
) -> Vec<&'static str> {
    let mut actions_to_do = Vec::new();
    let tid = TweetId::from_unchecked(tweet_id);

    if should_like(candidate_persona)
        && action_tracker.can_perform_action(&tid)
        && limits.can_like(counters)
    {
        actions_to_do.push("like");
    }
    if should_retweet(candidate_persona)
        && action_tracker.can_perform_action(&tid)
        && limits.can_retweet(counters)
    {
        actions_to_do.push("retweet");
    }
    if should_quote(candidate_persona)
        && action_tracker.can_perform_action(&tid)
        && limits.can_quote_tweet(counters)
    {
        actions_to_do.push("quote");
    }
    if should_follow(candidate_persona)
        && action_tracker.can_perform_action(&tid)
        && limits.can_follow(counters)
    {
        actions_to_do.push("follow");
    }
    if should_reply(candidate_persona)
        && action_tracker.can_perform_action(&tid)
        && limits.can_reply(counters)
    {
        actions_to_do.push("reply");
    }
    if should_bookmark(candidate_persona)
        && action_tracker.can_perform_action(&tid)
        && limits.can_bookmark(counters)
    {
        actions_to_do.push("bookmark");
    }

    actions_to_do
}

pub fn filter_detail_actions_for_gate(
    actions_to_do: &mut Vec<&'static str>,
    has_status_url: bool,
    dive_allowed: bool,
) {
    if actions_to_do.iter().any(|&action| action != "like") && (!has_status_url || !dive_allowed) {
        actions_to_do.retain(|&action| action == "like");
    }
}

pub fn filter_actions_for_decision_level(
    actions_to_do: &mut Vec<&'static str>,
    level: EngagementLevel,
) {
    match level {
        EngagementLevel::Full => {}
        EngagementLevel::Medium => {
            actions_to_do.retain(|&action| matches!(action, "like" | "retweet"));
        }
        EngagementLevel::Minimal => {
            actions_to_do.retain(|&action| action == "like");
        }
        EngagementLevel::None => {
            actions_to_do.clear();
        }
    }
}

pub fn should_engage_replies_after_root_action(
    did_dive: bool,
    root_action_success: bool,
    task_config: &TaskConfig,
) -> bool {
    did_dive && root_action_success && !task_config.dry_run_actions
}

pub fn should_navigate_home_after_dive(did_dive: bool, task_config: &TaskConfig) -> bool {
    did_dive && !task_config.dry_run_actions
}
