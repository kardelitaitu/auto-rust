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
use std::sync::OnceLock;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

static PERSISTENCE_SENDER: OnceLock<mpsc::Sender<PersistenceCommand>> = OnceLock::new();

/// Command sent to the background persistence writer task.
pub enum PersistenceCommand {
    Update {
        profile_name: String,
        update_fn: Box<dyn FnOnce(&mut TwitterPersistenceState) + Send + 'static>,
        reply_tx: oneshot::Sender<Result<(), String>>,
    },
}

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

    fn get_or_init_writer() -> &'static mpsc::Sender<PersistenceCommand> {
        PERSISTENCE_SENDER.get_or_init(|| {
            let (tx, rx) = mpsc::channel(100);
            tokio::spawn(Self::writer_loop(rx));
            tx
        })
    }

    async fn writer_loop(mut rx: mpsc::Receiver<PersistenceCommand>) {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                PersistenceCommand::Update {
                    profile_name,
                    update_fn,
                    reply_tx,
                } => {
                    let res = tokio::task::spawn_blocking(move || {
                        let mut state = Self::load(&profile_name);
                        update_fn(&mut state);
                        state.save(&profile_name);
                    })
                    .await;

                    let response = match res {
                        Ok(()) => Ok(()),
                        Err(e) => Err(format!("Task execution panicked: {e:?}")),
                    };

                    let _ = reply_tx.send(response);
                }
            }
        }
    }

    /// Update the state atomically. Offloads file system updates to the background
    /// queue to prevent file contention and Tokio scheduler blocking.
    pub async fn update_async<F>(profile_name: &str, f: F) -> Result<(), anyhow::Error>
    where
        F: FnOnce(&mut Self) + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();
        let cmd = PersistenceCommand::Update {
            profile_name: profile_name.to_string(),
            update_fn: Box::new(f),
            reply_tx: tx,
        };

        let sender = Self::get_or_init_writer();
        if sender.send(cmd).await.is_err() {
            return Err(anyhow::anyhow!(
                "Background persistence writer task has shut down"
            ));
        }

        rx.await
            .map_err(|e| anyhow::anyhow!("Persistence response channel dropped: {e}"))?
            .map_err(|e| anyhow::anyhow!("{e}"))
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
