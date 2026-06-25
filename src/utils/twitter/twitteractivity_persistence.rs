//! Inter-session persistence for Twitter activity automation.
//!
//! Stores session metadata to `~/.config/auto-rust/twitter-state.json` for
//! day-over-day behavior variance, time-of-day gating, and rate-limit backoff.
//!
//! ## Usage
//!
//! ```ignore
//! let mut state = TwitterPersistenceState::load();
//! state.record_action("like");
//! state.record_session_end();
//! state.save();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Persisted state for the twitteractivity task.
///
/// All times are Unix epoch milliseconds. Action counts are keyed by action name
/// (e.g., "like", "retweet", "follow", "reply", "bookmark", "quote").
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TwitterPersistenceState {
    /// Unix timestamp (ms) when the last session ended.
    pub last_session_end: Option<u64>,
    /// Action counts for the current calendar day, keyed by action name.
    pub daily_action_counts: HashMap<String, u32>,
    /// Unix timestamp (ms) of the last rate-limit event.
    pub last_rate_limit_timestamp: Option<u64>,
}

impl TwitterPersistenceState {
    /// Path to the state file: `~/.config/auto-rust/twitter-state.json`
    fn state_path() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home)
            .join(".config")
            .join("auto-rust")
            .join("twitter-state.json")
    }

    /// Load persisted state, or return defaults if the file doesn't exist or is corrupt.
    #[must_use]
    pub fn load() -> Self {
        let path = Self::state_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save current state to disk. Creates parent directories if needed.
    /// Silently ignores write errors.
    pub fn save(&self) {
        let path = Self::state_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, &json);
        }
    }

    /// Record a single action, incrementing the daily counter for this action type.
    pub fn record_action(&mut self, action: &str) {
        *self
            .daily_action_counts
            .entry(action.to_string())
            .or_insert(0) += 1;
    }

    /// Record the end of a session (updates `last_session_end` to now).
    pub fn record_session_end(&mut self) {
        self.last_session_end = Some(Self::now_ms());
    }

    /// Record a rate-limit event with the current timestamp.
    pub fn record_rate_limit(&mut self) {
        self.last_rate_limit_timestamp = Some(Self::now_ms());
    }

    /// Minutes since the last session ended. Returns `None` if no session has been recorded.
    #[must_use]
    pub fn minutes_since_last_session(&self) -> Option<u64> {
        let last = self.last_session_end?;
        let elapsed = Self::now_ms().saturating_sub(last);
        Some(elapsed / 60_000)
    }

    /// Current Unix time in milliseconds.
    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistence_default_is_empty() {
        let state = TwitterPersistenceState::default();
        assert!(state.last_session_end.is_none());
        assert!(state.daily_action_counts.is_empty());
        assert!(state.last_rate_limit_timestamp.is_none());
    }

    #[test]
    fn persistence_record_action_increments() {
        let mut state = TwitterPersistenceState::default();
        state.record_action("like");
        assert_eq!(state.daily_action_counts.get("like"), Some(&1));
        state.record_action("like");
        assert_eq!(state.daily_action_counts.get("like"), Some(&2));
    }

    #[test]
    fn persistence_record_session_end_sets_timestamp() {
        let mut state = TwitterPersistenceState::default();
        state.record_session_end();
        assert!(state.last_session_end.is_some());
    }

    #[test]
    fn persistence_record_rate_limit_sets_timestamp() {
        let mut state = TwitterPersistenceState::default();
        state.record_rate_limit();
        assert!(state.last_rate_limit_timestamp.is_some());
    }

    #[test]
    fn persistence_minutes_since_last_session_none_when_no_session() {
        let state = TwitterPersistenceState::default();
        assert!(state.minutes_since_last_session().is_none());
    }

    #[test]
    fn persistence_serialize_roundtrip() {
        let mut state = TwitterPersistenceState::default();
        state.record_action("like");
        state.record_action("retweet");
        state.record_session_end();

        let json = serde_json::to_string(&state).unwrap();
        let decoded: TwitterPersistenceState = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.daily_action_counts.get("like"), Some(&1));
        assert_eq!(decoded.daily_action_counts.get("retweet"), Some(&1));
        assert!(decoded.last_session_end.is_some());
    }

    #[test]
    fn persistence_load_from_nonexistent_file_returns_default() {
        // Should not panic when file doesn't exist
        let state = TwitterPersistenceState::load();
        // Default state is valid
        assert!(state.daily_action_counts.is_empty());
    }

    #[test]
    fn persistence_multiple_actions_track_independently() {
        let mut state = TwitterPersistenceState::default();
        state.record_action("like");
        state.record_action("retweet");
        state.record_action("like");
        state.record_action("follow");
        state.record_action("retweet");

        assert_eq!(state.daily_action_counts.get("like"), Some(&2));
        assert_eq!(state.daily_action_counts.get("retweet"), Some(&2));
        assert_eq!(state.daily_action_counts.get("follow"), Some(&1));
    }

    #[test]
    fn persistence_save_and_load_idempotent() {
        let mut state = TwitterPersistenceState::default();
        state.record_action("like");
        state.record_session_end();

        let json = serde_json::to_string_pretty(&state).unwrap();
        let decoded: TwitterPersistenceState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.daily_action_counts, decoded.daily_action_counts);
        assert_eq!(state.last_session_end, decoded.last_session_end);
    }
}
