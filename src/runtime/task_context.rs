//! Task API context for browser automation.
//!
//! The `TaskContext` provides a high-level, task-facing API for browser automation.
//! Tasks should use this API exclusively rather than accessing internal utilities directly.
//!
//! # Task API Verbs
//!
//! The TaskContext provides short, readable verbs for common actions:
//! - `click()` - Click an element with human-like cursor movement
//! - `nativeclick()` - Click an element using native OS input
//! - `nativecursor()` - Move native cursor to a visible element
//! - `keyboard()` or `r#type()` - Type text with human-like timing
//! - `hover()` - Hover over an element
//! - `focus()` - Focus an element
//! - `navigate()` - Navigate to a URL
//! - `scroll()` - Scroll the page
//! - `wait_for()` - Wait for an element to appear
//! - `exists()` - Check if an element exists
//! - `visible()` - Check if an element is visible
//! - `text()` - Get element text
//! - `html()` - Get element HTML
//! - `attr()` - Get element attribute
//! - `pause()` - Pause for a duration
//!
//! # Examples
//!
//! ```no_run
//! # use auto::runtime::task_context::TaskContext;
//! # use auto::config::NativeInteractionConfig;
//! # async fn example(api: &TaskContext) -> anyhow::Result<()> {
//! api.navigate("https://example.com", 30_000).await?;
//! api.click("#submit-button").await?;
//! api.keyboard("input", "Hello World").await?;
//! api.pause(1000).await;
//! # Ok(())
//! # }
//! ```

use chromiumoxide::Page;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::capabilities::{clipboard, keyboard, mouse, navigation, scroll, timing};
use crate::config::NativeInteractionConfig;
use crate::internal::page_size::{self, Viewport};
use crate::internal::profile::{BrowserProfile, ProfileRuntime};
use crate::logger::scoped_log_context;
use crate::metrics::{
    MetricsCollector, RUN_COUNTER_CLICK_ATTEMPTED, RUN_COUNTER_CLICK_FALLBACK_HIT,
    RUN_COUNTER_CLICK_STRICT_VERIFY_FAILED, RUN_COUNTER_CLICK_SUCCESS,
};
use crate::state::ClipboardState;
use crate::task::policy::TaskPolicy;
use crate::utils::mouse::{ClickOutcome, CursorMovementConfig, HoverOutcome, NativeCursorOutcome};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ClickPageContext {
    Home,
    Form,
    Social,
    Content,
    Commerce,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ClickElementPriority {
    Critical,
    Normal,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum ClickFatigueLevel {
    Rested,
    Normal,
    Tired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct ClickTimingContext {
    page: ClickPageContext,
    priority: ClickElementPriority,
    fatigue: ClickFatigueLevel,
    recent_success_rate: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct ClickTimingProfile {
    reaction_delay_ms: u64,
    reaction_variance_pct: u32,
    click_offset_px: i32,
    attention_pause_ms: u64,
    post_click_pause_ms: u64,
    primary_timeout_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct ClickAdaptation {
    extra_stability_wait_ms: u64,
    reaction_delay_multiplier: f64,
    reaction_variance_boost_pct: u32,
    click_offset_adjustment_px: i32,
    require_strict_verification: bool,
    prefer_coordinate_fallback: bool,
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

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct SelectorLearningStats {
    attempts: u32,
    successes: u32,
    consecutive_failures: u32,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct ClickLearningState {
    interaction_count: u64,
    total_attempts: u64,
    total_successes: u64,
    recent_results: VecDeque<bool>,
    selectors: HashMap<String, SelectorLearningStats>,
}

impl ClickLearningState {
    const RECENT_WINDOW: usize = 32;

    fn recent_success_rate(&self) -> f64 {
        if self.recent_results.is_empty() {
            return 1.0;
        }
        let success_count = self.recent_results.iter().filter(|v| **v).count();
        success_count as f64 / self.recent_results.len() as f64
    }

    fn record(&mut self, selector: &str, success: bool) {
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

    fn selector_stats(&self, selector: &str) -> SelectorLearningStats {
        self.selectors.get(selector).cloned().unwrap_or_default()
    }

    fn adaptation_for(&self, selector: &str, context: &ClickTimingContext) -> ClickAdaptation {
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

fn click_learning_path(session_id: &str, behavior_profile: &BrowserProfile) -> Option<PathBuf> {
    let base_dir = std::env::current_dir().ok()?.join("click-learning");
    let profile_component = sanitize_path_component(&behavior_profile.name);
    let session_component = sanitize_path_component(session_id);
    Some(
        base_dir
            .join(profile_component)
            .join(format!("{session_component}.json")),
    )
}

fn load_click_learning(path: &Path) -> Option<ClickLearningState> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_click_learning(path: &Path, state: &ClickLearningState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state)?;
    fs::write(path, json)?;
    Ok(())
}

impl ClickTimingContext {
    fn classify_page(url: &str) -> ClickPageContext {
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

    fn classify_priority(selector: &str) -> ClickElementPriority {
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

    fn classify_fatigue(interaction_count: u64) -> ClickFatigueLevel {
        if interaction_count < 15 {
            ClickFatigueLevel::Rested
        } else if interaction_count < 50 {
            ClickFatigueLevel::Normal
        } else {
            ClickFatigueLevel::Tired
        }
    }

    fn from_observation(
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

    fn timing_profile(
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Copy)]
pub struct FocusOutcome {
    pub focus: FocusStatus,
    pub x: f64,
    pub y: f64,
}

impl FocusOutcome {
    pub fn summary(&self) -> String {
        let status = match self.focus {
            FocusStatus::Success => "success",
            FocusStatus::Failed => "failed",
        };
        format!("focus:{status} ({:.1},{:.1})", self.x, self.y)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RandomCursorOutcome {
    pub x: f64,
    pub y: f64,
    pub movement: CursorMovementConfig,
}

impl RandomCursorOutcome {
    pub fn summary(&self) -> String {
        format!(
            "randomcursor ({:.1},{:.1}) delay:{}..{}",
            self.x,
            self.y,
            self.movement.min_step_delay_ms,
            self.movement
                .min_step_delay_ms
                .saturating_add(self.movement.max_step_delay_variance_ms)
        )
    }
}

fn nativeclick_public_log_line(selector: &str, x: f64, y: f64) -> String {
    format!("[task-api] clicked ({selector}) at {x:.1},{y:.1}")
}

#[derive(Debug, Clone)]
pub struct ClickAndWaitOutcome {
    pub click: ClickOutcome,
    pub next_selector: String,
    pub next_visible: WaitForVisibleStatus,
    pub timeout_ms: u64,
}

impl ClickAndWaitOutcome {
    pub fn summary(&self) -> String {
        let next_visible = match self.next_visible {
            WaitForVisibleStatus::Visible => "visible",
            WaitForVisibleStatus::Timeout => "timeout",
        };
        format!(
            "{} wait_for:{} visible:{} timeout:{}ms",
            self.click.summary(),
            self.next_selector,
            next_visible,
            self.timeout_ms
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitForVisibleStatus {
    Visible,
    Timeout,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::mouse::CursorMovementConfig;

    #[test]
    fn test_focus_summary_format() {
        let outcome = FocusOutcome {
            focus: FocusStatus::Success,
            x: 12.3,
            y: 45.6,
        };
        assert_eq!(outcome.summary(), "focus:success (12.3,45.6)");
    }

    #[test]
    fn test_randomcursor_summary_format() {
        let outcome = RandomCursorOutcome {
            x: 10.0,
            y: 20.0,
            movement: CursorMovementConfig {
                speed_multiplier: 1.0,
                min_step_delay_ms: 10,
                max_step_delay_variance_ms: 5,
                curve_spread: 20.0,
                steps: None,
                add_micro_pauses: true,
                path_style: crate::utils::mouse::PathStyle::Bezier,
                precision: crate::utils::mouse::Precision::Safe,
                speed: crate::utils::mouse::Speed::Normal,
            },
        };
        assert_eq!(outcome.summary(), "randomcursor (10.0,20.0) delay:10..15");
    }

    #[test]
    fn test_click_and_wait_summary_format() {
        let outcome = ClickAndWaitOutcome {
            click: ClickOutcome {
                click: crate::utils::mouse::ClickStatus::Success,
                x: 1.0,
                y: 2.0,
                screen_x: None,
                screen_y: None,
            },
            next_selector: ".next".into(),
            next_visible: WaitForVisibleStatus::Visible,
            timeout_ms: 500,
        };
        assert_eq!(
            outcome.summary(),
            "Clicked (1.0,2.0) wait_for:.next visible:visible timeout:500ms"
        );
    }

    #[test]
    fn test_click_and_wait_timeout_summary_format() {
        let outcome = ClickAndWaitOutcome {
            click: ClickOutcome {
                click: crate::utils::mouse::ClickStatus::Success,
                x: 1.0,
                y: 2.0,
                screen_x: None,
                screen_y: None,
            },
            next_selector: ".next".into(),
            next_visible: WaitForVisibleStatus::Timeout,
            timeout_ms: 500,
        };
        assert_eq!(
            outcome.summary(),
            "Clicked (1.0,2.0) wait_for:.next visible:timeout timeout:500ms"
        );
    }

    #[test]
    fn test_nativeclick_public_log_format() {
        let line = nativeclick_public_log_line("#submit", 708.04, 335.19);
        assert_eq!(line, "[task-api] clicked (#submit) at 708.0,335.2");
    }

    #[test]
    fn test_click_context_classification() {
        assert_eq!(
            ClickTimingContext::classify_page("https://x.com/home"),
            ClickPageContext::Social
        );
        assert_eq!(
            ClickTimingContext::classify_priority("button[data-testid='submit']"),
            ClickElementPriority::Critical
        );
        assert_eq!(
            ClickTimingContext::classify_fatigue(80),
            ClickFatigueLevel::Tired
        );
    }

    #[test]
    fn test_learning_adaptation_increases_after_failures() {
        let mut learning = ClickLearningState::default();
        for _ in 0..4 {
            learning.record("button[data-testid='retweet']", false);
        }
        let context = ClickTimingContext::from_observation(
            "https://x.com/home",
            "button[data-testid='retweet']",
            learning.interaction_count,
            learning.recent_success_rate(),
        );
        let adaptation = learning.adaptation_for("button[data-testid='retweet']", &context);
        assert!(adaptation.reaction_delay_multiplier > 1.0);
        assert!(adaptation.extra_stability_wait_ms >= 250);
        assert!(adaptation.require_strict_verification);
    }

    #[test]
    fn test_timing_profile_scales_with_low_success_rate() {
        let context = ClickTimingContext::from_observation(
            "https://x.com/home",
            "button[data-testid='like']",
            60,
            0.55,
        );
        let profile = context.timing_profile(250, 20, 8, &ClickAdaptation::default());
        assert!(profile.reaction_delay_ms >= 250);
        assert!(profile.attention_pause_ms >= 200);
    }

    #[test]
    fn test_learning_window_caps_recent_results() {
        let mut learning = ClickLearningState::default();
        for i in 0..80 {
            learning.record("a[href='/x']", i % 2 == 0);
        }
        assert_eq!(
            learning.recent_results.len(),
            ClickLearningState::RECENT_WINDOW
        );
    }

    #[test]
    fn test_click_learning_persistence_roundtrip() {
        let mut learning = ClickLearningState::default();
        learning.record("button[data-testid='like']", true);
        learning.record("button[data-testid='like']", false);
        learning.record("button[data-testid='retweet']", false);

        let unique = format!(
            "click-learning-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);

        save_click_learning(&path, &learning).expect("save click learning");
        let loaded = load_click_learning(&path).expect("load click learning");

        assert_eq!(loaded.interaction_count, learning.interaction_count);
        assert_eq!(loaded.total_attempts, learning.total_attempts);
        assert_eq!(loaded.total_successes, learning.total_successes);
        assert_eq!(loaded.recent_results.len(), learning.recent_results.len());
        assert_eq!(
            loaded.selector_stats("button[data-testid='like']").attempts,
            2
        );
        assert_eq!(
            loaded
                .selector_stats("button[data-testid='like']")
                .successes,
            1
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_click_page_context_variants() {
        assert_eq!(ClickPageContext::Home, ClickPageContext::Home);
        assert_eq!(ClickPageContext::Form, ClickPageContext::Form);
        assert_eq!(ClickPageContext::Social, ClickPageContext::Social);
    }

    #[test]
    fn test_click_element_priority_variants() {
        assert_eq!(ClickElementPriority::Critical, ClickElementPriority::Critical);
        assert_eq!(ClickElementPriority::Normal, ClickElementPriority::Normal);
        assert_eq!(ClickElementPriority::Optional, ClickElementPriority::Optional);
    }

    #[test]
    fn test_click_fatigue_level_variants() {
        assert_eq!(ClickFatigueLevel::Rested, ClickFatigueLevel::Rested);
        assert_eq!(ClickFatigueLevel::Normal, ClickFatigueLevel::Normal);
        assert_eq!(ClickFatigueLevel::Tired, ClickFatigueLevel::Tired);
    }

    #[test]
    fn test_click_adaptation_default() {
        let adaptation = ClickAdaptation::default();
        assert_eq!(adaptation.extra_stability_wait_ms, 0);
        assert_eq!(adaptation.reaction_delay_multiplier, 1.0);
        assert!(!adaptation.require_strict_verification);
    }

    #[test]
    fn test_selector_learning_stats_default() {
        let stats = SelectorLearningStats::default();
        assert_eq!(stats.attempts, 0);
        assert_eq!(stats.successes, 0);
        assert_eq!(stats.consecutive_failures, 0);
    }

    #[test]
    fn test_click_learning_state_default() {
        let state = ClickLearningState::default();
        assert_eq!(state.interaction_count, 0);
        assert_eq!(state.total_attempts, 0);
        assert!(state.recent_results.is_empty());
    }

    #[test]
    fn test_click_learning_state_recent_success_rate_empty() {
        let state = ClickLearningState::default();
        assert_eq!(state.recent_success_rate(), 1.0);
    }

    #[test]
    fn test_click_learning_state_record_success() {
        let mut state = ClickLearningState::default();
        state.record("#button", true);
        assert_eq!(state.total_attempts, 1);
        assert_eq!(state.total_successes, 1);
    }

    #[test]
    fn test_click_learning_state_record_failure() {
        let mut state = ClickLearningState::default();
        state.record("#button", false);
        assert_eq!(state.total_attempts, 1);
        assert_eq!(state.total_successes, 0);
    }

    #[test]
    fn test_click_learning_state_selector_stats() {
        let mut state = ClickLearningState::default();
        state.record("#button", true);
        state.record("#button", false);
        let stats = state.selector_stats("#button");
        assert_eq!(stats.attempts, 2);
        assert_eq!(stats.successes, 1);
    }

    #[test]
    fn test_focus_status_variants() {
        assert_eq!(FocusStatus::Success, FocusStatus::Success);
        assert_eq!(FocusStatus::Failed, FocusStatus::Failed);
    }

    #[test]
    fn test_wait_for_visible_status_variants() {
        assert_eq!(WaitForVisibleStatus::Visible, WaitForVisibleStatus::Visible);
        assert_eq!(WaitForVisibleStatus::Timeout, WaitForVisibleStatus::Timeout);
    }

    #[test]
    fn test_sanitize_path_component_alphanumeric() {
        assert_eq!(sanitize_path_component("test123"), "test123");
    }

    #[test]
    fn test_sanitize_path_component_special_chars() {
        assert_eq!(sanitize_path_component("test@#$"), "test");
    }

    #[test]
    fn test_sanitize_path_component_empty() {
        assert_eq!(sanitize_path_component("@#$"), "default");
    }

    #[test]
    fn test_sanitize_path_component_spaces() {
        assert_eq!(sanitize_path_component("test name"), "test_name");
    }

    #[test]
    fn test_click_timing_context_classify_page_home() {
        assert_eq!(
            ClickTimingContext::classify_page("https://example.com/"),
            ClickPageContext::Home
        );
    }

    #[test]
    fn test_click_timing_context_classify_page_form() {
        assert_eq!(
            ClickTimingContext::classify_page("https://example.com/login"),
            ClickPageContext::Form
        );
    }

    #[test]
    fn test_click_timing_context_classify_priority_normal() {
        assert_eq!(
            ClickTimingContext::classify_priority("button"),
            ClickElementPriority::Normal
        );
    }

    #[test]
    fn test_click_timing_context_classify_priority_optional() {
        assert_eq!(
            ClickTimingContext::classify_priority("button.ad"),
            ClickElementPriority::Optional
        );
    }

    #[test]
    fn test_click_timing_context_classify_fatigue_rested() {
        assert_eq!(
            ClickTimingContext::classify_fatigue(10),
            ClickFatigueLevel::Rested
        );
    }

    #[test]
    fn test_click_timing_context_classify_fatigue_normal() {
        assert_eq!(
            ClickTimingContext::classify_fatigue(30),
            ClickFatigueLevel::Normal
        );
    }

    #[test]
    fn test_click_learning_state_consecutive_failures_tracking() {
        let mut state = ClickLearningState::default();
        state.record("#button", false);
        state.record("#button", false);
        let stats = state.selector_stats("#button");
        assert_eq!(stats.consecutive_failures, 2);
    }

    #[test]
    fn test_click_learning_state_consecutive_failures_reset_on_success() {
        let mut state = ClickLearningState::default();
        state.record("#button", false);
        state.record("#button", false);
        state.record("#button", true);
        let stats = state.selector_stats("#button");
        assert_eq!(stats.consecutive_failures, 0);
    }

    #[test]
    fn test_click_learning_state_multiple_selectors() {
        let mut state = ClickLearningState::default();
        state.record("#button1", true);
        state.record("#button2", false);
        let stats1 = state.selector_stats("#button1");
        let stats2 = state.selector_stats("#button2");
        assert_eq!(stats1.attempts, 1);
        assert_eq!(stats1.successes, 1);
        assert_eq!(stats2.attempts, 1);
        assert_eq!(stats2.successes, 0);
    }

    #[test]
    fn test_click_learning_state_interaction_count() {
        let mut state = ClickLearningState::default();
        state.record("#button", true);
        state.record("#button", false);
        state.record("#link", true);
        assert_eq!(state.interaction_count, 3);
    }

    #[test]
    fn test_click_learning_state_recent_success_rate_mixed() {
        let mut state = ClickLearningState::default();
        for i in 0..10 {
            state.record("#button", i % 2 == 0);
        }
        let rate = state.recent_success_rate();
        assert!(rate >= 0.0 && rate <= 1.0);
    }

    #[test]
    fn test_click_learning_state_recent_success_rate_all_success() {
        let mut state = ClickLearningState::default();
        for _ in 0..10 {
            state.record("#button", true);
        }
        assert_eq!(state.recent_success_rate(), 1.0);
    }

    #[test]
    fn test_click_learning_state_recent_success_rate_all_failure() {
        let mut state = ClickLearningState::default();
        for _ in 0..10 {
            state.record("#button", false);
        }
        assert_eq!(state.recent_success_rate(), 0.0);
    }

    #[test]
    fn test_click_adaptation_with_high_multiplier() {
        let adaptation = ClickAdaptation {
            extra_stability_wait_ms: 500,
            reaction_delay_multiplier: 2.5,
            require_strict_verification: true,
            click_offset_adjustment_px: 0,
            prefer_coordinate_fallback: false,
            reaction_variance_boost_pct: 0,
        };
        assert_eq!(adaptation.extra_stability_wait_ms, 500);
        assert_eq!(adaptation.reaction_delay_multiplier, 2.5);
        assert!(adaptation.require_strict_verification);
    }

    #[test]
    fn test_click_timing_profile_from_observation() {
        let context = ClickTimingContext::from_observation(
            "https://example.com",
            "#button",
            10,
            0.8,
        );
        assert_eq!(context.page, ClickPageContext::Home);
        assert_eq!(context.priority, ClickElementPriority::Normal);
        assert_eq!(context.fatigue, ClickFatigueLevel::Rested);
        assert_eq!(context.recent_success_rate, 0.8);
    }

    #[test]
    fn test_click_timing_context_classify_page_commerce() {
        assert_eq!(
            ClickTimingContext::classify_page("https://shop.example.com"),
            ClickPageContext::Commerce
        );
    }

    #[test]
    fn test_click_timing_context_classify_page_content() {
        assert_eq!(
            ClickTimingContext::classify_page("https://blog.example.com/article"),
            ClickPageContext::Content
        );
    }

    #[test]
    fn test_click_timing_context_classify_page_other() {
        // Need a URL with more than 2 path segments to trigger Other
        assert_eq!(
            ClickTimingContext::classify_page("https://unknown.example.com/path/to/page"),
            ClickPageContext::Other
        );
    }

    #[test]
    fn test_click_timing_context_classify_priority_critical_data_testid() {
        assert_eq!(
            ClickTimingContext::classify_priority("[data-testid='submit']"),
            ClickElementPriority::Critical
        );
    }

    #[test]
    fn test_click_timing_context_classify_priority_critical_type_submit() {
        assert_eq!(
            ClickTimingContext::classify_priority("button[type='submit']"),
            ClickElementPriority::Critical
        );
    }

    #[test]
    fn test_click_timing_context_classify_fatigue_boundary_normal() {
        // Boundary is < 50 for Normal, so 49 should be Normal
        assert_eq!(
            ClickTimingContext::classify_fatigue(49),
            ClickFatigueLevel::Normal
        );
    }

    #[test]
    fn test_click_timing_context_classify_fatigue_boundary_tired() {
        assert_eq!(
            ClickTimingContext::classify_fatigue(70),
            ClickFatigueLevel::Tired
        );
    }

    #[test]
    fn test_click_timing_context_classify_fatigue_boundary_rested() {
        // Boundary is < 15 for Rested, so 14 should be Rested
        assert_eq!(
            ClickTimingContext::classify_fatigue(14),
            ClickFatigueLevel::Rested
        );
    }

    #[test]
    fn test_sanitize_path_component_unicode() {
        assert_eq!(sanitize_path_component("test🎉"), "test");
    }

    #[test]
    fn test_sanitize_path_component_underscores() {
        assert_eq!(sanitize_path_component("test_name"), "test_name");
    }

    #[test]
    fn test_sanitize_path_component_dashes() {
        assert_eq!(sanitize_path_component("test-name"), "test-name");
    }

    #[test]
    fn test_click_learning_state_selector_stats_nonexistent() {
        let state = ClickLearningState::default();
        let stats = state.selector_stats("#nonexistent");
        assert_eq!(stats.attempts, 0);
        assert_eq!(stats.successes, 0);
    }
}

/// High-level API context for browser automation tasks.
///
/// `TaskContext` provides a task-facing API with human-like interaction patterns.
/// Tasks should use this API exclusively rather than accessing internal utilities.
///
/// # Features
///
/// - Human-like mouse movement with configurable paths and timing
/// - Keyboard typing with realistic delays
/// - Clipboard state management
/// - Behavior profiles for consistent interaction patterns
///
/// # Examples
///
/// ```no_run
/// # use auto::runtime::task_context::TaskContext;
/// # use chromiumoxide::Page;
/// # use std::sync::Arc;
/// # use auto::internal::profile::{BrowserProfile, ProfileRuntime};
/// # use auto::config::NativeInteractionConfig;
/// # use auto::task::policy::DEFAULT_TASK_POLICY;
/// # async fn example(page: Arc<Page>, profile: BrowserProfile, runtime: ProfileRuntime) {
/// let api = TaskContext::new("session-1", page, profile, runtime, NativeInteractionConfig::default(), &DEFAULT_TASK_POLICY);
/// // Use the API for browser automation
/// # }
/// ```
#[derive(Clone)]
pub struct TaskContext {
    session_id: String,
    page: Arc<Page>,
    clipboard: ClipboardState,
    behavior_profile: BrowserProfile,
    behavior_runtime: ProfileRuntime,
    native_interaction: NativeInteractionConfig,
    metrics: Option<Arc<MetricsCollector>>,
    click_learning: Arc<Mutex<ClickLearningState>>,
    click_learning_path: Option<PathBuf>,
    policy: &'static TaskPolicy,
}

impl TaskContext {
    /// Creates a new TaskContext for browser automation.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier
    /// * `page` - The browser page to automate
    /// * `behavior_profile` - The behavior profile for human-like interactions
    /// * `behavior_runtime` - The runtime behavior configuration
    /// * `native_interaction` - Native OS input calibration and timing settings
    ///
    /// # Returns
    ///
    /// A new `TaskContext` instance ready for task execution.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::runtime::task_context::TaskContext;
    /// # use chromiumoxide::Page;
    /// # use std::sync::Arc;
    /// # use auto::internal::profile::{BrowserProfile, ProfileRuntime};
    /// # use auto::config::NativeInteractionConfig;
    /// # use auto::task::policy::DEFAULT_TASK_POLICY;
    /// # async fn example(page: Arc<Page>, profile: BrowserProfile, runtime: ProfileRuntime) {
    /// let api = TaskContext::new("session-1", page, profile, runtime, NativeInteractionConfig::default(), &DEFAULT_TASK_POLICY);
    /// # }
    /// ```
    pub fn new(
        session_id: impl Into<String>,
        page: Arc<Page>,
        behavior_profile: BrowserProfile,
        behavior_runtime: ProfileRuntime,
        native_interaction: NativeInteractionConfig,
        policy: &'static TaskPolicy,
    ) -> Self {
        let session_id = session_id.into();
        let clipboard = ClipboardState::new(session_id.clone());
        let click_learning_path = click_learning_path(&session_id, &behavior_profile);
        let click_learning = click_learning_path
            .as_deref()
            .and_then(load_click_learning)
            .unwrap_or_default();
        Self {
            session_id,
            page,
            clipboard,
            behavior_profile,
            behavior_runtime,
            native_interaction,
            metrics: None,
            click_learning: Arc::new(Mutex::new(click_learning)),
            click_learning_path,
            policy,
        }
    }

    pub fn new_with_metrics(
        session_id: impl Into<String>,
        page: Arc<Page>,
        behavior_profile: BrowserProfile,
        behavior_runtime: ProfileRuntime,
        native_interaction: NativeInteractionConfig,
        metrics: Arc<MetricsCollector>,
        policy: &'static TaskPolicy,
    ) -> Self {
        let mut ctx = Self::new(
            session_id,
            page,
            behavior_profile,
            behavior_runtime,
            native_interaction,
            policy,
        );
        ctx.metrics = Some(metrics);
        ctx
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(crate) fn page(&self) -> &Page {
        &self.page
    }

    pub fn clipboard(&self) -> &ClipboardState {
        &self.clipboard
    }

    pub fn behavior_profile(&self) -> &BrowserProfile {
        &self.behavior_profile
    }

    pub fn behavior_runtime(&self) -> &ProfileRuntime {
        &self.behavior_runtime
    }

    pub fn native_interaction(&self) -> &NativeInteractionConfig {
        &self.native_interaction
    }

    pub fn increment_run_counter(&self, name: &str, amount: usize) {
        if let Some(metrics) = &self.metrics {
            metrics.increment_run_counter(name, amount);
        }
    }

    pub fn metrics(&self) -> &MetricsCollector {
        self.metrics
            .as_ref()
            .expect("Metrics collector not initialized")
    }

    fn click_learning_path(&self) -> Option<&Path> {
        self.click_learning_path.as_deref()
    }

    async fn record_click_learning(&self, selector: &str, success: bool) -> Result<()> {
        let snapshot = {
            let mut learning = self.click_learning.lock().await;
            learning.record(selector, success);
            learning.clone()
        };

        if let Some(path) = self.click_learning_path() {
            save_click_learning(path, &snapshot)?;
        }

        Ok(())
    }

    /// Navigates to a URL with human-like delays and settle pauses.
    ///
    /// This method performs navigation with realistic timing:
    /// - Pre-navigation action delay
    /// - Post-navigation settle pause
    /// - Wait for page load completion
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to navigate to
    /// * `timeout_ms` - Maximum time to wait for navigation to complete
    ///
    /// # Errors
    ///
    /// Returns an error if navigation fails or times out.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::runtime::task_context::TaskContext;
    /// # async fn example(api: &TaskContext) -> anyhow::Result<()> {
    /// api.navigate("https://example.com", 30000).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn navigate(&self, url: &str, timeout_ms: u64) -> Result<()> {
        navigation::goto(self.page(), url, timeout_ms).await?;

        let action_delay = &self.behavior_runtime.action_delay;
        timing::human_pause(
            action_delay.min_ms,
            action_delay.variance_pct.round() as u32,
        )
        .await;

        let settle_base = action_delay
            .min_ms
            .saturating_add(timeout_ms.min(2_000) / 4)
            .clamp(150, 4_000);
        let settle_variance = action_delay.variance_pct.round().clamp(10.0, 60.0) as u32;
        timing::human_pause(settle_base, settle_variance).await;

        let settle_ms = timeout_ms.min(3_000);
        let _ = self.wait_for_load(settle_ms).await;
        self.post_interaction_pause().await;

        Ok(())
    }

    // --- Permission checking ---

    /// Check if a specific permission is granted.
    ///
    /// Returns `Ok(())` if permission is granted, `Err(TaskError::PermissionDenied)` if not.
    /// Uses effective_permissions() to account for implied permissions.
    ///
    /// # Arguments
    ///
    /// * `permission` - The permission name to check (e.g., "allow_screenshot")
    ///
    /// # Examples
    ///
    /// ```ignore
    /// self.check_permission("allow_screenshot")?;
    /// ```
    pub fn check_permission(&self, permission: &'static str) -> crate::error::Result<()> {
        let perms = self.policy.effective_permissions();

        let has_permission = match permission {
            "allow_screenshot" => perms.allow_screenshot,
            "allow_export_cookies" => perms.allow_export_cookies,
            "allow_import_cookies" => perms.allow_import_cookies,
            "allow_export_session" => perms.allow_export_session,
            "allow_import_session" => perms.allow_import_session,
            "allow_session_clipboard" => perms.allow_session_clipboard,
            "allow_read_data" => perms.allow_read_data,
            "allow_write_data" => perms.allow_write_data,
            _ => {
                log::warn!("Unknown permission '{}' requested", permission);
                false
            }
        };

        if has_permission {
            Ok(())
        } else {
            Err(crate::error::TaskError::PermissionDenied {
                permission,
                task_name: self.session_id.clone(),
            }
            .into())
        }
    }

    /// Check if the browser page is still connected.
    ///
    /// Performs a lightweight connectivity check before CDP operations.
    /// Uses the browser handle as a proxy for connection state.
    ///
    /// # Returns
    ///
    /// `Ok(())` if page appears connected, `Err(TaskError::CdpError)` if not.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// self.check_page_connected().await?;
    /// let cookies = self.export_cookies("").await?;
    /// ```
    pub async fn check_page_connected(&self) -> crate::error::Result<()> {
        // Try a lightweight JS evaluation to verify actual connectivity
        // This detects if the page has been closed or the browser disconnected
        match self.page.evaluate("1").await {
            Ok(_) => Ok(()),
            Err(e) => Err(crate::error::TaskError::CdpError {
                operation: "Page.connection_check".to_string(),
                reason: format!("Page not responding to CDP: {}", e),
            }
            .into()),
        }
    }

    // --- Permission-gated operations ---

    /// Check if task has screenshot permission.
    pub async fn screenshot(&self) -> Result<Vec<u8>> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_screenshot {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_screenshot' permission",
                self.session_id
            ));
        }
        // CDP: Page.captureScreenshot
        self.page
            .screenshot(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotParams::default())
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Page.captureScreenshot - {}", e))
    }

    /// Check if task has cookie export permission.
    pub async fn export_cookies(&self, _url: &str) -> Result<Vec<serde_json::Value>> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_export_cookies {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_export_cookies' permission",
                self.session_id
            ));
        }
        // CDP: Network.getCookies
        let cookies = self
            .page
            .execute(chromiumoxide::cdp::browser_protocol::network::GetCookiesParams::default())
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Network.getCookies - {}", e))?;
        // Convert cookies to serde_json::Value for portability
        let json = serde_json::to_value(&cookies.cookies)
            .map_err(|e| anyhow::anyhow!("Failed to serialize cookies: {}", e))?;
        Ok(json.as_array().unwrap_or(&vec![]).clone())
    }

    /// Check if task has clipboard read permission.
    pub fn read_clipboard(&self) -> Result<String> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_session_clipboard {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_session_clipboard' permission",
                self.session_id
            ));
        }
        crate::state::ClipboardState::new(self.session_id.clone())
            .get()
            .ok_or_else(|| anyhow::anyhow!("Clipboard empty or session not found"))
    }

    /// Check if task has clipboard write permission.
    pub fn write_clipboard(&self, text: &str) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_session_clipboard {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_session_clipboard' permission",
                self.session_id
            ));
        }
        crate::state::ClipboardState::new(self.session_id.clone()).set(text);
        Ok(())
    }

    /// Check if task has read data permission.
    pub fn read_data_file(&self, relative_path: &str) -> Result<String> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_read_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_read_data' permission",
                self.session_id
            ));
        }
        // Validate path is within config/ or data/
        let base_dirs = [std::path::Path::new("config"), std::path::Path::new("data")];
        let mut final_path = None;
        for base in &base_dirs {
            let path = base.join(relative_path);
            if path.exists() {
                final_path = Some(path);
                break;
            }
        }
        let path = final_path.ok_or_else(|| anyhow::anyhow!("File not found: {}", relative_path))?;
        // Simple path validation: reject absolute paths and traversal
        let path_str = path.to_string_lossy();
        if path.is_absolute() || path_str.contains("..") {
            return Err(anyhow::anyhow!("Invalid path: Path not allowed"));
        }
        std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))
    }

    /// Check if task has write data permission.
    pub fn write_data_file(&self, relative_path: &str, content: &[u8]) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_write_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_write_data' permission",
                self.session_id
            ));
        }
        let base_dirs = [std::path::Path::new("config"), std::path::Path::new("data")];
        let mut final_path = None;
        for base in &base_dirs {
            let path = base.join(relative_path);
            if let Some(parent) = path.parent() {
                if parent.exists() || std::fs::create_dir_all(parent).is_ok() {
                    final_path = Some(path);
                    break;
                }
            }
        }
        let path = final_path.ok_or_else(|| anyhow::anyhow!("Cannot determine write path"))?;
        let path_str = path.to_string_lossy();
        if path.is_absolute() || path_str.contains("..") {
            return Err(anyhow::anyhow!("Invalid path: Path not allowed"));
        }
        std::fs::write(&path, content)
            .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))
    }

    /// Import cookies from a Vec<serde_json::Value> (each must have name, value, domain).
    pub async fn import_cookies(&self, cookies: &[serde_json::Value]) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_import_cookies {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_import_cookies' permission",
                self.session_id
            ));
        }

        for cookie in cookies {
            let name = cookie.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let value = cookie.get("value").and_then(|v| v.as_str()).unwrap_or("");
            let domain = cookie.get("domain").and_then(|v| v.as_str());
            let path = cookie.get("path").and_then(|v| v.as_str());
            if name.is_empty() || value.is_empty() {
                continue;
            }
            let mut params = chromiumoxide::cdp::browser_protocol::network::SetCookieParams::builder()
                .name(name)
                .value(value);
            if let Some(d) = domain {
                params = params.domain(d);
            }
            if let Some(p) = path {
                params = params.path(p);
            }
            let params = params.build().map_err(|e| {
                anyhow::anyhow!("Failed to build SetCookieParams: {}", e)
            })?;
            self.page.execute(params).await.map_err(|e| {
                anyhow::anyhow!("CDP error: Network.setCookie - {}", e)
            })?;
        }

        log::warn!(
            "task_policy_audit: task={} permission={} count={}",
            self.session_id, "allow_import_cookies", cookies.len()
        );
        Ok(())
    }

    /// Export session data (cookies + localStorage) as SessionData.
    pub async fn export_session(&self, url: &str) -> Result<crate::task::policy::SessionData> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_export_session {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_export_session' permission",
                self.session_id
            ));
        }

        // Export cookies via CDP
        let cookies_result = self
            .page
            .execute(chromiumoxide::cdp::browser_protocol::network::GetCookiesParams::default())
            .await;
        let cookies_json = match cookies_result {
            Ok(cookies) => {
                serde_json::to_value(&cookies.cookies).unwrap_or(serde_json::Value::Array(vec![]))
            }
            Err(e) => {
                log::warn!("Failed to export cookies: {}", e);
                serde_json::Value::Array(vec![])
            }
        };
        let cookies = cookies_json
            .as_array()
            .unwrap_or(&vec![])
            .clone();

        // Export localStorage via JavaScript
        let local_storage_js = r#"
            (function() {
                const data = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    data[key] = localStorage.getItem(key);
                }
                return JSON.stringify(data);
            })()
        "#;
        let local_storage_str = self
            .page
            .evaluate(local_storage_js)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {}", e))?;
        let local_storage_value = local_storage_str.value().cloned().unwrap_or(serde_json::Value::Null);
        let local_storage: std::collections::HashMap<String, String> =
            serde_json::from_value(local_storage_value)
                .unwrap_or_default();

        let session_data = crate::task::policy::SessionData {
            cookies,
            local_storage,
            exported_at: chrono::Utc::now(),
            url: url.to_string(),
        };

        log::warn!(
            "task_policy_audit: task={} permission={} url={} count={}",
            self.session_id, "allow_export_session", url, session_data.cookies.len()
        );

        Ok(session_data)
    }

    /// Import session data (cookies + localStorage) from SessionData.
    pub async fn import_session(&self, session_data: &crate::task::policy::SessionData) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_import_session {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_import_session' permission",
                self.session_id
            ));
        }

        // Import cookies
        self.import_cookies(&session_data.cookies).await?;

        // Import localStorage via JavaScript
        let local_storage_json = serde_json::to_string(&session_data.local_storage)
            .map_err(|e| anyhow::anyhow!("Failed to serialize localStorage: {}", e))?;
        let js_code = format!(
            r#"
            (function() {{
                const data = {};
                Object.entries(data).forEach(([k, v]) => {{
                    localStorage.setItem(k, v);
                }});
                return 'localStorage restored';
            }})()
            "#,
            local_storage_json
        );
        self.page
            .evaluate(js_code)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {}", e))?;

        log::warn!(
            "task_policy_audit: task={} permission={} url={} count={}",
            self.session_id, "allow_import_session", session_data.url, session_data.cookies.len()
        );

        Ok(())
    }

    /// Set custom user agent string for subsequent navigations.
    pub async fn set_user_agent(&self, user_agent: &str) -> Result<()> {
        navigation::set_user_agent(self.page(), user_agent).await
    }

    /// Set extra HTTP headers for subsequent navigations.
    pub async fn set_extra_http_headers(&self, headers: &BTreeMap<String, String>) -> Result<()> {
        navigation::set_extra_http_headers(self.page(), headers).await
    }

    /// Apply user agent and/or extra HTTP headers in one call.
    pub async fn apply_browser_context(
        &self,
        user_agent: Option<&str>,
        headers: &BTreeMap<String, String>,
    ) -> Result<()> {
        if let Some(user_agent) = user_agent {
            self.set_user_agent(user_agent).await?;
        }
        if !headers.is_empty() {
            self.set_extra_http_headers(headers).await?;
        }
        Ok(())
    }

    /// Wait for 'load' event with timeout. Uses page load event.
    pub async fn wait_for_load(&self, timeout_ms: u64) -> Result<()> {
        navigation::wait_for_load(self.page(), timeout_ms).await
    }

    /// Wait until any of the given selectors becomes visible. Returns first match or false.
    pub async fn wait_for_any_visible_selector(
        &self,
        selectors: &[&str],
        timeout_ms: u64,
    ) -> Result<bool> {
        navigation::wait_for_any_visible_selector(self.page(), selectors, timeout_ms).await
    }

    /// Scrolls an element into view, focuses it, and returns the focus outcome.
    ///
    /// This method:
    /// - Scrolls the element into view if needed
    /// - Focuses the element
    /// - Returns the center coordinates and focus status
    ///
    /// # Arguments
    ///
    /// * `selector` - CSS selector for the element to focus
    ///
    /// # Returns
    ///
    /// A `FocusOutcome` containing the focus status and element coordinates.
    ///
    /// # Errors
    ///
    /// Returns an error if the element cannot be found or focused.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::runtime::task_context::TaskContext;
    /// # async fn example(api: &TaskContext) -> anyhow::Result<()> {
    /// let outcome = api.focus("#input-field").await?;
    /// println!("Focus status: {:?}", outcome.focus);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn focus(&self, selector: &str) -> Result<FocusOutcome> {
        scroll::scroll_into_view(self.page(), selector).await?;
        let (x, y) = page_size::get_element_center(self.page(), selector).await?;
        navigation::focus(self.page(), selector).await?;
        self.post_interaction_pause().await;
        Ok(FocusOutcome {
            focus: FocusStatus::Success,
            x,
            y,
        })
    }

    /// Performs a human-like hover over an element with configurable timing.
    ///
    /// This method simulates realistic mouse movement with:
    /// - Configurable reaction delay
    /// - Timing variance for natural behavior
    /// - Offset from element center
    /// - Post-interaction pause
    ///
    /// # Arguments
    ///
    /// * `selector` - CSS selector for the element to hover over
    ///
    /// # Returns
    ///
    /// A `HoverOutcome` containing the hover status and coordinates.
    ///
    /// # Errors
    ///
    /// Returns an error if the element cannot be found or hovered.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::runtime::task_context::TaskContext;
    /// # async fn example(api: &TaskContext) -> anyhow::Result<()> {
    /// api.hover("#menu-item").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn hover(&self, selector: &str) -> Result<HoverOutcome> {
        let click = &self.behavior_runtime.click;
        let outcome = mouse::hover_selector_human(
            self.page(),
            selector,
            click.reaction_delay_ms / 2,
            self.behavior_runtime.action_delay.variance_pct.round() as u32,
            click.offset_px,
        )
        .await?;
        self.post_interaction_pause().await;
        Ok(outcome)
    }

    /// Move cursor to absolute coordinates with post-move pause for human-like behavior.
    pub async fn move_mouse_to(&self, x: f64, y: f64) -> Result<()> {
        mouse::cursor_move_to(self.page(), x, y).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    /// Immediate cursor move without animation or pause.
    pub async fn move_mouse_fast(&self, x: f64, y: f64) -> Result<()> {
        mouse::cursor_move_to_immediate(self.page(), x, y).await
    }

    /// Move cursor to a random viewport position for human-like behavior.
    pub async fn randomcursor(&self) -> Result<RandomCursorOutcome> {
        let viewport = self.viewport().await?;
        let edge_ratio = self
            .behavior_runtime
            .random_cursor_safe_edge_ratio
            .max(0.10);
        let (x, y) = page_size::random_position_with_edge_ratio(&viewport, edge_ratio);
        let config = self.behavior_profile.cursor_movement_config();
        mouse::cursor_move_to_with_config(self.page(), x, y, &config).await?;
        self.post_interaction_pause().await;
        Ok(RandomCursorOutcome {
            x,
            y,
            movement: config,
        })
    }

    /// Sync visual cursor overlay with actual cursor position.
    pub async fn sync_cursor_overlay(&self) -> Result<()> {
        mouse::sync_cursor_overlay(self.page()).await
    }

    /// Fast cursor move + left-click at raw coordinates.
    pub async fn click_at(&self, x: f64, y: f64) -> Result<()> {
        let fast_move = CursorMovementConfig {
            speed_multiplier: 2.5,
            min_step_delay_ms: 1,
            max_step_delay_variance_ms: 1,
            curve_spread: 20.0,
            steps: Some(8),
            add_micro_pauses: false,
            path_style: crate::utils::mouse::PathStyle::Bezier,
            precision: crate::utils::mouse::Precision::Safe,
            speed: crate::utils::mouse::Speed::Fast,
        };
        mouse::cursor_move_to_with_config(self.page(), x, y, &fast_move).await?;
        mouse::left_click_at_without_move(self.page(), x, y).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    /// Primary click method with selector pipeline and fallback to coordinate click.
    ///
    /// This method:
    /// - Runs the full selector pipeline (scroll, move, click)
    /// - Uses human-like cursor movement and timing
    /// - Falls back to coordinate click if selector fails
    /// - Includes post-interaction pause
    ///
    /// # Arguments
    ///
    /// * `selector` - CSS selector for the element to click
    ///
    /// # Returns
    ///
    /// A `ClickOutcome` containing the click status and coordinates.
    ///
    /// # Errors
    ///
    /// Returns an error if both selector and coordinate clicks fail.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::runtime::task_context::TaskContext;
    /// # async fn example(api: &TaskContext) -> anyhow::Result<()> {
    /// api.click("#submit-button").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn click(&self, selector: &str) -> Result<ClickOutcome> {
        const CLICK_TOTAL_TIMEOUT_SECS: u64 = 12;
        const CLICK_MAX_ATTEMPTS: u32 = 3;
        let click = &self.behavior_runtime.click;
        self.increment_run_counter(RUN_COUNTER_CLICK_ATTEMPTED, 1);
        let default_url = String::new();
        let observed_url = self.url().await.unwrap_or(default_url);
        let base_variance = self.behavior_runtime.action_delay.variance_pct.round() as u32;

        let (timing_profile, adaptation, fatigue, recent_success_rate) = {
            let learning = self.click_learning.lock().await;
            let timing_context = ClickTimingContext::from_observation(
                &observed_url,
                selector,
                learning.interaction_count,
                learning.recent_success_rate(),
            );
            let adaptation = learning.adaptation_for(selector, &timing_context);
            let timing_profile = timing_context.timing_profile(
                click.reaction_delay_ms,
                base_variance,
                click.offset_px,
                &adaptation,
            );
            (
                timing_profile,
                adaptation,
                timing_context.fatigue,
                timing_context.recent_success_rate,
            )
        };

        timing::human_pause(
            timing_profile.attention_pause_ms,
            timing_profile.reaction_variance_pct.min(45),
        )
        .await;

        let click_future = async {
            let mut last_error: Option<anyhow::Error> = None;

            for attempt in 1..=CLICK_MAX_ATTEMPTS {
                let attempt_delay = (timing_profile.reaction_delay_ms as f64
                    * (1.0 + ((attempt.saturating_sub(1)) as f64 * 0.18)))
                    .round() as u64;
                let attempt_offset = timing_profile.click_offset_px + (attempt as i32 - 1);

                match self
                    .execute_primary_click_attempt(
                        selector,
                        attempt_delay,
                        timing_profile.reaction_variance_pct,
                        attempt_offset,
                        timing_profile.primary_timeout_ms,
                    )
                    .await
                {
                    Ok(outcome) => return Ok(outcome),
                    Err(err) => {
                        last_error = Some(err);
                        if attempt < CLICK_MAX_ATTEMPTS {
                            let backoff_ms = (150 + (attempt as u64 * 180))
                                .saturating_add(adaptation.extra_stability_wait_ms / 2)
                                .clamp(100, 1_000);
                            timing::uniform_pause(backoff_ms, 30).await;
                        }
                    }
                }
            }

            if adaptation.prefer_coordinate_fallback {
                warn!(
                    "[task-api] click '{}' entering coordinate fallback after retry exhaustion",
                    selector
                );
            }
            self.increment_run_counter(RUN_COUNTER_CLICK_FALLBACK_HIT, 1);

            match self
                .fallback_click_with_adaptation(selector, &adaptation)
                .await
            {
                Ok(outcome) => Ok(outcome),
                Err(fallback_err) => Err(last_error.unwrap_or(fallback_err)),
            }
        };

        let outcome =
            match tokio::time::timeout(Duration::from_secs(CLICK_TOTAL_TIMEOUT_SECS), click_future)
                .await
            {
                Ok(Ok(outcome)) => outcome,
                Ok(Err(err)) => {
                    if let Err(persist_err) = self.record_click_learning(selector, false).await {
                        warn!(
                            "[task-api] click learning persistence failed: {}",
                            persist_err
                        );
                    }
                    return Err(err);
                }
                Err(_) => {
                    if let Err(persist_err) = self.record_click_learning(selector, false).await {
                        warn!(
                            "[task-api] click learning persistence failed: {}",
                            persist_err
                        );
                    }
                    return Err(anyhow::anyhow!("click timed out for '{}'", selector));
                }
            };

        if adaptation.require_strict_verification {
            let verified = self
                .verify_selector_hit(selector, outcome.x, outcome.y)
                .await
                .unwrap_or(false);
            if !verified {
                self.increment_run_counter(RUN_COUNTER_CLICK_STRICT_VERIFY_FAILED, 1);
                if let Err(persist_err) = self.record_click_learning(selector, false).await {
                    warn!(
                        "[task-api] click learning persistence failed: {}",
                        persist_err
                    );
                }
                self.increment_run_counter(RUN_COUNTER_CLICK_FALLBACK_HIT, 1);
                match self
                    .fallback_click_with_adaptation(selector, &adaptation)
                    .await
                {
                    Ok(fallback_outcome) => {
                        if let Err(persist_err) = self.record_click_learning(selector, true).await {
                            warn!(
                                "[task-api] click learning persistence failed: {}",
                                persist_err
                            );
                        }
                        self.increment_run_counter(RUN_COUNTER_CLICK_SUCCESS, 1);
                        self.post_interaction_pause_with_budget(timing_profile.post_click_pause_ms)
                            .await;
                        return Ok(fallback_outcome);
                    }
                    Err(err) => {
                        return Err(anyhow::anyhow!(
                            "[task-api] strict click verification failed for '{}': {}",
                            selector,
                            err
                        ));
                    }
                }
            }
        }

        {
            if let Err(persist_err) = self.record_click_learning(selector, true).await {
                warn!(
                    "[task-api] click learning persistence failed: {}",
                    persist_err
                );
            }
        }
        self.increment_run_counter(RUN_COUNTER_CLICK_SUCCESS, 1);

        info!(
            "[task-api] click '{}' tuned delay={}ms variance={} fatigue={:?} recent_success={:.2}",
            selector,
            timing_profile.reaction_delay_ms,
            timing_profile.reaction_variance_pct,
            fatigue,
            recent_success_rate
        );

        self.post_interaction_pause_with_budget(timing_profile.post_click_pause_ms)
            .await;
        Ok(outcome)
    }

    async fn execute_primary_click_attempt(
        &self,
        selector: &str,
        reaction_delay_ms: u64,
        reaction_delay_variance_pct: u32,
        click_offset_px: i32,
        timeout_ms: u64,
    ) -> Result<ClickOutcome> {
        match tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            mouse::click_selector_human(
                self.page(),
                selector,
                reaction_delay_ms,
                reaction_delay_variance_pct,
                click_offset_px,
            ),
        )
        .await
        {
            Ok(Ok(outcome)) => Ok(outcome),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(anyhow::anyhow!(
                "[task-api] primary click attempt timed out for '{}'",
                selector
            )),
        }
    }

    async fn fallback_click_with_adaptation(
        &self,
        selector: &str,
        adaptation: &ClickAdaptation,
    ) -> Result<ClickOutcome> {
        const FALLBACK_FOCUS_TIMEOUT_SECS: u64 = 2;
        const FALLBACK_CLICK_TIMEOUT_SECS: u64 = 2;

        if adaptation.extra_stability_wait_ms > 0 {
            timing::uniform_pause(adaptation.extra_stability_wait_ms.min(700), 25).await;
        }

        info!("[task-api] click fallback '{}': focus begin", selector);
        let focus = match tokio::time::timeout(
            Duration::from_secs(FALLBACK_FOCUS_TIMEOUT_SECS),
            self.focus(selector),
        )
        .await
        {
            Ok(Ok(focus)) => focus,
            Ok(Err(err)) => {
                return Err(anyhow::anyhow!(
                    "[task-api] fallback focus failed for '{}': {}",
                    selector,
                    err
                ));
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "[task-api] fallback focus timed out for '{}'",
                    selector
                ));
            }
        };
        info!(
            "[task-api] click fallback '{}': focus ok at ({:.1},{:.1})",
            selector, focus.x, focus.y
        );

        info!("[task-api] click fallback '{}': click_at begin", selector);
        match tokio::time::timeout(
            Duration::from_secs(FALLBACK_CLICK_TIMEOUT_SECS),
            self.click_at(focus.x, focus.y),
        )
        .await
        {
            Ok(Ok(())) => {
                info!("[task-api] click fallback '{}': click_at ok", selector);
                let verified = self.verify_selector_hit(selector, focus.x, focus.y).await?;
                if adaptation.require_strict_verification && !verified {
                    return Err(anyhow::anyhow!(
                        "[task-api] fallback click target verification failed for '{}'",
                        selector
                    ));
                }
                if !verified {
                    warn!(
                        "[task-api] fallback click verification inconclusive for '{}'",
                        selector
                    );
                }
                Ok(ClickOutcome {
                    click: crate::utils::mouse::ClickStatus::Success,
                    x: focus.x,
                    y: focus.y,
                    screen_x: None,
                    screen_y: None,
                })
            }
            Ok(Err(err)) => Err(anyhow::anyhow!(
                "[task-api] fallback click_at failed for '{}': {}",
                selector,
                err
            )),
            Err(_) => Err(anyhow::anyhow!(
                "[task-api] fallback click_at timed out for '{}'",
                selector
            )),
        }
    }

    /// Click selector, then wait for next selector to become visible within timeout.
    pub async fn click_and_wait(
        &self,
        selector: &str,
        next_selector: &str,
        timeout_ms: u64,
    ) -> Result<ClickAndWaitOutcome> {
        let click = self.click(selector).await?;
        let next_visible =
            self.wait_for_visible(next_selector, timeout_ms)
                .await
                .map(|visible| {
                    if visible {
                        WaitForVisibleStatus::Visible
                    } else {
                        WaitForVisibleStatus::Timeout
                    }
                })?;
        Ok(ClickAndWaitOutcome {
            click,
            next_selector: next_selector.to_string(),
            next_visible,
            timeout_ms,
        })
    }

    /// Human-like double click on selector with delay and variance.
    pub async fn double_click(&self, selector: &str) -> Result<ClickOutcome> {
        let click = &self.behavior_runtime.click;
        let outcome = mouse::double_click_selector_human(
            self.page(),
            selector,
            click.reaction_delay_ms,
            self.behavior_runtime.action_delay.variance_pct.round() as u32,
            click.offset_px,
        )
        .await?;
        self.post_interaction_pause().await;
        Ok(outcome)
    }

    /// Middle-click (mouse wheel) on selector with human-like behavior.
    pub async fn middle_click(&self, selector: &str) -> Result<ClickOutcome> {
        let click = &self.behavior_runtime.click;
        let outcome = mouse::middle_click_selector_human(
            self.page(),
            selector,
            click.reaction_delay_ms,
            self.behavior_runtime.action_delay.variance_pct.round() as u32,
            click.offset_px,
        )
        .await?;
        self.post_interaction_pause().await;
        Ok(outcome)
    }

    /// Left-click at absolute coordinates with cursor animation.
    pub async fn left_click(&self, x: f64, y: f64) -> Result<()> {
        mouse::left_click_at(self.page(), x, y).await
    }

    /// Native OS-level click pipeline:
    /// 1) human-like scroll to selector,
    /// 2) native move + click via backend,
    /// 3) public task log with clicked selector and point.
    pub async fn nativeclick(&self, selector: &str) -> Result<ClickOutcome> {
        let session_id = self.session_id().to_string();
        let click = &self.behavior_runtime.click;
        let outcome = match mouse::native_click_selector_human(
            self.page(),
            &session_id,
            selector,
            click.reaction_delay_ms,
            self.behavior_runtime.action_delay.variance_pct.round() as u32,
            click.offset_px,
            self.native_interaction(),
        )
        .await
        {
            Ok(outcome) => outcome,
            Err(err) => {
                let mut ctx = crate::logger::get_log_context();
                ctx.session_id = Some(session_id.clone());
                let _guard = scoped_log_context(ctx);
                warn!(
                    "[task-api] nativeclick failed selector={} error={}",
                    selector, err
                );
                return Err(err);
            }
        };
        {
            let mut ctx = crate::logger::get_log_context();
            ctx.session_id = Some(session_id.clone());
            let _guard = scoped_log_context(ctx);
            info!(
                "{}",
                nativeclick_public_log_line(selector, outcome.x, outcome.y)
            );
            if let (Some(screen_x), Some(screen_y)) = (outcome.screen_x, outcome.screen_y) {
                debug!(
                    "[task-api] nativeclick session={} selector={} screen_point=({}, {})",
                    session_id, selector, screen_x, screen_y
                );
            } else {
                debug!(
                    "[task-api] nativeclick session={} selector={} screen_point=(unknown)",
                    session_id, selector
                );
            }
            debug!(
                "[task-api] nativeclick session={} selector={} summary={}",
                session_id,
                selector,
                outcome.summary()
            );
        }
        self.post_interaction_pause().await;
        Ok(outcome)
    }

    /// Native OS-level cursor move to any random visible element on the current page.
    pub async fn nativecursor(&self) -> Result<NativeCursorOutcome> {
        self.execute_nativecursor(None).await
    }

    /// Native OS-level cursor move to a random visible element matching the query.
    pub async fn nativecursor_query(&self, query: &str) -> Result<NativeCursorOutcome> {
        self.execute_nativecursor(Some(query)).await
    }

    /// Alias for selector-driven native cursor movement.
    pub async fn nativecursor_selector(&self, selector: &str) -> Result<NativeCursorOutcome> {
        self.execute_nativecursor(Some(selector)).await
    }

    /// Immediate left-click at coordinates without cursor animation.
    pub async fn left_click_fast(&self, x: f64, y: f64) -> Result<()> {
        mouse::left_click_at_without_move(self.page(), x, y).await
    }

    /// Right-click context menu at absolute coordinates.
    pub async fn right_click_at(&self, x: f64, y: f64) -> Result<()> {
        mouse::right_click_at(self.page(), x, y).await
    }

    /// Immediate right-click at coordinates without cursor animation.
    pub async fn right_click_fast(&self, x: f64, y: f64) -> Result<()> {
        mouse::right_click_at_without_move(self.page(), x, y).await
    }

    /// Human-like right-click (context menu) on selector.
    pub async fn right_click(&self, selector: &str) -> Result<ClickOutcome> {
        let click = &self.behavior_runtime.click;
        let outcome = mouse::right_click_selector_human(
            self.page(),
            selector,
            click.reaction_delay_ms,
            self.behavior_runtime.action_delay.variance_pct.round() as u32,
            click.offset_px,
        )
        .await?;
        self.post_interaction_pause().await;
        Ok(outcome)
    }

    /// Drag from one selector to another with human-like behavior.
    pub async fn drag(&self, from_selector: &str, to_selector: &str) -> Result<()> {
        let click = &self.behavior_runtime.click;
        mouse::drag_selector_to_selector(
            self.page(),
            from_selector,
            to_selector,
            click.reaction_delay_ms,
            self.behavior_runtime.action_delay.variance_pct.round() as u32,
        )
        .await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    /// Press a single key (e.g., "Enter", "Tab", "Escape").
    pub async fn press(&self, key: &str) -> Result<()> {
        keyboard::press(self.page(), key).await
    }

    /// Press key with modifiers (e.g., Ctrl+C, Shift+A).
    pub async fn press_with_modifiers(&self, key: &str, modifiers: &[&str]) -> Result<()> {
        keyboard::press_with_modifiers(self.page(), key, modifiers).await
    }

    /// Types text into a focused element with human-like keystroke timing.
    ///
    /// This method:
    /// - Focuses the element first
    /// - Types text with realistic keystroke delays
    /// - Uses the configured typing profile
    /// - Includes post-interaction pause
    ///
    /// # Arguments
    ///
    /// * `selector` - CSS selector for the element to type into
    /// * `text` - The text to type
    ///
    /// # Errors
    ///
    /// Returns an error if the element cannot be found or focused.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::runtime::task_context::TaskContext;
    /// # async fn example(api: &TaskContext) -> anyhow::Result<()> {
    /// api.r#type("#input-field", "Hello World").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn r#type(&self, selector: &str, text: &str) -> Result<()> {
        info!("[task-api] keyboard {} -> {}", selector, text);
        let _ = self.focus(selector).await?;
        let typing = &self.behavior_runtime.typing;
        keyboard::type_text_profiled(self.page(), text, typing).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    /// Types text into an element. Alias for `r#type()`.
    ///
    /// This is the preferred method name for typing text, as it's more readable
    /// than the Rust-keyword-safe `r#type()` alias.
    ///
    /// # Arguments
    ///
    /// * `selector` - CSS selector for the element to type into
    /// * `text` - The text to type
    ///
    /// # Errors
    ///
    /// Returns an error if the element cannot be found or focused.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::runtime::task_context::TaskContext;
    /// # async fn example(api: &TaskContext) -> anyhow::Result<()> {
    /// api.keyboard("#input-field", "Hello World").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn keyboard(&self, selector: &str, text: &str) -> Result<()> {
        self.r#type(selector, text).await
    }

    /// Type text into selector. Alias for `keyboard()`.
    pub async fn type_into(&self, selector: &str, text: &str) -> Result<()> {
        self.r#type(selector, text).await
    }

    /// Type text directly without focusing. Applies to currently focused element.
    pub async fn type_text(&self, text: &str) -> Result<()> {
        let typing = &self.behavior_runtime.typing;
        keyboard::type_text_profiled(self.page(), text, typing).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    /// Scroll in a random direction by random amount.
    pub async fn random_scroll(&self) -> Result<()> {
        scroll::random_scroll(self.page()).await
    }

    /// Scroll selector into view with post-scroll pause.
    pub async fn scroll_to(&self, selector: &str) -> Result<()> {
        scroll::scroll_into_view(self.page(), selector).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    /// Scroll through page content with pauses for reading. Params: pause count, scroll px, variable speed, scroll back after.
    pub async fn scroll_read(
        &self,
        pauses: u32,
        scroll_amount: i32,
        variable_speed: bool,
        back_scroll: bool,
    ) -> Result<()> {
        scroll::read(
            self.page(),
            pauses,
            scroll_amount,
            variable_speed,
            back_scroll,
        )
        .await
    }

    /// Scroll through page content for a specified duration (ms). Automatically calculates pause count.
    pub async fn scrollread(&self, duration_ms: u64) -> Result<()> {
        scroll::read_by_duration(self.page(), duration_ms).await
    }

    /// Scroll to selector, then read with pauses. Params: selector, pause count, scroll px, variable speed, scroll back after.
    pub async fn scroll_read_to(
        &self,
        selector: &str,
        pauses: u32,
        scroll_amount: i32,
        variable_speed: bool,
        back_scroll: bool,
    ) -> Result<()> {
        scroll::scroll_read_to(
            self.page(),
            selector,
            pauses,
            scroll_amount,
            variable_speed,
            back_scroll,
        )
        .await
    }

    /// Scroll back by distance in pixels (negative goes forward).
    pub async fn scroll_back(&self, distance: i32) -> Result<()> {
        scroll::back(self.page(), distance).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    /// Scroll selector into view. Alias for `scroll_to()`.
    pub async fn scroll_into_view(&self, selector: &str) -> Result<()> {
        self.scroll_to(selector).await
    }

    /// Scroll to top of page (y=0).
    pub async fn scroll_to_top(&self) -> Result<()> {
        scroll::scroll_to_top(self.page()).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    /// Scroll to bottom of page (max scroll).
    pub async fn scroll_to_bottom(&self) -> Result<()> {
        scroll::scroll_to_bottom(self.page()).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    /// Select all + copy to clipboard. Returns clipboard content.
    pub async fn copy(&self) -> Result<String> {
        clipboard::copy(self.session_id(), self.page()).await
    }

    /// Select all + cut to clipboard. Returns cut content.
    pub async fn cut(&self) -> Result<String> {
        clipboard::cut(self.session_id(), self.page()).await
    }

    /// Paste clipboard content into focused element. Returns pasted content.
    pub async fn paste(&self) -> Result<String> {
        clipboard::paste_from_clipboard(self.session_id(), self.page()).await
    }

    /// Wait for base_ms with 20% variance (uniform distribution).
    pub async fn pause(&self, base_ms: u64) {
        timing::uniform_pause(base_ms, 20).await;
    }

    /// Wait with custom variance percentage (e.g., 20 for 20%).
    pub async fn pause_with_variance(&self, base_ms: u64, variance_pct: u32) {
        timing::human_pause(base_ms, variance_pct).await;
    }

    /// Check if selector exists in DOM (may be hidden).
    pub async fn exists(&self, selector: &str) -> Result<bool> {
        navigation::selector_exists(self.page(), selector).await
    }

    /// Check if selector is visible (displayed and not hidden).
    pub async fn visible(&self, selector: &str) -> Result<bool> {
        navigation::selector_is_visible(self.page(), selector).await
    }

    /// Get text content of selector. Returns None if not found.
    pub async fn text(&self, selector: &str) -> Result<Option<String>> {
        navigation::selector_text(self.page(), selector).await
    }

    /// Get inner HTML of selector. Returns None if not found.
    pub async fn html(&self, selector: &str) -> Result<Option<String>> {
        navigation::selector_html(self.page(), selector).await
    }

    /// Get element attribute by name. Returns None if not found.
    pub async fn attr(&self, selector: &str, name: &str) -> Result<Option<String>> {
        navigation::selector_attr(self.page(), selector, name).await
    }

    /// Get input/textarea value attribute. Returns None if not found.
    pub async fn value(&self, selector: &str) -> Result<Option<String>> {
        navigation::selector_value(self.page(), selector).await
    }

    /// Wait for selector to exist in DOM. Returns true if found within timeout.
    pub async fn wait_for(&self, selector: &str, timeout_ms: u64) -> Result<bool> {
        navigation::wait_for_selector(self.page(), selector, timeout_ms).await
    }

    /// Wait for selector to be visible. Returns true if visible within timeout.
    pub async fn wait_for_visible(&self, selector: &str, timeout_ms: u64) -> Result<bool> {
        navigation::wait_for_visible_selector(self.page(), selector, timeout_ms).await
    }

    /// Get current page URL.
    pub async fn url(&self) -> Result<String> {
        navigation::page_url(self.page()).await
    }

    /// Get page title from DOM.
    pub async fn title(&self) -> Result<String> {
        navigation::page_title(self.page()).await
    }

    /// Get viewport dimensions (width, height, device_scale_factor).
    pub async fn viewport(&self) -> Result<Viewport> {
        page_size::get_viewport(self.page()).await
    }

    /// Select all text in element (Ctrl+A).
    pub async fn select_all(&self, selector: &str) -> Result<()> {
        let _ = self.focus(selector).await?;
        self.press_with_modifiers("a", &["Control"]).await
    }

    /// Clear input by selecting all + pressing Backspace.
    pub async fn clear(&self, selector: &str) -> Result<()> {
        self.select_all(selector).await?;
        self.press("Backspace").await
    }

    async fn verify_selector_hit(&self, selector: &str, x: f64, y: f64) -> Result<bool> {
        let selector_js = serde_json::to_string(selector)?;
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({selector_js});
                if (!el) return false;
                const rect = el.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) return false;
                const hit = document.elementFromPoint({x}, {y});
                if (!hit) return false;
                return el === hit || el.contains(hit) || hit.contains(el);
            }})()"#
        );
        let eval = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            self.page.evaluate(js),
        )
        .await
        .map_err(|_| anyhow::anyhow!("fallback click verification timeout"))??;
        Ok(eval.value().and_then(|v| v.as_bool()).unwrap_or(false))
    }

    async fn post_interaction_pause(&self) {
        self.post_interaction_pause_with_budget(0).await;
    }

    async fn post_interaction_pause_with_budget(&self, min_budget_ms: u64) {
        let action_delay = &self.behavior_runtime.action_delay;
        let base_ms = action_delay.min_ms.clamp(120, 1_500).max(min_budget_ms);
        let variance_pct = action_delay.variance_pct.round().clamp(10.0, 60.0) as u32;
        timing::uniform_pause(base_ms, variance_pct).await;
    }

    async fn execute_nativecursor(&self, query: Option<&str>) -> Result<NativeCursorOutcome> {
        let session_id = self.session_id().to_string();
        let click = &self.behavior_runtime.click;
        let outcome = mouse::native_move_cursor_human(
            self.page(),
            &session_id,
            query,
            click.reaction_delay_ms,
            self.behavior_runtime.action_delay.variance_pct.round() as u32,
            self.native_interaction(),
        )
        .await?;
        {
            let mut ctx = crate::logger::get_log_context();
            ctx.session_id = Some(session_id.clone());
            let _guard = scoped_log_context(ctx);
            let screen_point = match (outcome.screen_x, outcome.screen_y) {
                (Some(x), Some(y)) => format!("({}, {})", x, y),
                _ => "unknown".to_string(),
            };
            info!(
                "[task-api] t={} ({:.1},{:.1}) p=({:.1},{:.1}) s={}",
                outcome.target, outcome.x, outcome.y, outcome.x, outcome.y, screen_point
            );
        }
        self.post_interaction_pause().await;
        Ok(outcome)
    }
}
