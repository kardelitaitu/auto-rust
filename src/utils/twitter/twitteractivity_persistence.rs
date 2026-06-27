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
    /// Sanitize profile name to only allow alphanumeric characters, dashes, and underscores.
    /// Falls back to "default" if the name is empty.
    fn sanitize_profile_name(profile_name: &str) -> String {
        let trimmed = profile_name.trim();
        if trimmed.is_empty() {
            return "default".to_string();
        }
        let sanitized: String = trimmed
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if sanitized.is_empty() {
            "default".to_string()
        } else {
            sanitized
        }
    }

    /// Path to the state file: `~/.config/auto-rust/twitter-state-<profile_name>.json`
    fn state_path(profile_name: &str) -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        let sanitized = Self::sanitize_profile_name(profile_name);
        let filename = format!("twitter-state-{}.json", sanitized);
        PathBuf::from(home)
            .join(".config")
            .join("auto-rust")
            .join(filename)
    }

    /// Load persisted state, or return defaults if the file doesn't exist or is corrupt.
    #[must_use]
    pub fn load(profile_name: &str) -> Self {
        let path = Self::state_path(profile_name);
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save current state to disk atomically using a temp file + rename.
    /// Creates parent directories if needed. Silently ignores write errors.
    pub fn save(&self, profile_name: &str) {
        let path = Self::state_path(profile_name);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let temp_path = path.with_extension("tmp");
            if std::fs::write(&temp_path, &json).is_ok() {
                let _ = std::fs::rename(&temp_path, &path);
            }
        }
    }

    /// Path to the lock file: `~/.config/auto-rust/twitter-state-<profile_name>.json.lock`
    fn lock_path(profile_name: &str) -> PathBuf {
        Self::state_path(profile_name).with_extension("json.lock")
    }

    /// Try to acquire the lock. Auto-recovers if lock is stale (older than 15s).
    fn acquire_lock(profile_name: &str) -> bool {
        let lock_path = Self::lock_path(profile_name);
        if let Some(parent) = lock_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Auto-recover stale lock file (older than 15 seconds)
        if let Ok(metadata) = std::fs::metadata(&lock_path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    if elapsed.as_secs() > 15 {
                        log::warn!(
                            "[persistence-lock] Stale lock file found ({}s old) for profile '{}', removing to auto-recover",
                            elapsed.as_secs(),
                            profile_name
                        );
                        let _ = std::fs::remove_file(&lock_path);
                    }
                }
            }
        }

        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .is_ok()
    }

    /// Release the lock file.
    fn release_lock(profile_name: &str) {
        let _ = std::fs::remove_file(Self::lock_path(profile_name));
    }

    /// Update the state atomically. Blocks until the lock can be acquired or times out.
    pub async fn update_async<F>(profile_name: &str, f: F) -> Result<(), anyhow::Error>
    where
        F: FnOnce(&mut Self),
    {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(5);

        while !Self::acquire_lock(profile_name) {
            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!(
                    "Timeout waiting for twitter-state-{}.json lock",
                    Self::sanitize_profile_name(profile_name)
                ));
            }
            // Sleep briefly and retry (randomize to prevent thundering herd)
            let sleep_ms = 50 + rand::random::<u64>() % 100;
            tokio::time::sleep(std::time::Duration::from_millis(sleep_ms)).await;
        }

        // LockGuard ensures that the lock is released even if F panics or we return early.
        struct LockGuard {
            profile: String,
        }
        impl Drop for LockGuard {
            fn drop(&mut self) {
                TwitterPersistenceState::release_lock(&self.profile);
            }
        }

        let _guard = LockGuard {
            profile: profile_name.to_string(),
        };

        let mut state = Self::load(profile_name);
        f(&mut state);
        state.save(profile_name);

        Ok(())
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
        let state = TwitterPersistenceState::load("nonexistent_test_profile");
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

    #[tokio::test]
    async fn persistence_save_and_load_file_system() {
        let profile = format!("test_profile_{}", rand::random::<u32>());
        let mut state = TwitterPersistenceState::default();
        state.record_action("like");
        state.record_session_end();
        state.save(&profile);

        let loaded = TwitterPersistenceState::load(&profile);
        assert_eq!(loaded.daily_action_counts.get("like"), Some(&1));
        assert!(loaded.last_session_end.is_some());

        // Clean up
        let path = TwitterPersistenceState::state_path(&profile);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn persistence_update_async_concurrency() {
        let profile = format!("test_profile_{}", rand::random::<u32>());
        let p_clone = profile.clone();

        let handle = tokio::spawn(async move {
            let res = TwitterPersistenceState::update_async(&p_clone, |s| {
                s.record_action("like");
            })
            .await;
            assert!(res.is_ok());
        });

        let res = TwitterPersistenceState::update_async(&profile, |s| {
            s.record_action("retweet");
        })
        .await;
        assert!(res.is_ok());

        handle.await.unwrap();

        let loaded = TwitterPersistenceState::load(&profile);
        assert_eq!(loaded.daily_action_counts.get("like"), Some(&1));
        assert_eq!(loaded.daily_action_counts.get("retweet"), Some(&1));

        // Clean up
        let path = TwitterPersistenceState::state_path(&profile);
        let _ = std::fs::remove_file(path);
    }
}
