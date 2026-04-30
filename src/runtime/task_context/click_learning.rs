//! Click learning and adaptation system.
//!
//! Tracks click success/failure patterns and adapts timing strategies
//! based on page context, element priority, and fatigue levels.

use crate::utils::profile::BrowserProfile;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

/// Page context classification for click timing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClickPageContext {
    Home,
    Form,
    Social,
    Content,
    Commerce,
    Other,
}

/// Element priority classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClickElementPriority {
    Critical,
    Normal,
    Optional,
}

/// Fatigue level based on interaction count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClickFatigueLevel {
    Rested,
    Normal,
    Tired,
}

/// Timing context for a click operation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClickTimingContext {
    pub(crate) page: ClickPageContext,
    pub(crate) priority: ClickElementPriority,
    pub(crate) fatigue: ClickFatigueLevel,
    pub(crate) recent_success_rate: f64,
}

/// Computed timing profile for a click operation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClickTimingProfile {
    pub reaction_delay_ms: u64,
    pub reaction_variance_pct: u32,
    pub click_offset_px: i32,
    pub attention_pause_ms: u64,
    pub post_click_pause_ms: u64,
    pub primary_timeout_ms: u64,
}

/// Adaptations applied based on learning.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClickAdaptation {
    pub extra_stability_wait_ms: u64,
    pub reaction_delay_multiplier: f64,
    pub reaction_variance_boost_pct: u32,
    pub click_offset_adjustment_px: i32,
    pub require_strict_verification: bool,
    pub prefer_coordinate_fallback: bool,
}

impl Default for ClickAdaptation {
    fn default() -> Self {
        Self {
            extra_stability_wait_ms: 0,
            reaction_delay_multiplier: 1.0,
            reaction_variance_boost_pct: 0,
            click_offset_adjustment_px: 0,
            require_strict_verification: false,
            prefer_coordinate_fallback: false,
        }
    }
}

/// Statistics for a specific selector.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SelectorLearningStats {
    pub attempts: u32,
    pub successes: u32,
    pub consecutive_failures: u32,
}

/// Click learning state tracking selector performance.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ClickLearningState {
    pub interaction_count: u64,
    pub total_attempts: u64,
    pub total_successes: u64,
    pub recent_results: VecDeque<bool>,
    pub selectors: HashMap<String, SelectorLearningStats>,
}

impl ClickLearningState {
    pub const RECENT_WINDOW: usize = 32;

    /// Calculate recent success rate (last 32 results).
    pub fn recent_success_rate(&self) -> f64 {
        if self.recent_results.is_empty() {
            return 1.0;
        }
        let success_count = self.recent_results.iter().filter(|v| **v).count();
        success_count as f64 / self.recent_results.len() as f64
    }

    /// Record a click result for learning.
    pub fn record(&mut self, selector: &str, success: bool) {
        self.interaction_count = self.interaction_count.saturating_add(1);
        self.total_attempts = self.total_attempts.saturating_add(1);
        if success {
            self.total_successes = self.total_successes.saturating_add(1);
        }

        self.recent_results.push_back(success);
        if self.recent_results.len() > Self::RECENT_WINDOW {
            let _ = self.recent_results.pop_front();
        }

        let entry = self.selectors.entry(selector.to_string()).or_default();
        entry.attempts = entry.attempts.saturating_add(1);
        if success {
            entry.successes = entry.successes.saturating_add(1);
            entry.consecutive_failures = 0;
        } else {
            entry.consecutive_failures = entry.consecutive_failures.saturating_add(1);
        }
    }

    /// Get statistics for a specific selector.
    pub fn selector_stats(&self, selector: &str) -> SelectorLearningStats {
        self.selectors.get(selector).cloned().unwrap_or_default()
    }

    /// Calculate adaptations based on context and selector performance.
    pub fn adaptation_for(&self, selector: &str, context: &ClickTimingContext) -> ClickAdaptation {
        let mut adaptation = ClickAdaptation::default();
        let selector_stats = self.selector_stats(selector);

        let selector_complexity = selector.len() > 45
            || selector.contains(":nth-child")
            || selector.contains('>')
            || selector.contains("data-testid");
        if selector_complexity {
            adaptation.extra_stability_wait_ms = adaptation.extra_stability_wait_ms.max(120);
            adaptation.reaction_delay_multiplier *= 1.08;
            adaptation.click_offset_adjustment_px += 1;
        }

        if selector_stats.attempts >= 3 {
            let selector_success_rate =
                selector_stats.successes as f64 / selector_stats.attempts as f64;
            if selector_success_rate < 0.75 {
                adaptation.extra_stability_wait_ms = adaptation.extra_stability_wait_ms.max(250);
                adaptation.reaction_delay_multiplier *= 1.20;
                adaptation.reaction_variance_boost_pct += 8;
                adaptation.require_strict_verification = true;
                adaptation.prefer_coordinate_fallback = true;
            }
        }

        if selector_stats.consecutive_failures >= 2 {
            adaptation.extra_stability_wait_ms = adaptation.extra_stability_wait_ms.max(380);
            adaptation.reaction_delay_multiplier *= 1.22;
            adaptation.reaction_variance_boost_pct += 10;
            adaptation.click_offset_adjustment_px += 2;
            adaptation.require_strict_verification = true;
            adaptation.prefer_coordinate_fallback = true;
        }

        if context.fatigue == ClickFatigueLevel::Tired {
            adaptation.reaction_delay_multiplier *= 1.15;
            adaptation.reaction_variance_boost_pct += 6;
            adaptation.extra_stability_wait_ms = adaptation.extra_stability_wait_ms.max(140);
        }

        adaptation
    }
}

impl ClickTimingContext {
    /// Classify page type from URL.
    pub fn classify_page(url: &str) -> ClickPageContext {
        if url.contains("x.com") || url.contains("twitter.com") {
            ClickPageContext::Social
        } else if url.contains("login") || url.contains("signup") || url.contains("form") {
            ClickPageContext::Form
        } else if url.contains("shop") || url.contains("cart") || url.contains("checkout") {
            ClickPageContext::Commerce
        } else if url.contains("article") || url.contains("news") || url.contains("blog") {
            ClickPageContext::Content
        } else if url.trim_end_matches('/').matches('/').count() <= 2 {
            ClickPageContext::Home
        } else {
            ClickPageContext::Other
        }
    }

    /// Classify element priority from selector.
    pub fn classify_priority(selector: &str) -> ClickElementPriority {
        if selector.contains("submit")
            || selector.contains("confirm")
            || selector.contains("primary")
            || selector.contains("cta")
        {
            ClickElementPriority::Critical
        } else if selector.contains("ad")
            || selector.contains("promo")
            || selector.contains("secondary")
        {
            ClickElementPriority::Optional
        } else {
            ClickElementPriority::Normal
        }
    }

    /// Classify fatigue level from interaction count.
    pub fn classify_fatigue(interaction_count: u64) -> ClickFatigueLevel {
        if interaction_count < 15 {
            ClickFatigueLevel::Rested
        } else if interaction_count < 50 {
            ClickFatigueLevel::Normal
        } else {
            ClickFatigueLevel::Tired
        }
    }

    /// Create context from observation.
    pub fn from_observation(
        url: &str,
        selector: &str,
        interaction_count: u64,
        recent_success_rate: f64,
    ) -> Self {
        Self {
            page: Self::classify_page(url),
            priority: Self::classify_priority(selector),
            fatigue: Self::classify_fatigue(interaction_count),
            recent_success_rate,
        }
    }

    /// Compute timing profile based on context and adaptations.
    pub fn timing_profile(
        &self,
        base_reaction_delay_ms: u64,
        base_variance_pct: u32,
        base_offset_px: i32,
        adaptation: &ClickAdaptation,
    ) -> ClickTimingProfile {
        let page_multiplier = match self.page {
            ClickPageContext::Home => 0.95,
            ClickPageContext::Form => 1.20,
            ClickPageContext::Social => 1.10,
            ClickPageContext::Content => 1.00,
            ClickPageContext::Commerce => 1.15,
            ClickPageContext::Other => 1.00,
        };
        let priority_multiplier = match self.priority {
            ClickElementPriority::Critical => 1.18,
            ClickElementPriority::Normal => 1.00,
            ClickElementPriority::Optional => 0.92,
        };
        let fatigue_multiplier = match self.fatigue {
            ClickFatigueLevel::Rested => 0.95,
            ClickFatigueLevel::Normal => 1.00,
            ClickFatigueLevel::Tired => 1.22,
        };
        let quality_multiplier = if self.recent_success_rate < 0.75 {
            1.0 + (0.75 - self.recent_success_rate) * 0.50
        } else {
            1.0
        };

        let reaction_multiplier = page_multiplier
            * priority_multiplier
            * fatigue_multiplier
            * quality_multiplier
            * adaptation.reaction_delay_multiplier;
        let reaction_delay_ms = (base_reaction_delay_ms as f64 * reaction_multiplier)
            .round()
            .clamp(70.0, 6_000.0) as u64;

        let fatigue_variance_boost = match self.fatigue {
            ClickFatigueLevel::Rested => 0,
            ClickFatigueLevel::Normal => 4,
            ClickFatigueLevel::Tired => 10,
        };
        let reaction_variance_pct = base_variance_pct
            .saturating_add(adaptation.reaction_variance_boost_pct)
            .saturating_add(fatigue_variance_boost)
            .clamp(8, 80);

        let click_offset_px = (base_offset_px + adaptation.click_offset_adjustment_px).clamp(2, 24);

        let mut attention_pause_ms: u64 = match self.fatigue {
            ClickFatigueLevel::Rested => 60,
            ClickFatigueLevel::Normal => 120,
            ClickFatigueLevel::Tired => 230,
        };
        if self.priority == ClickElementPriority::Critical {
            attention_pause_ms += 80;
        }
        attention_pause_ms = attention_pause_ms
            .saturating_add(adaptation.extra_stability_wait_ms / 3)
            .clamp(40, 800);

        let mut post_click_pause_ms = match self.page {
            ClickPageContext::Form => 320,
            ClickPageContext::Commerce => 280,
            ClickPageContext::Social => 220,
            _ => 180,
        };
        if self.fatigue == ClickFatigueLevel::Tired {
            post_click_pause_ms += 80;
        }
        post_click_pause_ms = post_click_pause_ms.clamp(120, 900);

        let primary_timeout_ms = (4_000u64)
            .saturating_add(adaptation.extra_stability_wait_ms)
            .clamp(3_200, 7_500);

        ClickTimingProfile {
            reaction_delay_ms,
            reaction_variance_pct,
            click_offset_px,
            attention_pause_ms,
            post_click_pause_ms,
            primary_timeout_ms,
        }
    }
}

/// Sanitize a path component for file storage.
pub(crate) fn sanitize_path_component(value: &str) -> String {
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

/// Get the file path for click learning data.
pub fn click_learning_path(session_id: &str, behavior_profile: &BrowserProfile) -> Option<PathBuf> {
    let base_dir = std::env::current_dir().ok()?.join("click-learning");
    let profile_component = sanitize_path_component(&behavior_profile.name);
    let session_component = sanitize_path_component(session_id);
    Some(
        base_dir
            .join(profile_component)
            .join(format!("{session_component}.json")),
    )
}

/// Load click learning state from file.
pub fn load_click_learning(path: &Path) -> Option<ClickLearningState> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save click learning state to file.
pub fn save_click_learning(path: &Path, state: &ClickLearningState) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state)?;
    fs::write(path, json)?;
    Ok(())
}
