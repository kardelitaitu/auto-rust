//! Click learning engine for adaptive automation.
//!
//! Provides a service-based API for click learning persistence with:
//! - TTL-based data expiration
//! - Privacy controls (enable/disable)
//! - Automatic cleanup of stale data
//! - Decoupled from TaskContext for better testability

use crate::runtime::task_context::click_learning::{
    ClickAdaptation, ClickLearningState, ClickTimingContext, SelectorLearningStats,
};
use crate::utils::profile::BrowserProfile;
use anyhow::Result;
use chrono::{Duration, Utc};
use std::fs;
use std::path::{Path, PathBuf};

/// LearningEngine manages click learning persistence and adaptation.
///
/// This service decouples click learning from TaskContext and provides
/// a clean API for recording, retrieving, and managing learning data.
pub struct LearningEngine {
    state: ClickLearningState,
    path: Option<PathBuf>,
    enabled: bool,
    ttl_days: u32,
}

impl LearningEngine {
    /// Create a new LearningEngine for a session.
    ///
    /// # Arguments
    /// * `session_id` - Unique session identifier
    /// * `behavior_profile` - Browser profile for path determination
    /// * `enabled` - Whether learning is enabled
    /// * `ttl_days` - Days before data expires (0 = never)
    pub fn new(
        session_id: &str,
        behavior_profile: &BrowserProfile,
        enabled: bool,
        ttl_days: u32,
    ) -> Self {
        let path = learning_data_path(session_id, behavior_profile);
        let state = path
            .as_ref()
            .and_then(|p| load_learning_state(p))
            .unwrap_or_default();

        let mut engine = Self {
            state,
            path,
            enabled,
            ttl_days,
        };

        // Prune expired entries on load
        if ttl_days > 0 {
            let _ = engine.prune_expired();
        }

        engine
    }

    /// Create a disabled engine (no-op, no persistence).
    pub fn disabled() -> Self {
        Self {
            state: ClickLearningState::default(),
            path: None,
            enabled: false,
            ttl_days: 0,
        }
    }

    /// Record a click result.
    ///
    /// If disabled, this is a no-op that returns Ok.
    pub fn record(&mut self, selector: &str, success: bool) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        self.state.record(selector, success);
        self.save()?;
        Ok(())
    }

    /// Get adaptation for a selector based on current state.
    pub fn adaptation_for(&self, selector: &str, context: &ClickTimingContext) -> ClickAdaptation {
        self.state.adaptation_for(selector, context)
    }

    /// Get statistics for a specific selector.
    pub fn selector_stats(&self, selector: &str) -> SelectorLearningStats {
        self.state.selector_stats(selector)
    }

    /// Clear all learning data for this session.
    pub fn clear(&mut self) -> Result<()> {
        self.state = ClickLearningState::default();
        if let Some(ref path) = self.path {
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    /// Prune expired selector entries.
    ///
    /// Returns the number of entries pruned.
    pub fn prune_expired(&mut self) -> Result<usize> {
        if self.ttl_days == 0 || !self.enabled {
            return Ok(0);
        }

        let cutoff = Utc::now() - Duration::days(self.ttl_days as i64);
        let before = self.state.selectors.len();

        self.state.selectors.retain(|_, stats| {
            stats.last_updated.map(|dt| dt > cutoff).unwrap_or(true) // Keep if no timestamp (backward compat)
        });

        let pruned = before - self.state.selectors.len();
        if pruned > 0 {
            self.save()?;
        }
        Ok(pruned)
    }

    /// Save current state to disk.
    pub fn save(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        if let Some(ref path) = self.path {
            save_learning_state(path, &self.state)?;
        }
        Ok(())
    }

    /// Get recent success rate (last 32 interactions).
    pub fn recent_success_rate(&self) -> f64 {
        self.state.recent_success_rate()
    }

    /// Get total interaction count.
    pub fn interaction_count(&self) -> u64 {
        self.state.interaction_count
    }

    /// Check if learning is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get TTL in days.
    pub fn ttl_days(&self) -> u32 {
        self.ttl_days
    }

    /// Clear all learning data across all sessions.
    ///
    /// This removes the entire click-learning directory.
    pub fn clear_all() -> Result<()> {
        let base_dir = std::env::current_dir()?.join("click-learning");
        if base_dir.exists() {
            fs::remove_dir_all(&base_dir)?;
        }
        Ok(())
    }
}

/// Get the file path for learning data.
fn learning_data_path(session_id: &str, behavior_profile: &BrowserProfile) -> Option<PathBuf> {
    let base_dir = std::env::current_dir().ok()?.join("click-learning");
    let profile_component = sanitize_path_component(&behavior_profile.name);
    let session_component = sanitize_path_component(session_id);
    Some(
        base_dir
            .join(profile_component)
            .join(format!("{session_component}.json")),
    )
}

/// Sanitize a path component for file storage.
fn sanitize_path_component(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "default".to_string()
    } else {
        trimmed
    }
}

/// Load learning state from file.
fn load_learning_state(path: &Path) -> Option<ClickLearningState> {
    let content = fs::read_to_string(path).ok()?;
    let mut state: ClickLearningState = serde_json::from_str(&content).ok()?;

    // Backward compatibility: ensure last_updated is set
    let now = Utc::now();
    for stats in state.selectors.values_mut() {
        if stats.last_updated.is_none() {
            stats.last_updated = Some(now);
        }
    }

    Some(state)
}

/// Save learning state to file.
fn save_learning_state(path: &Path, state: &ClickLearningState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state)?;
    fs::write(path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::task_context::click_learning::{
        ClickElementPriority, ClickFatigueLevel, ClickPageContext, ClickTimingContext,
    };
    use crate::utils::profile::BrowserProfile;

    fn create_test_profile() -> BrowserProfile {
        // Only use the name field which is common between utils::profile::BrowserProfile
        // and config::BrowserProfile. The learning engine only needs the name for path.
        let mut profile = BrowserProfile::average();
        profile.name = "test-profile".to_string();
        profile
    }

    fn create_test_context() -> ClickTimingContext {
        ClickTimingContext {
            page: ClickPageContext::Social,
            priority: ClickElementPriority::Normal,
            fatigue: ClickFatigueLevel::Normal,
            recent_success_rate: 1.0,
        }
    }

    #[test]
    fn test_learning_engine_disabled() {
        let _profile = create_test_profile();
        let mut engine = LearningEngine::disabled();

        // Recording should be no-op
        assert!(engine.record("#button", true).is_ok());
        assert!(engine.record("#button", false).is_ok());

        // Stats should be default
        let stats = engine.selector_stats("#button");
        assert_eq!(stats.attempts, 0);
        assert!(!engine.is_enabled());
    }

    #[test]
    fn test_learning_convergence() {
        let profile = create_test_profile();
        let mut engine = LearningEngine::new("test-session", &profile, true, 30);

        // Record 3 failures for the same selector
        for _ in 0..3 {
            engine
                .record("button[data-testid='retweet']", false)
                .unwrap();
        }

        let context = create_test_context();
        let adaptation = engine.adaptation_for("button[data-testid='retweet']", &context);

        // After 3 failures, adaptation should increase
        assert!(adaptation.reaction_delay_multiplier > 1.0);
        assert!(adaptation.require_strict_verification);
        assert!(adaptation.prefer_coordinate_fallback);
    }

    #[test]
    fn test_learning_success_improves_adaptation() {
        let profile = create_test_profile();
        let mut engine = LearningEngine::new("test-session", &profile, true, 30);

        // Record 5 successes with simple selector
        for _ in 0..5 {
            engine.record("#like", true).unwrap();
        }

        let context = create_test_context();
        let adaptation = engine.adaptation_for("#like", &context);

        // With all successes and simple selector, should use defaults
        assert!(adaptation.reaction_delay_multiplier >= 1.0);
        assert!(!adaptation.require_strict_verification);
    }

    #[test]
    fn test_ttl_pruning() {
        let profile = create_test_profile();
        let mut engine = LearningEngine::new("test-session", &profile, true, 7);

        // Manually insert stats with old timestamps
        let old_date = Utc::now() - Duration::days(10);
        let recent_date = Utc::now() - Duration::days(5);

        engine.state.selectors.insert(
            "old-selector".to_string(),
            SelectorLearningStats {
                attempts: 5,
                successes: 3,
                consecutive_failures: 0,
                last_updated: Some(old_date),
            },
        );

        engine.state.selectors.insert(
            "recent-selector".to_string(),
            SelectorLearningStats {
                attempts: 5,
                successes: 5,
                consecutive_failures: 0,
                last_updated: Some(recent_date),
            },
        );

        // Prune should remove old-selector
        let pruned = engine.prune_expired().unwrap();
        assert_eq!(pruned, 1);
        assert!(!engine.state.selectors.contains_key("old-selector"));
        assert!(engine.state.selectors.contains_key("recent-selector"));
    }

    #[test]
    fn test_ttl_zero_never_expires() {
        let profile = create_test_profile();
        let mut engine = LearningEngine::new("test-session", &profile, true, 0); // 0 = never

        // Insert old stats
        let old_date = Utc::now() - Duration::days(365);
        engine.state.selectors.insert(
            "very-old".to_string(),
            SelectorLearningStats {
                attempts: 5,
                successes: 3,
                consecutive_failures: 0,
                last_updated: Some(old_date),
            },
        );

        // Should not prune anything
        let pruned = engine.prune_expired().unwrap();
        assert_eq!(pruned, 0);
        assert!(engine.state.selectors.contains_key("very-old"));
    }

    #[test]
    #[ignore = "filesystem test - run with --ignored flag"]
    fn test_clear_session_data() {
        let temp_dir =
            std::env::temp_dir().join(format!("clear-session-test-{}", std::process::id()));
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).ok();

        let profile = create_test_profile();
        let mut engine = LearningEngine::new("clear-test-session", &profile, true, 30);

        engine.record("#button", true).unwrap();
        assert_eq!(engine.interaction_count(), 1);

        engine.clear().unwrap();
        assert_eq!(engine.interaction_count(), 0);
        assert_eq!(engine.selector_stats("#button").attempts, 0);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::env::set_current_dir(original_dir).ok();
    }

    #[test]
    #[ignore = "filesystem test - run with --ignored flag"]
    fn test_clear_all() {
        // Create temp directory for test
        let temp_dir =
            std::env::temp_dir().join(format!("click-learning-test-{}", std::process::id()));

        // Set current dir to temp for this test
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).ok();

        // Create some learning files
        let profile = create_test_profile();
        let mut engine1 = LearningEngine::new("session-1", &profile, true, 30);
        engine1.record("#btn", true).unwrap();
        engine1.save().unwrap();

        let mut engine2 = LearningEngine::new("session-2", &profile, true, 30);
        engine2.record("#btn", true).unwrap();
        engine2.save().unwrap();

        // Verify directory exists
        let base_dir = temp_dir.join("click-learning");
        assert!(base_dir.exists(), "base_dir should exist after save");

        // Clear all
        LearningEngine::clear_all().unwrap();

        // Verify directory is gone
        assert!(
            !base_dir.exists(),
            "base_dir should not exist after clear_all"
        );

        // Restore original dir
        std::env::set_current_dir(original_dir).ok();
    }

    #[test]
    fn test_decay_algorithm() {
        let profile = create_test_profile();
        let mut engine = LearningEngine::new("test-session", &profile, true, 30);

        // Mix of successes and failures
        for i in 0..20 {
            engine.record("#dynamic", i % 3 != 0).unwrap(); // 66% success rate
        }

        let context = ClickTimingContext {
            recent_success_rate: engine.recent_success_rate(),
            ..create_test_context()
        };

        let adaptation = engine.adaptation_for("#dynamic", &context);

        // With mixed results, should have moderate adaptation
        assert!(adaptation.reaction_delay_multiplier >= 1.0);
    }

    #[test]
    #[ignore = "filesystem test - run with --ignored flag"]
    fn test_persistence_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("learning-test-{}", std::process::id()));
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).ok();

        let profile = create_test_profile();

        // Create and populate engine
        {
            let mut engine = LearningEngine::new("persist-test", &profile, true, 30);
            engine.record("#like", true).unwrap();
            engine.record("#like", false).unwrap();
            engine.record("#retweet", true).unwrap();
            engine.save().unwrap();
        }

        // Load in new engine
        {
            let engine = LearningEngine::new("persist-test", &profile, true, 30);
            let like_stats = engine.selector_stats("#like");
            assert_eq!(like_stats.attempts, 2, "like should have 2 attempts");
            assert_eq!(like_stats.successes, 1, "like should have 1 success");

            let retweet_stats = engine.selector_stats("#retweet");
            assert_eq!(retweet_stats.attempts, 1, "retweet should have 1 attempt");
            assert_eq!(retweet_stats.successes, 1, "retweet should have 1 success");
        }

        std::env::set_current_dir(original_dir).ok();
    }

    #[test]
    fn test_consecutive_failures_triggers_stronger_adaptation() {
        let profile = create_test_profile();
        let mut engine = LearningEngine::new("test-session", &profile, true, 30);

        // Record 2 consecutive failures
        engine.record("#tricky", false).unwrap();
        engine.record("#tricky", false).unwrap();

        let context = create_test_context();
        let adaptation = engine.adaptation_for("#tricky", &context);

        // Consecutive failures should trigger stricter verification
        assert!(adaptation.require_strict_verification);
        assert!(adaptation.extra_stability_wait_ms >= 380);
    }

    #[test]
    fn test_complex_selector_gets_extra_wait() {
        let profile = create_test_profile();
        let mut engine = LearningEngine::new("test-session", &profile, true, 30);

        // Record success on complex selector
        engine
            .record(
                "div.container > button[data-testid='submit']:nth-child(2)",
                true,
            )
            .unwrap();

        let context = create_test_context();
        let adaptation = engine.adaptation_for(
            "div.container > button[data-testid='submit']:nth-child(2)",
            &context,
        );

        // Complex selector should get extra stability wait
        assert!(adaptation.extra_stability_wait_ms >= 120);
    }

    #[test]
    fn test_backward_compatibility_no_timestamp() {
        // Simulate old data without last_updated
        let stats = SelectorLearningStats {
            attempts: 5,
            successes: 4,
            consecutive_failures: 0,
            last_updated: None,
        };

        // Should not be pruned (backward compat)
        let profile = create_test_profile();
        let mut engine = LearningEngine::new("compat-test", &profile, true, 7);
        engine.state.selectors.insert("legacy".to_string(), stats);

        let pruned = engine.prune_expired().unwrap();
        assert_eq!(pruned, 0); // Not pruned because no timestamp
    }
}
