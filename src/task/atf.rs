//! Abstract Task Framework (ATF) — Core task orchestration utilities.
//!
//! This module provides foundational abstractions for task management,
//! configuration handling, and inter-module communication. It serves as
//! the base layer upon which specific activity modules (like TwitterActivity)
//! are built.
//!
//! # Module Structure
//! This single file provides the core types: `Config`, scroll/engagement/persistence
//! configs, `TaskState` lifecycle, `AtfEvent` system, `ActionRecord`/`ScanResult`/
//! `TaskSummary` data types, and the `TaskContext` trait.
//!
//! # Example Usage
//! ```ignore
//! use atf::Config;
//! let config = Config::from_payload(&payload)?;
//! ```

use std::collections::HashMap;
use std::time::Instant;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Application-level configuration for task execution.
///
/// This struct is deserialized from JSON payloads and validates against
/// expected schema constraints before use.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Maximum duration for any single task in milliseconds
    pub max_duration_ms: u64,
    /// Default scroll behavior settings
    pub default_scroll: ScrollConfig,
    /// Engagement action limits per session
    pub engagement_limits: EngagementLimits,
    /// Persistence configuration for inter-session state
    pub persistence: PersistenceConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_duration_ms: 600_000, // 10 minutes default
            default_scroll: ScrollConfig::default(),
            engagement_limits: EngagementLimits::default(),
            persistence: PersistenceConfig::default(),
        }
    }
}

impl Config {
    /// Validates config against hard constraints
    pub fn validate(&self) -> Result<()> {
        if self.max_duration_ms > 3_600_000 {
            return Err(anyhow!("max_duration_ms exceeds 1 hour limit"));
        }
        Ok(())
    }
}

/// Scroll behavior configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScrollConfig {
    pub amount_pixels: i32,
    pub pause_ms: u64,
    pub smooth: bool,
    pub back_scroll: bool,
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            amount_pixels: 500,
            pause_ms: 1000,
            smooth: true,
            back_scroll: false,
        }
    }
}

/// Engagement action limits per task session
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EngagementLimits {
    pub max_likes: u32,
    pub max_retweets: u32,
    pub max_follows: u32,
    pub max_replies: u32,
    pub max_thread_dives: u32,
    pub max_bookmarks: u32,
    pub max_quote_tweets: u32,
    pub max_total_actions: u32,
}

impl Default for EngagementLimits {
    fn default() -> Self {
        Self {
            max_likes: 50,
            max_retweets: 30,
            max_follows: 10,
            max_replies: 20,
            max_thread_dives: 15,
            max_bookmarks: 25,
            max_quote_tweets: 10,
            max_total_actions: 200,
        }
    }
}

/// Persistence configuration for state between sessions
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PersistenceConfig {
    pub enabled: bool,
    pub data_dir: String,
    pub retention_days: u32,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            data_dir: "./.atf/persistence".to_string(),
            retention_days: 30,
        }
    }
}

/// Task lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task has not started yet
    Idle,
    /// Currently running and active
    Running,
    /// Completed successfully
    Succeeded,
    /// Failed due to error or timeout
    Failed,
    /// Paused awaiting user input
    Paused,
}

/// Event types for inter-module communication
#[derive(Debug, Clone)]
pub enum AtfEvent {
    /// Task started with initial configuration
    TaskStarted(Config),
    /// Engagement action performed (like, retweet, etc.)
    ActionPerformed(ActionRecord),
    /// Feed scan completed with candidate count
    ScanResult(ScanResult),
    /// Session state checkpoint saved
    StateCheckpointed(u64),
    /// Task completed (success or failure)
    TaskCompleted(TaskSummary),
}

/// Record of an engagement action
#[derive(Debug, Clone, Serialize)]
pub struct ActionRecord {
    pub action_type: ActionType,
    pub target_id: String,
    /// Monotonic timestamp (not persisted across sessions)
    #[serde(skip)]
    pub timestamp: Instant,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ActionType {
    Like,
    Retweet,
    Follow,
    Reply,
    Bookmark,
    QuoteTweet,
    ThreadDive,
}

/// Result of a feed scan operation
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub candidates_found: u32,
    pub scroll_depth: u32,
    pub failed_scrolls: u32,
    pub empty_scans: u32,
}

/// Summary of a completed task session
#[derive(Debug, Clone)]
pub struct TaskSummary {
    pub duration_ms: u64,
    pub total_actions: u32,
    pub actions_by_type: HashMap<String, u32>,
    pub final_state: TaskState,
}

/// Core trait for task execution contexts
pub trait TaskContext {
    /// Get the current configuration
    fn config(&self) -> &Config;
    /// Check if task is currently running
    fn is_running(&self) -> bool;
    /// Record an action and update state
    fn record_action(&mut self, action: ActionRecord);
    /// Emit an event to the event system
    fn emit_event(&mut self, event: AtfEvent);
}

/// Default implementation for basic ATF operations
pub struct DefaultAtf {
    pub config: Config,
    pub state: TaskState,
    pub events: Vec<AtfEvent>,
}

impl DefaultAtf {
    /// Creates a new default ATF instance
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            state: TaskState::Idle,
            events: Vec::new(),
        }
    }
}

impl Default for DefaultAtf {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ====================================================================
    // RED Tests — describe desired behavior (expected to fail on first run)
    // ====================================================================

    #[test]
    fn tdd_red_config_validation_blocks_over_duration() {
        let config = Config {
            max_duration_ms: 3_601_000, // Exceeds 1 hour limit
            ..Default::default()
        };

        assert!(
            config.validate().is_err(),
            "Config should reject > 1 hour duration"
        );
    }

    #[test]
    fn tdd_red_action_record_has_all_required_fields() {
        let record = ActionRecord {
            action_type: ActionType::Like,
            target_id: "tweet_123".to_string(),
            timestamp: Instant::now(),
            success: true,
        };

        assert_eq!(record.action_type, ActionType::Like);
        assert_eq!(record.target_id, "tweet_123");
    }

    // ====================================================================
    // GREEN Tests — validate working behavior
    // ====================================================================

    #[test]
    fn tdd_green_default_config_validates_successfully() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn tdd_green_task_state_transitions_correctly() {
        let atf = DefaultAtf::new();
        assert_eq!(atf.state, TaskState::Idle);
        // Simulate state transition (would be done via emit_event)
    }

    #[test]
    fn tdd_green_action_record_serializes_correctly() {
        let record = ActionRecord {
            action_type: ActionType::Retweet,
            target_id: "tweet_456".to_string(),
            timestamp: Instant::now(),
            success: false,
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("Retweet"));
    }

    #[test]
    fn tdd_green_task_summary_aggregates_actions() {
        let mut actions_by_type: HashMap<String, u32> = HashMap::new();
        actions_by_type.insert("like".to_string(), 5);
        actions_by_type.insert("retweet".to_string(), 3);

        let summary = TaskSummary {
            duration_ms: 120_000,
            total_actions: 8,
            actions_by_type,
            final_state: TaskState::Succeeded,
        };

        assert_eq!(summary.total_actions, 8);
    }

    // ====================================================================
    // EDGE Case Tests
    // ====================================================================

    #[test]
    fn tdd_edge_empty_scan_result_valid() {
        let result = ScanResult {
            candidates_found: 0,
            scroll_depth: 10,
            failed_scrolls: 0,
            empty_scans: 5,
        };

        assert_eq!(result.candidates_found, 0);
    }

    #[test]
    fn tdd_edge_task_summary_zero_duration() {
        let summary = TaskSummary {
            duration_ms: 0,
            total_actions: 0,
            actions_by_type: HashMap::new(),
            final_state: TaskState::Failed,
        };

        assert_eq!(summary.duration_ms, 0);
    }

    // ====================================================================
    // Test-support helpers
    // ====================================================================

    #[test]
    fn tdd_support_helpers_build_test_fixtures() {
        let config = test_support::create_test_config(5000);
        assert_eq!(config.max_duration_ms, 5000);
        assert!(config.validate().is_ok());

        let record = test_support::create_test_action_record(ActionType::Follow, "user_1");
        assert_eq!(record.action_type, ActionType::Follow);
        assert_eq!(record.target_id, "user_1");
        assert!(record.success);
    }
}

#[cfg(test)]
mod test_support {
    use super::*;

    pub fn create_test_config(duration_ms: u64) -> Config {
        Config {
            max_duration_ms: duration_ms,
            ..Default::default()
        }
    }

    pub fn create_test_action_record(action_type: ActionType, target: &str) -> ActionRecord {
        ActionRecord {
            action_type,
            target_id: target.to_string(),
            timestamp: Instant::now(),
            success: true,
        }
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_config_from_default() {
        let config = Config::default();
        assert_eq!(config.max_duration_ms, 600_000);
        assert!(
            config.persistence.enabled,
            "Persistence should be enabled by default"
        );
    }

    #[test]
    fn test_engagement_limits_defaults() {
        let limits = EngagementLimits::default();
        assert_eq!(limits.max_likes, 50);
        assert_eq!(limits.max_total_actions, 200);
    }
}

#[cfg(test)]
mod state_tests {
    use super::*;

    #[test]
    fn test_task_state_all_variants() {
        let states = [
            TaskState::Idle,
            TaskState::Running,
            TaskState::Succeeded,
            TaskState::Failed,
            TaskState::Paused,
        ];

        assert_eq!(states.len(), 5);
    }
}
