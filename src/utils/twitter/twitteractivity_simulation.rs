//! Pure simulation engine for Twitter activity.
//!
//! This module never touches the browser. It only uses the payload,
//! configuration, and a seeded RNG to produce a deterministic log plan.

use crate::config::Config;
use crate::utils::twitter::{
    twitteractivity_constants::MIN_CANDIDATE_SCAN_INTERVAL_MS,
    twitteractivity_limits::{EngagementCounters, EngagementLimits},
    twitteractivity_persona::PersonaWeights,
    twitteractivity_state::TaskConfig,
};
use anyhow::Result;
use log::info;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::fmt;

const SIMULATION_PROFILE_NAME: &str = "Average";
const SIMULATION_PERSONA_DEFAULT: &str = "config_default";
const SIMULATION_PERSONA_CUSTOM: &str = "payload_custom";
const EMPTY_SCAN_LIMIT: u32 = 3;

#[derive(Debug, Clone)]
pub struct SimulationReport {
    pub lines: Vec<String>,
    pub stop_reason: SimulationStopReason,
    pub total_actions: u32,
    pub scans: u32,
    pub remaining_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulationStopReason {
    DurationExhausted,
    LimitReached,
    CandidateBudgetExhausted,
    NoMorePlannedActions,
    SimulatedError(String),
}

impl SimulationStopReason {
    pub fn as_str(&self) -> &str {
        match self {
            Self::DurationExhausted => "duration_exhausted",
            Self::LimitReached => "limit_reached",
            Self::CandidateBudgetExhausted => "candidate_budget_exhausted",
            Self::NoMorePlannedActions => "no_more_planned_actions",
            Self::SimulatedError(_) => "simulated_error",
        }
    }
}

impl fmt::Display for SimulationStopReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SimulatedError(message) => write!(f, "{}: {}", self.as_str(), message),
            _ => f.write_str(self.as_str()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum SimAction {
    Like,
    Retweet,
    Quote,
    Follow,
    Reply,
    Bookmark,
    Dive,
}

impl SimAction {
    const ALL: [Self; 7] = [
        Self::Like,
        Self::Retweet,
        Self::Quote,
        Self::Follow,
        Self::Reply,
        Self::Bookmark,
        Self::Dive,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Like => "like",
            Self::Retweet => "retweet",
            Self::Quote => "quote",
            Self::Follow => "follow",
            Self::Reply => "reply",
            Self::Bookmark => "bookmark",
            Self::Dive => "dive",
        }
    }

    fn probability(self, persona: &PersonaWeights) -> f64 {
        match self {
            Self::Like => persona.like_prob,
            Self::Retweet => persona.retweet_prob,
            Self::Quote => persona.quote_prob,
            Self::Follow => persona.follow_prob,
            Self::Reply => persona.reply_prob,
            Self::Bookmark => persona.bookmark_prob,
            Self::Dive => persona.thread_dive_prob,
        }
    }

    fn used_and_limit(
        self,
        counters: &EngagementCounters,
        limits: &EngagementLimits,
    ) -> (u32, u32) {
        match self {
            Self::Like => (counters.likes, limits.max_likes),
            Self::Retweet => (counters.retweets, limits.max_retweets),
            Self::Quote => (counters.quote_tweets, limits.max_quote_tweets),
            Self::Follow => (counters.follows, limits.max_follows),
            Self::Reply => (counters.replies, limits.max_replies),
            Self::Bookmark => (counters.bookmarks, limits.max_bookmarks),
            Self::Dive => (counters.thread_dives, limits.max_thread_dives),
        }
    }

    fn is_allowed(self, counters: &EngagementCounters, limits: &EngagementLimits) -> bool {
        let (used, limit) = self.used_and_limit(counters, limits);
        used < limit && counters.total_actions() < limits.max_total_actions
    }

    fn increment(self, counters: &mut EngagementCounters) {
        match self {
            Self::Like => counters.increment_like(),
            Self::Retweet => counters.increment_retweet(),
            Self::Quote => counters.increment_quote_tweet(),
            Self::Follow => counters.increment_follow(),
            Self::Reply => counters.increment_reply(),
            Self::Bookmark => counters.increment_bookmark(),
            Self::Dive => counters.increment_thread_dive(),
        }
    }
}

pub fn run_simulation(task_config: &TaskConfig, config: &Config) -> Result<()> {
    let report = simulate(task_config, config);
    for line in report.lines {
        info!("{}", line);
    }
    Ok(())
}

pub fn simulate(task_config: &TaskConfig, config: &Config) -> SimulationReport {
    let persona = build_persona_weights(task_config, config);
    let persona_label = if task_config.weights.is_some() {
        SIMULATION_PERSONA_CUSTOM
    } else {
        SIMULATION_PERSONA_DEFAULT
    };

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

    let scan_interval_ms = config
        .twitter_activity
        .candidate_scan_interval_ms
        .max(MIN_CANDIDATE_SCAN_INTERVAL_MS);
    let mut rng = StdRng::seed_from_u64(task_config.seed);
    let mut counters = EngagementCounters::new();
    let mut lines = Vec::new();
    let mut scan_index = 0u32;
    let mut elapsed_ms = 0u64;
    let mut empty_scan_streak = 0u32;
    let mut stop_reason = SimulationStopReason::DurationExhausted;

    lines.push(format!(
        "simulation | seed={} simulate_only=true duration_ms={} persona={} profile={}",
        task_config.seed, task_config.duration_ms, persona_label, SIMULATION_PROFILE_NAME
    ));
    lines.push("simulation | phase=navigation entry_point=home action=simulate".to_string());

    while elapsed_ms < task_config.duration_ms {
        scan_index += 1;

        let candidate_budget = task_config.candidate_count;
        let candidates_found = rng.gen_range(1..=candidate_budget);
        lines.push(format!(
            "simulation | phase=scan scan_index={} candidate_budget={} candidates_found={}",
            scan_index, candidate_budget, candidates_found
        ));

        let mut actions_this_scan = 0u32;
        let mut scan_had_hit = false;

        'candidate_loop: for candidate_index in 0..candidates_found {
            for action in SimAction::ALL {
                if actions_this_scan >= task_config.max_actions_per_scan {
                    break 'candidate_loop;
                }

                let (used, limit) = action.used_and_limit(&counters, &limits);
                let allowed = action.is_allowed(&counters, &limits);
                lines.push(format!(
                    "simulation | budget action={} used={} limit={} result={}",
                    action.name(),
                    used,
                    limit,
                    if allowed { "allow" } else { "block" }
                ));

                if !allowed {
                    if counters.total_actions() >= limits.max_total_actions {
                        stop_reason = SimulationStopReason::LimitReached;
                        break 'candidate_loop;
                    }
                    continue;
                }

                let p = action.probability(&persona).clamp(0.0, 1.0);
                let r = rng.gen_range(0.0..1.0);
                let hit = r < p;
                lines.push(format!(
                    "simulation | roll candidate_index={} action={} p={:.2} r={:.2} result={}",
                    candidate_index,
                    action.name(),
                    p,
                    r,
                    if hit { "hit" } else { "miss" }
                ));

                if hit {
                    action.increment(&mut counters);
                    actions_this_scan += 1;
                    scan_had_hit = true;

                    if counters.total_actions() >= limits.max_total_actions {
                        stop_reason = SimulationStopReason::LimitReached;
                        break 'candidate_loop;
                    }
                }
            }
        }

        elapsed_ms = elapsed_ms.saturating_add(scan_interval_ms);

        if matches!(stop_reason, SimulationStopReason::LimitReached) {
            break;
        }

        if scan_had_hit {
            empty_scan_streak = 0;
        } else {
            empty_scan_streak += 1;
            if empty_scan_streak >= EMPTY_SCAN_LIMIT {
                stop_reason = SimulationStopReason::NoMorePlannedActions;
                break;
            }
        }
    }

    if matches!(stop_reason, SimulationStopReason::DurationExhausted)
        && elapsed_ms >= task_config.duration_ms
    {
        stop_reason = SimulationStopReason::DurationExhausted;
    }

    let remaining_ms = task_config.duration_ms.saturating_sub(elapsed_ms);
    lines.push(format!(
        "simulation | stop_reason={} total_actions={} scans={} remaining_ms={}",
        stop_reason.as_str(),
        counters.total_actions(),
        scan_index,
        remaining_ms
    ));

    SimulationReport {
        lines,
        stop_reason,
        total_actions: counters.total_actions(),
        scans: scan_index,
        remaining_ms,
    }
}

fn build_persona_weights(task_config: &TaskConfig, config: &Config) -> PersonaWeights {
    crate::utils::twitter::twitteractivity_persona::select_persona_weights(
        task_config.weights.as_ref(),
        &config.twitter_activity.probabilities,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, EngagementLimitsConfig};

    fn simulation_config() -> Config {
        Config {
            twitter_activity: crate::config::TwitterActivityConfig {
                candidate_scan_interval_ms: 2500,
                engagement_candidate_count: 3,
                engagement_limits: EngagementLimitsConfig {
                    max_likes: 2,
                    max_retweets: 2,
                    max_follows: 1,
                    max_replies: 1,
                    max_thread_dives: 1,
                    max_bookmarks: 1,
                    max_quote_tweets: 1,
                    max_total_actions: 4,
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn simulation_task(seed: u64) -> TaskConfig {
        TaskConfig {
            duration_ms: 10_000,
            candidate_count: 3,
            max_actions_per_scan: 2,
            simulate_only: true,
            seed,
            ..Default::default()
        }
    }

    #[test]
    fn simulation_is_deterministic_for_same_seed() {
        let config = simulation_config();
        let first = simulate(&simulation_task(42), &config);
        let second = simulate(&simulation_task(42), &config);

        assert_eq!(first.lines, second.lines);
        assert_eq!(first.stop_reason, second.stop_reason);
    }

    #[test]
    fn simulation_changes_with_seed() {
        let config = simulation_config();
        let first = simulate(&simulation_task(42), &config);
        let second = simulate(&simulation_task(43), &config);

        assert_ne!(first.lines, second.lines);
    }

    #[test]
    fn simulation_emits_required_schema_lines() {
        let config = simulation_config();
        let report = simulate(&simulation_task(7), &config);

        assert!(report
            .lines
            .iter()
            .any(|line| line.starts_with("simulation | seed=7")));
        assert!(report
            .lines
            .iter()
            .any(|line| line.starts_with("simulation | phase=navigation")));
        assert!(report
            .lines
            .iter()
            .any(|line| line.starts_with("simulation | phase=scan")));
        assert!(report
            .lines
            .iter()
            .any(|line| line.starts_with("simulation | stop_reason=")));
    }
}
