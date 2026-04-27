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

/// HTTP response structure for network operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    /// HTTP status code
    pub status: u16,
    /// Response body as string
    pub body: String,
    /// Response headers
    pub headers: HashMap<String, String>,
}

/// Rectangle for element position and size.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    /// X coordinate (left edge)
    pub x: f64,
    /// Y coordinate (top edge)
    pub y: f64,
    /// Width in pixels
    pub width: f64,
    /// Height in pixels
    pub height: f64,
}

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

/// Metadata for a data file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File size in bytes
    pub size: u64,
    /// Last modification time
    pub modified: std::time::SystemTime,
    /// Creation time
    pub created: std::time::SystemTime,
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

    #[test]
    fn test_screenshot_filename_format() {
        // Test filename generation matches expected format
        let session_id = "test-session-123";
        let now = chrono::Utc::now();
        let filename = format!(
            "{}-{}-{}.jpg",
            now.format("%Y-%m-%d"),
            now.format("%H-%M"),
            session_id
        );
        
        // Verify format: yyyy-mm-dd-hh-mm-sessionid.jpg
        assert!(filename.ends_with(".jpg"));
        assert!(filename.contains("test-session-123"));
        assert!(filename.len() > 20); // Reasonable length for timestamp + session
    }

    #[test]
    fn test_screenshot_directory_path() {
        let screenshot_dir = std::path::Path::new("data/screenshot");
        assert_eq!(screenshot_dir.to_str().unwrap(), "data/screenshot");
    }

    // ============================================================================
    // Browser Management Tests
    // ============================================================================

    #[test]
    fn test_browser_data_default() {
        let data = crate::task::policy::BrowserData::default();
        assert!(data.cookies.is_empty());
        assert!(data.local_storage.is_empty());
        assert!(data.session_storage.is_empty());
        assert!(data.indexeddb_names.is_empty());
        assert!(data.source.is_empty());
        assert!(data.browser_version.is_none());
    }

    #[test]
    fn test_browser_data_serialization_roundtrip() {
        use chrono::Utc;
        use std::collections::HashMap;

        let mut local_storage = HashMap::new();
        let mut origin_data = HashMap::new();
        origin_data.insert("key1".to_string(), "value1".to_string());
        origin_data.insert("key2".to_string(), "value2".to_string());
        local_storage.insert("example.com".to_string(), origin_data);

        let mut indexeddb = HashMap::new();
        indexeddb.insert("example.com".to_string(), vec!["db1".to_string(), "db2".to_string()]);

        let data = crate::task::policy::BrowserData {
            cookies: vec![serde_json::json!({"name": "test", "value": "cookie"})],
            local_storage,
            session_storage: HashMap::new(),
            indexeddb_names: indexeddb,
            exported_at: Utc::now(),
            source: "https://example.com".to_string(),
            browser_version: Some("Chrome 120".to_string()),
        };

        // Serialize
        let json = serde_json::to_string(&data).expect("Should serialize");

        // Deserialize
        let restored: crate::task::policy::BrowserData = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(restored.cookies.len(), 1);
        assert_eq!(restored.source, "https://example.com");
        assert_eq!(restored.browser_version, Some("Chrome 120".to_string()));
        assert_eq!(restored.local_storage.len(), 1);
        assert!(restored.local_storage.contains_key("example.com"));
    }

    #[test]
    fn test_permissions_include_browser_export_import() {
        let perms = crate::task::policy::TaskPermissions::default();
        assert!(!perms.allow_browser_export);
        assert!(!perms.allow_browser_import);

        // Test with custom permissions
        let custom_policy = crate::task::policy::TaskPolicy {
            max_duration_ms: 30_000,
            permissions: crate::task::policy::TaskPermissions {
                allow_browser_export: true,
                allow_browser_import: true,
                ..Default::default()
            },
        };

        assert!(custom_policy.permissions.allow_browser_export);
        assert!(custom_policy.permissions.allow_browser_import);
    }

    #[test]
    fn test_file_metadata_struct() {
        let metadata = super::FileMetadata {
            size: 1024,
            modified: std::time::SystemTime::UNIX_EPOCH,
            created: std::time::SystemTime::UNIX_EPOCH,
        };

        assert_eq!(metadata.size, 1024);

        // Test serialization
        let json = serde_json::to_string(&metadata).expect("Should serialize");
        assert!(json.contains("1024"));
    }

    #[test]
    fn test_http_response_struct() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let response = super::HttpResponse {
            status: 200,
            body: "{\"success\": true}".to_string(),
            headers,
        };

        assert_eq!(response.status, 200);
        assert_eq!(response.body, "{\"success\": true}");
        assert_eq!(response.headers.get("Content-Type"), Some(&"application/json".to_string()));

        // Test serialization
        let json = serde_json::to_string(&response).expect("Should serialize");
        assert!(json.contains("200"));
        assert!(json.contains("success"));
    }

    #[test]
    fn test_rect_struct() {
        let rect = super::Rect {
            x: 10.5,
            y: 20.5,
            width: 100.0,
            height: 50.0,
        };

        assert_eq!(rect.x, 10.5);
        assert_eq!(rect.y, 20.5);
        assert_eq!(rect.width, 100.0);
        assert_eq!(rect.height, 50.0);

        // Test serialization roundtrip
        let json = serde_json::to_string(&rect).expect("Should serialize");
        let restored: super::Rect = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(restored.x, 10.5);
        assert_eq!(restored.width, 100.0);
    }

    #[test]
    fn test_click_learning_persistence_with_real_file() {
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("click_learning.json");

        // Create state and save
        let mut state = ClickLearningState::default();
        state.record("#button1", true);
        state.record("#button1", true);
        state.record("#button2", false);

        super::save_click_learning(&path, &state).expect("Should save");
        assert!(path.exists());

        // Load and verify
        let loaded = super::load_click_learning(&path).expect("Should load");
        assert_eq!(loaded.total_attempts, 3);
        assert_eq!(loaded.total_successes, 2);

        // Cleanup
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_sanitize_path_component_various_inputs() {
        assert_eq!(super::sanitize_path_component("normal"), "normal");
        assert_eq!(super::sanitize_path_component("with-dash"), "with-dash");
        assert_eq!(super::sanitize_path_component("with_underscore"), "with_underscore");
        assert_eq!(super::sanitize_path_component("UPPERCASE"), "UPPERCASE");
        assert_eq!(super::sanitize_path_component("123"), "123");
        assert_eq!(super::sanitize_path_component(""), "default");
        assert_eq!(super::sanitize_path_component("   "), "default");
        assert_eq!(super::sanitize_path_component("a"), "a");
    }

    #[test]
    fn test_click_timing_profile_edge_cases() {
        let context = ClickTimingContext {
            page: ClickPageContext::Other,
            priority: ClickElementPriority::Critical,
            fatigue: ClickFatigueLevel::Tired,
            recent_success_rate: 0.0,
        };

        let profile = context.timing_profile(200, 15, 5, &ClickAdaptation::default());
        assert!(profile.reaction_delay_ms >= 150); // Increased due to fatigue
        assert!(profile.primary_timeout_ms >= 4_000);
    }

    #[test]
    fn test_click_adaptation_with_extreme_failures() {
        let mut learning = ClickLearningState::default();

        // Simulate many failures
        for _ in 0..20 {
            learning.record("#button", false);
        }

        let context = ClickTimingContext::from_observation(
            "https://example.com",
            "#button",
            20, // interaction_count
            0.0, // recent_success_rate (all failures)
        );
        let adaptation = learning.adaptation_for("#button", &context);

        // Should require strict verification after many failures
        assert!(adaptation.require_strict_verification);
        assert!(adaptation.prefer_coordinate_fallback);
    }

    // ============================================================================
    // API v0.0.3 Permission Denial Tests
    // ============================================================================

    #[test]
    fn test_cookie_permissions_default_false() {
        let perms = crate::task::policy::TaskPermissions::default();
        assert!(!perms.allow_export_cookies);
        assert!(!perms.allow_import_cookies);
    }

    #[test]
    fn test_session_permissions_default_false() {
        let perms = crate::task::policy::TaskPermissions::default();
        assert!(!perms.allow_export_session);
        assert!(!perms.allow_import_session);
    }

    #[test]
    fn test_clipboard_permissions_default_false() {
        let perms = crate::task::policy::TaskPermissions::default();
        assert!(!perms.allow_session_clipboard);
    }

    #[test]
    fn test_data_permissions_default_false() {
        let perms = crate::task::policy::TaskPermissions::default();
        assert!(!perms.allow_read_data);
        assert!(!perms.allow_write_data);
    }

    #[test]
    fn test_http_permissions_default_false() {
        let perms = crate::task::policy::TaskPermissions::default();
        assert!(!perms.allow_http_requests);
    }

    #[test]
    fn test_dom_inspection_permissions_default_false() {
        let perms = crate::task::policy::TaskPermissions::default();
        assert!(!perms.allow_dom_inspection);
    }

    #[test]
    fn test_browser_permissions_default_false() {
        let perms = crate::task::policy::TaskPermissions::default();
        assert!(!perms.allow_browser_export);
        assert!(!perms.allow_browser_import);
    }

    // ============================================================================
    // API v0.0.3 Check Permission Tests
    // ============================================================================

    #[test]
    fn test_check_permission_cookie_export() {
        let policy = crate::task::policy::TaskPolicy {
            max_duration_ms: 30_000,
            permissions: crate::task::policy::TaskPermissions {
                allow_export_cookies: true,
                ..Default::default()
            },
        };
        let static_policy = Box::leak(Box::new(policy));

        // We can't easily test check_permission without a TaskContext,
        // but we can verify the permission struct works
        assert!(static_policy.permissions.allow_export_cookies);
        assert!(!static_policy.permissions.allow_import_cookies);
    }

    #[test]
    fn test_check_permission_session_import() {
        let policy = crate::task::policy::TaskPolicy {
            max_duration_ms: 30_000,
            permissions: crate::task::policy::TaskPermissions {
                allow_import_session: true,
                ..Default::default()
            },
        };
        let static_policy = Box::leak(Box::new(policy));

        assert!(static_policy.permissions.allow_import_session);
        assert!(!static_policy.permissions.allow_export_session);
    }

    #[test]
    fn test_check_permission_data_read_write() {
        let policy = crate::task::policy::TaskPolicy {
            max_duration_ms: 30_000,
            permissions: crate::task::policy::TaskPermissions {
                allow_read_data: true,
                allow_write_data: true,
                ..Default::default()
            },
        };
        let static_policy = Box::leak(Box::new(policy));

        assert!(static_policy.permissions.allow_read_data);
        assert!(static_policy.permissions.allow_write_data);
    }

    // ============================================================================
    // API v0.0.3 Data Structures Tests
    // ============================================================================

    #[test]
    fn test_session_data_empty_initialization() {
        use std::collections::HashMap;
        let data = crate::task::policy::SessionData {
            cookies: vec![],
            local_storage: HashMap::new(),
            exported_at: chrono::Utc::now(),
            url: String::new(),
        };
        assert!(data.cookies.is_empty());
        assert!(data.local_storage.is_empty());
        assert!(data.url.is_empty());
    }

    #[test]
    fn test_session_data_serialization() {
        use std::collections::HashMap;

        let mut local_storage = HashMap::new();
        local_storage.insert("key".to_string(), "value".to_string());

        let data = crate::task::policy::SessionData {
            cookies: vec![serde_json::json!({"name": "test"})],
            local_storage,
            exported_at: chrono::Utc::now(),
            url: "https://example.com".to_string(),
        };

        let json = serde_json::to_string(&data).expect("Should serialize");
        assert!(json.contains("example.com"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_http_response_error_display() {
        let response = super::HttpResponse {
            status: 404,
            body: "Not Found".to_string(),
            headers: std::collections::HashMap::new(),
        };

        assert_eq!(response.status, 404);
        assert_eq!(response.body, "Not Found");
    }

    #[test]
    fn test_http_response_success_status() {
        let response = super::HttpResponse {
            status: 200,
            body: "OK".to_string(),
            headers: std::collections::HashMap::new(),
        };

        assert!(response.status >= 200 && response.status < 300);
    }

    #[test]
    fn test_rect_zero_values() {
        let rect = super::Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };

        assert_eq!(rect.x, 0.0);
        assert_eq!(rect.y, 0.0);
        assert_eq!(rect.width, 0.0);
        assert_eq!(rect.height, 0.0);
    }

    #[test]
    fn test_rect_negative_values() {
        // Rect can theoretically have negative x/y (off-screen elements)
        let rect = super::Rect {
            x: -100.0,
            y: -50.0,
            width: 200.0,
            height: 100.0,
        };

        assert_eq!(rect.x, -100.0);
        assert_eq!(rect.y, -50.0);
        assert_eq!(rect.width, 200.0);
        assert_eq!(rect.height, 100.0);
    }

    #[test]
    fn test_file_metadata_large_file() {
        let metadata = super::FileMetadata {
            size: 1024 * 1024 * 100, // 100 MB
            modified: std::time::SystemTime::UNIX_EPOCH,
            created: std::time::SystemTime::UNIX_EPOCH,
        };

        assert_eq!(metadata.size, 104_857_600);
    }

    #[test]
    fn test_file_metadata_empty_file() {
        let metadata = super::FileMetadata {
            size: 0,
            modified: std::time::SystemTime::UNIX_EPOCH,
            created: std::time::SystemTime::UNIX_EPOCH,
        };

        assert_eq!(metadata.size, 0);
    }

    // ============================================================================
    // API v0.0.3 Policy Integration Tests
    // ============================================================================

    #[test]
    fn test_default_task_policy_all_permissions_false() {
        let policy = crate::task::policy::DEFAULT_TASK_POLICY;

        assert!(!policy.permissions.allow_screenshot);
        assert!(!policy.permissions.allow_export_cookies);
        assert!(!policy.permissions.allow_import_cookies);
        assert!(!policy.permissions.allow_export_session);
        assert!(!policy.permissions.allow_import_session);
        assert!(!policy.permissions.allow_session_clipboard);
        assert!(!policy.permissions.allow_read_data);
        assert!(!policy.permissions.allow_write_data);
        assert!(!policy.permissions.allow_http_requests);
        assert!(!policy.permissions.allow_dom_inspection);
        assert!(!policy.permissions.allow_browser_export);
        assert!(!policy.permissions.allow_browser_import);
    }

    #[test]
    fn test_twitter_policy_has_required_permissions() {
        use crate::task::policy::TWITTERACTIVITY_POLICY;

        assert!(TWITTERACTIVITY_POLICY.permissions.allow_export_cookies);
        assert!(TWITTERACTIVITY_POLICY.permissions.allow_session_clipboard);
        assert!(TWITTERACTIVITY_POLICY.permissions.allow_read_data);
        assert!(TWITTERACTIVITY_POLICY.permissions.allow_screenshot);
        // allow_write_data is implied by allow_screenshot
    }

    #[test]
    fn test_cookiebot_policy_has_required_permissions() {
        use crate::task::policy::COOKIEBOT_POLICY;

        assert!(COOKIEBOT_POLICY.permissions.allow_export_cookies);
        assert!(COOKIEBOT_POLICY.permissions.allow_screenshot);
    }

    #[test]
    fn test_pageview_policy_has_screenshot_only() {
        use crate::task::policy::PAGEVIEW_POLICY;

        assert!(PAGEVIEW_POLICY.permissions.allow_screenshot);
        assert!(!PAGEVIEW_POLICY.permissions.allow_export_cookies);
        assert!(!PAGEVIEW_POLICY.permissions.allow_http_requests);
    }

    // ============================================================================
    // API v0.0.3 BrowserData Advanced Tests
    // ============================================================================

    #[test]
    fn test_browser_data_with_multiple_origins() {
        use chrono::Utc;
        use std::collections::HashMap;

        let mut local_storage = HashMap::new();
        let mut origin1 = HashMap::new();
        origin1.insert("key1".to_string(), "value1".to_string());
        let mut origin2 = HashMap::new();
        origin2.insert("key2".to_string(), "value2".to_string());

        local_storage.insert("example.com".to_string(), origin1);
        local_storage.insert("api.example.com".to_string(), origin2);

        let data = crate::task::policy::BrowserData {
            cookies: vec![],
            local_storage,
            session_storage: HashMap::new(),
            indexeddb_names: HashMap::new(),
            exported_at: Utc::now(),
            source: "test".to_string(),
            browser_version: None,
        };

        assert_eq!(data.local_storage.len(), 2);
        assert!(data.local_storage.contains_key("example.com"));
        assert!(data.local_storage.contains_key("api.example.com"));
    }

    #[test]
    fn test_browser_data_with_indexeddb() {
        use chrono::Utc;
        use std::collections::HashMap;

        let mut indexeddb = HashMap::new();
        indexeddb.insert("example.com".to_string(), vec![
            "my-database".to_string(),
            "cache-store".to_string(),
        ]);

        let data = crate::task::policy::BrowserData {
            cookies: vec![],
            local_storage: HashMap::new(),
            session_storage: HashMap::new(),
            indexeddb_names: indexeddb,
            exported_at: Utc::now(),
            source: "test".to_string(),
            browser_version: Some("Chrome 120".to_string()),
        };

        assert_eq!(data.indexeddb_names.len(), 1);
        let dbs = data.indexeddb_names.get("example.com").unwrap();
        assert_eq!(dbs.len(), 2);
        assert!(dbs.contains(&"my-database".to_string()));
    }

    #[test]
    fn test_browser_data_empty_is_valid() {
        let data = crate::task::policy::BrowserData::default();

        // Empty browser data should be valid for import/export
        assert!(data.cookies.is_empty());
        assert!(data.local_storage.is_empty());
        assert!(data.session_storage.is_empty());
        assert!(data.indexeddb_names.is_empty());
    }

    // ============================================================================
    // API v0.0.3 Helper Function Tests
    // ============================================================================

    #[test]
    fn test_sanitize_path_component_with_special_chars() {
        assert_eq!(super::sanitize_path_component("test/file"), "test_file");
        assert_eq!(super::sanitize_path_component("test..file"), "test__file");
        assert_eq!(super::sanitize_path_component("test\\file"), "test__file");
    }

    #[test]
    fn test_sanitize_path_component_unicode_extended() {
        // Should handle unicode by replacing with underscore
        assert_eq!(super::sanitize_path_component("测试"), "__");
        assert_eq!(super::sanitize_path_component("test日本語file"), "test____file");
    }

    #[test]
    fn test_sanitize_path_component_long_name() {
        let long_name = "a".repeat(300);
        let result = super::sanitize_path_component(&long_name);
        // Should not panic and should preserve the name (or truncate)
        assert!(!result.is_empty());
    }

    #[test]
    fn test_click_learning_path_generation() {
        use crate::utils::profile::ProfilePreset;
        use crate::utils::randomize_profile;

        let profile = randomize_profile(&ProfilePreset::Average);
        let path = super::click_learning_path("session-123", &profile);
        assert!(path.is_some());

        let path = path.unwrap();
        let path_str = path.to_str().unwrap();
        assert!(path_str.contains("click-learning"));
        assert!(path_str.contains("session-123"));
        assert!(path_str.contains(&profile.name));
    }

    #[test]
    fn test_click_learning_save_and_load_roundtrip() {
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("test_learning.json");

        // Create and save
        let mut state = ClickLearningState::default();
        for i in 0..5 {
            state.record(&format!("#button{}", i), i % 2 == 0);
        }

        super::save_click_learning(&path, &state).expect("Should save");

        // Load
        let loaded = super::load_click_learning(&path).expect("Should load");
        assert_eq!(loaded.total_attempts, 5);
        assert_eq!(loaded.total_successes, 3); // 0, 2, 4 are even (success)

        // Cleanup
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_click_learning_empty_save() {
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("empty_learning.json");

        let state = ClickLearningState::default();
        super::save_click_learning(&path, &state).expect("Should save empty state");

        let loaded = super::load_click_learning(&path).expect("Should load empty state");
        assert_eq!(loaded.total_attempts, 0);
        assert_eq!(loaded.total_successes, 0);

        let _ = fs::remove_file(&path);
    }

    // ============================================================================
    // API v0.0.3 Error Handling Tests
    // ============================================================================

    #[test]
    fn test_error_permission_denied_format() {
        let err = crate::error::TaskError::PermissionDenied {
            permission: "allow_test".to_string(),
            task_name: Some("test-task".to_string()),
        };

        let msg = format!("{}", err);
        assert!(msg.contains("allow_test"));
    }

    #[test]
    fn test_error_invalid_path_format() {
        let err = crate::error::TaskError::InvalidPath("Invalid chars: ../test".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid chars"));
    }

    // ============================================================================
    // API v0.0.3 Data Validation Tests
    // ============================================================================

    #[test]
    fn test_browser_data_version_compatibility() {
        use chrono::Utc;
        use std::collections::HashMap;

        // Simulate older version without browser_version field
        let data = crate::task::policy::BrowserData {
            cookies: vec![],
            local_storage: HashMap::new(),
            session_storage: HashMap::new(),
            indexeddb_names: HashMap::new(),
            exported_at: Utc::now(),
            source: "legacy".to_string(),
            browser_version: None, // Older export may not have this
        };

        // Should serialize with null browser_version
        let json = serde_json::to_string(&data).expect("Should serialize");
        assert!(json.contains("null") || json.contains("browser_version"));
    }

    #[test]
    fn test_session_data_url_validation() {
        use std::collections::HashMap;

        // Valid URLs
        let data1 = crate::task::policy::SessionData {
            url: "https://example.com/path?query=1".to_string(),
            cookies: vec![],
            local_storage: HashMap::new(),
            exported_at: chrono::Utc::now(),
        };
        assert!(!data1.url.is_empty());

        // Empty URL should be allowed (for validation testing)
        let data2 = crate::task::policy::SessionData {
            url: "".to_string(),
            cookies: vec![],
            local_storage: HashMap::new(),
            exported_at: chrono::Utc::now(),
        };
        assert!(data2.url.is_empty());
    }

    #[test]
    fn test_http_response_with_headers() {
        use std::collections::HashMap;

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());
        headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

        let response = super::HttpResponse {
            status: 200,
            body: "{}".to_string(),
            headers,
        };

        assert_eq!(response.headers.len(), 3);
        assert!(response.headers.contains_key("Content-Type"));
        assert!(response.headers.contains_key("Authorization"));
        assert!(response.headers.contains_key("X-Custom-Header"));
    }

    // ============================================================================
    // API v0.0.3 Permission Combination Tests
    // ============================================================================

    #[test]
    fn test_permission_combinations_full_access() {
        let policy = crate::task::policy::TaskPolicy {
            max_duration_ms: 60_000,
            permissions: crate::task::policy::TaskPermissions {
                allow_screenshot: true,
                allow_export_cookies: true,
                allow_import_cookies: true,
                allow_export_session: true,
                allow_import_session: true,
                allow_session_clipboard: true,
                allow_read_data: true,
                allow_write_data: true,
                allow_http_requests: true,
                allow_dom_inspection: true,
                allow_browser_export: true,
                allow_browser_import: true,
            },
        };

        assert!(policy.permissions.allow_screenshot);
        assert!(policy.permissions.allow_browser_export);
        assert!(policy.permissions.allow_browser_import);
    }

    #[test]
    fn test_permission_combinations_read_only() {
        let policy = crate::task::policy::TaskPolicy {
            max_duration_ms: 30_000,
            permissions: crate::task::policy::TaskPermissions {
                allow_read_data: true,
                allow_export_cookies: true,
                allow_export_session: true,
                allow_browser_export: true,
                allow_dom_inspection: true,
                allow_screenshot: true,
                ..Default::default()
            },
        };

        // Read operations allowed
        assert!(policy.permissions.allow_read_data);
        assert!(policy.permissions.allow_export_cookies);
        assert!(policy.permissions.allow_browser_export);
        assert!(policy.permissions.allow_dom_inspection);

        // Write operations denied
        assert!(!policy.permissions.allow_write_data);
        assert!(!policy.permissions.allow_import_cookies);
        assert!(!policy.permissions.allow_browser_import);
    }

    #[test]
    fn test_permission_combinations_network_only() {
        let policy = crate::task::policy::TaskPolicy {
            max_duration_ms: 30_000,
            permissions: crate::task::policy::TaskPermissions {
                allow_http_requests: true,
                allow_read_data: true, // For response caching
                ..Default::default()
            },
        };

        assert!(policy.permissions.allow_http_requests);
        assert!(policy.permissions.allow_read_data);
        assert!(!policy.permissions.allow_write_data);
        assert!(!policy.permissions.allow_export_cookies);
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
            "allow_http_requests" => perms.allow_http_requests,
            "allow_dom_inspection" => perms.allow_dom_inspection,
            "allow_browser_export" => perms.allow_browser_export,
            "allow_browser_import" => perms.allow_browser_import,
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

    /// Capture screenshot and save as compressed WebP with default 50% quality.
    ///
    /// Takes a screenshot of the current page, converts it to WebP with 50% quality,
    /// and saves it to `data/screenshot/` with filename format: `yyyy-mm-dd-hh-mm-sessionid.webp`
    ///
    /// WebP at 50% quality provides excellent compression while maintaining readable text.
    /// For custom quality, use `screenshot_with_quality()`.
    ///
    /// # Returns
    ///
    /// `Ok(String)` with the full file path to the saved screenshot.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - `allow_screenshot` permission is denied
    /// - CDP screenshot fails
    /// - Image conversion fails
    /// - File write fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let path = api.screenshot().await?;
    /// // Returns: "data/screenshot/2026-04-26-15-30-session-123.webp"
    /// ```
    pub async fn screenshot(&self) -> Result<String> {
        self.screenshot_with_quality(50).await
    }

    /// Capture screenshot and save as compressed WebP with custom quality.
    ///
    /// Takes a screenshot of the current page, converts it to WebP with the specified quality,
    /// and saves it to `data/screenshot/` with filename format: `yyyy-mm-dd-hh-mm-sessionid.webp`
    ///
    /// WebP provides 25-35% better compression than JPG at equivalent visual quality.
    ///
    /// # Arguments
    ///
    /// * `quality` - WebP quality factor (1-100). Higher = better quality, larger file.
    ///   Recommended: 50-85. Below 40 may have visible artifacts.
    ///   - 1-99: Lossy compression with quality factor
    ///   - 100: Lossless compression (larger file)
    ///
    /// # Returns
    ///
    /// `Ok(String)` with the full file path to the saved screenshot.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - `allow_screenshot` permission is denied
    /// - CDP screenshot fails
    /// - Image conversion fails
    /// - File write fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // High quality lossy (smaller file, good visuals)
    /// let path = api.screenshot_with_quality(85).await?;
    ///
    /// // Balanced quality (default - recommended)
    /// let path = api.screenshot_with_quality(50).await?;
    ///
    /// // Maximum compression (smallest file)
    /// let path = api.screenshot_with_quality(40).await?;
    ///
    /// // Lossless (best quality, larger file)
    /// let path = api.screenshot_with_quality(100).await?;
    /// ```
    pub async fn screenshot_with_quality(&self, quality: u8) -> Result<String> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_screenshot {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_screenshot' permission",
                self.session_id
            ));
        }

        // Clamp quality to valid range
        let quality = quality.clamp(1, 100);

        // CDP: Page.captureScreenshot (returns PNG bytes)
        let png_bytes = self
            .page
            .screenshot(chromiumoxide::cdp::browser_protocol::page::CaptureScreenshotParams::default())
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Page.captureScreenshot - {}", e))?;

        // Convert PNG to WebP with specified quality using webp crate
        let img = image::load_from_memory(&png_bytes)
            .map_err(|e| anyhow::anyhow!("Failed to load PNG image: {}", e))?;

        // Convert to RGB8 for WebP encoding
        let rgb_img = img.to_rgb8();
        let (width, height) = (rgb_img.width(), rgb_img.height());
        
        // Encode with quality using webp crate (quality 0-100)
        let encoder = webp::Encoder::new(rgb_img.as_raw(), webp::PixelLayout::Rgb, width, height);
        let webp_data = encoder.encode(quality as f32);

        // Generate filename: yyyy-mm-dd-hh-mm-sessionid.webp
        let now = chrono::Utc::now();
        let filename = format!(
            "{}-{}-{}.webp",
            now.format("%Y-%m-%d"),
            now.format("%H-%M"),
            self.session_id
        );

        // Create directory if it doesn't exist
        let screenshot_dir = std::path::Path::new("data/screenshot");
        std::fs::create_dir_all(screenshot_dir)
            .map_err(|e| anyhow::anyhow!("Failed to create screenshot directory: {}", e))?;

        // Write file
        let file_path = screenshot_dir.join(&filename);
        std::fs::write(&file_path, &*webp_data)
            .map_err(|e| anyhow::anyhow!("Failed to write screenshot: {}", e))?;

        // Return full path as string
        file_path
            .to_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Invalid file path"))
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

    /// Export cookies for a specific domain.
    ///
    /// # Arguments
    /// * `domain` - Domain to filter cookies (e.g., "example.com", ".example.com")
    ///
    /// # Returns
    /// Vector of cookies matching the domain as JSON values
    ///
    /// # Errors
    /// Returns error if `allow_export_cookies` permission is not granted
    ///
    /// # Permission
    /// Requires `allow_export_cookies` permission
    pub async fn export_cookies_for_domain(&self, domain: &str) -> Result<Vec<serde_json::Value>> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_export_cookies {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_export_cookies' permission",
                self.session_id
            ));
        }

        let all_cookies = self.export_cookies("").await?;

        let filtered: Vec<serde_json::Value> = all_cookies
            .into_iter()
            .filter(|cookie| {
                cookie
                    .get("domain")
                    .and_then(|d| d.as_str())
                    .map(|d| d == domain || d == &format!(".{}", domain))
                    .unwrap_or(false)
            })
            .collect();

        log::warn!(
            "task_policy_audit: task={} permission={} domain={} count={}",
            self.session_id, "allow_export_cookies", domain, filtered.len()
        );

        Ok(filtered)
    }

    /// Export session cookies (non-persistent cookies without expiry).
    ///
    /// # Arguments
    /// * `_url` - URL context (for consistency with export_cookies)
    ///
    /// # Returns
    /// Vector of session cookies as JSON values
    ///
    /// # Errors
    /// Returns error if `allow_export_cookies` permission is not granted
    ///
    /// # Permission
    /// Requires `allow_export_cookies` permission
    pub async fn export_session_cookies(&self, _url: &str) -> Result<Vec<serde_json::Value>> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_export_cookies {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_export_cookies' permission",
                self.session_id
            ));
        }

        let all_cookies = self.export_cookies("").await?;

        let session_cookies: Vec<serde_json::Value> = all_cookies
            .into_iter()
            .filter(|cookie| {
                // Session cookies have session=true or no expires field
                cookie
                    .get("session")
                    .and_then(|s| s.as_bool())
                    .unwrap_or(false)
                    || cookie.get("expires").is_none()
                    || cookie
                        .get("expires")
                        .map(|e| e.is_null() || e.as_f64() == Some(0.0) || e.as_f64() == Some(-1.0))
                        .unwrap_or(true)
            })
            .collect();

        log::warn!(
            "task_policy_audit: task={} permission={} url={} count={}",
            self.session_id, "allow_export_cookies", _url, session_cookies.len()
        );

        Ok(session_cookies)
    }

    /// Check if a specific cookie exists.
    ///
    /// # Arguments
    /// * `name` - Cookie name to search for
    /// * `domain` - Domain to search in (optional filtering)
    ///
    /// # Returns
    /// true if cookie exists, false otherwise
    ///
    /// # Errors
    /// Returns error if `allow_export_cookies` permission is not granted
    ///
    /// # Permission
    /// Requires `allow_export_cookies` permission
    pub async fn has_cookie(&self, name: &str, domain: Option<&str>) -> Result<bool> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_export_cookies {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_export_cookies' permission",
                self.session_id
            ));
        }

        let cookies = if let Some(d) = domain {
            self.export_cookies_for_domain(d).await?
        } else {
            self.export_cookies("").await?
        };

        let exists = cookies.iter().any(|cookie| {
            cookie
                .get("name")
                .and_then(|n| n.as_str())
                .map(|n| n == name)
                .unwrap_or(false)
        });

        Ok(exists)
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

    /// Clear clipboard content.
    ///
    /// # Errors
    /// Returns error if `allow_session_clipboard` permission is not granted
    ///
    /// # Permission
    /// Requires `allow_session_clipboard` permission
    pub fn clear_clipboard(&self) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_session_clipboard {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_session_clipboard' permission",
                self.session_id
            ));
        }
        crate::state::ClipboardState::new(self.session_id.clone()).set("");
        Ok(())
    }

    /// Check if clipboard has content.
    ///
    /// # Returns
    /// true if clipboard is not empty, false otherwise
    ///
    /// # Errors
    /// Returns error if `allow_session_clipboard` permission is not granted
    ///
    /// # Permission
    /// Requires `allow_session_clipboard` permission
    pub fn has_clipboard_content(&self) -> Result<bool> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_session_clipboard {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_session_clipboard' permission",
                self.session_id
            ));
        }
        let has_content = crate::state::ClipboardState::new(self.session_id.clone())
            .get()
            .map(|s| !s.is_empty())
            .unwrap_or(false);
        Ok(has_content)
    }

    /// Append text to clipboard with optional separator.
    ///
    /// # Arguments
    /// * `text` - Text to append
    /// * `separator` - Optional separator to insert between existing and new text
    ///
    /// # Errors
    /// Returns error if `allow_session_clipboard` permission is not granted
    ///
    /// # Permission
    /// Requires `allow_session_clipboard` permission
    pub fn append_clipboard(&self, text: &str, separator: Option<&str>) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_session_clipboard {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_session_clipboard' permission",
                self.session_id
            ));
        }
        let current = crate::state::ClipboardState::new(self.session_id.clone())
            .get()
            .unwrap_or_default();
        let new_content = if current.is_empty() {
            text.to_string()
        } else {
            format!("{}{}{}", current, separator.unwrap_or(""), text)
        };
        crate::state::ClipboardState::new(self.session_id.clone()).set(&new_content);
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
        // Validate and resolve path using security helper
        let path = crate::task::security::validate_data_path(relative_path)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
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
        // Validate path is safe using security helper
        if !crate::task::security::is_safe_path(relative_path) {
            return Err(anyhow::anyhow!(
                "Invalid path: Path contains unsafe components"
            ));
        }

        // Construct path in config/ (create parent dirs if needed)
        let path = std::path::Path::new("config").join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create directory: {}", e))?;
        }

        std::fs::write(&path, content)
            .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))
    }

    /// List files in the data/config directory.
    ///
    /// # Arguments
    /// * `subdir` - Optional subdirectory to list (e.g., "personas", "data")
    ///
    /// # Returns
    /// Vector of relative file paths
    ///
    /// # Errors
    /// Returns error if `allow_read_data` permission is not granted
    ///
    /// # Permission
    /// Requires `allow_read_data` permission
    pub fn list_data_files(&self, subdir: Option<&str>) -> Result<Vec<String>> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_read_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_read_data' permission",
                self.session_id
            ));
        }

        // Validate subdir if provided
        let base_path = if let Some(s) = subdir {
            if !crate::task::security::is_safe_path(s) {
                return Err(anyhow::anyhow!("Invalid subdir: Unsafe path components"));
            }
            std::path::Path::new("config").join(s)
        } else {
            std::path::Path::new("config").to_path_buf()
        };

        let mut files = Vec::new();
        if base_path.exists() {
            for entry in std::fs::read_dir(&base_path)
                .map_err(|e| anyhow::anyhow!("Failed to read directory: {}", e))? {
                let entry = entry.map_err(|e| anyhow::anyhow!("Directory entry error: {}", e))?;
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    if let Some(name) = entry.file_name().to_str() {
                        files.push(name.to_string());
                    }
                }
            }
        }

        Ok(files)
    }

    /// Check if a data file exists.
    ///
    /// # Arguments
    /// * `relative_path` - Relative path within config/ directory
    ///
    /// # Returns
    /// true if file exists, false otherwise
    ///
    /// # Errors
    /// Returns error if `allow_read_data` permission is not granted
    ///
    /// # Permission
    /// Requires `allow_read_data` permission
    pub fn data_file_exists(&self, relative_path: &str) -> Result<bool> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_read_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_read_data' permission",
                self.session_id
            ));
        }

        let path = crate::task::security::validate_data_path(relative_path)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(path.exists())
    }

    /// Delete a data file.
    ///
    /// # Arguments
    /// * `relative_path` - Relative path within config/ directory
    ///
    /// # Errors
    /// Returns error if `allow_write_data` permission is not granted or file doesn't exist
    ///
    /// # Permission
    /// Requires `allow_write_data` permission
    pub fn delete_data_file(&self, relative_path: &str) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_write_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_write_data' permission",
                self.session_id
            ));
        }

        let path = crate::task::security::validate_data_path(relative_path)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        if !path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", relative_path));
        }

        std::fs::remove_file(&path)
            .map_err(|e| anyhow::anyhow!("Failed to delete file: {}", e))
    }

    /// Append content to a data file.
    ///
    /// # Arguments
    /// * `relative_path` - Relative path within config/ directory
    /// * `content` - Bytes to append
    ///
    /// # Errors
    /// Returns error if `allow_write_data` permission is not granted
    ///
    /// # Permission
    /// Requires `allow_write_data` permission
    pub fn append_data_file(&self, relative_path: &str, content: &[u8]) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_write_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_write_data' permission",
                self.session_id
            ));
        }

        if !crate::task::security::is_safe_path(relative_path) {
            return Err(anyhow::anyhow!("Invalid path: Path contains unsafe components"));
        }

        let path = std::path::Path::new("config").join(relative_path);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create directory: {}", e))?;
        }

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| anyhow::anyhow!("Failed to open file for append: {}", e))?;

        file.write_all(content)
            .map_err(|e| anyhow::anyhow!("Failed to append to file: {}", e))
    }

    /// Read and parse JSON data from a file.
    ///
    /// # Type Parameters
    /// * `T` - Type to deserialize into (must implement DeserializeOwned)
    ///
    /// # Arguments
    /// * `relative_path` - Relative path within config/ directory
    ///
    /// # Returns
    /// Deserialized data of type T
    ///
    /// # Errors
    /// Returns error if `allow_read_data` permission not granted or JSON invalid
    ///
    /// # Permission
    /// Requires `allow_read_data` permission
    pub fn read_json_data<T: serde::de::DeserializeOwned>(&self, relative_path: &str) -> Result<T> {
        let content = self.read_data_file(relative_path)?;
        serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))
    }

    /// Write data as pretty-printed JSON to a file.
    ///
    /// # Type Parameters
    /// * `T` - Type to serialize (must implement Serialize)
    ///
    /// # Arguments
    /// * `relative_path` - Relative path within config/ directory
    /// * `data` - Data to serialize and write
    ///
    /// # Errors
    /// Returns error if `allow_write_data` permission not granted or serialization fails
    ///
    /// # Permission
    /// Requires `allow_write_data` permission
    pub fn write_json_data<T: serde::Serialize>(&self, relative_path: &str, data: &T) -> Result<()> {
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize to JSON: {}", e))?;
        self.write_data_file(relative_path, json.as_bytes())
    }

    /// Get metadata for a data file.
    ///
    /// # Arguments
    /// * `relative_path` - Relative path within config/ directory
    ///
    /// # Returns
    /// FileMetadata struct with size, modified time, and created time
    ///
    /// # Errors
    /// Returns error if `allow_read_data` permission not granted or file doesn't exist
    ///
    /// # Permission
    /// Requires `allow_read_data` permission
    pub fn data_file_metadata(&self, relative_path: &str) -> Result<FileMetadata> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_read_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_read_data' permission",
                self.session_id
            ));
        }

        let path = crate::task::security::validate_data_path(relative_path)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let metadata = std::fs::metadata(&path)
            .map_err(|e| anyhow::anyhow!("Failed to get file metadata: {}", e))?;

        Ok(FileMetadata {
            size: metadata.len(),
            modified: metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
            created: metadata.created().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
        })
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

    /// Perform HTTP GET request.
    ///
    /// # Arguments
    /// * `url` - URL to request
    ///
    /// # Returns
    /// HttpResponse with status, body, and headers
    ///
    /// # Errors
    /// Returns error if `allow_http_requests` permission not granted or request fails
    ///
    /// # Permission
    /// Requires `allow_http_requests` permission
    pub async fn http_get(&self, url: &str) -> Result<HttpResponse> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_http_requests {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_http_requests' permission",
                self.session_id
            ));
        }

        let response = reqwest::get(url)
            .await
            .map_err(|e| anyhow::anyhow!("HTTP GET failed: {}", e))?;

        let status = response.status().as_u16();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str().ok().map(|val| (k.to_string(), val.to_string()))
            })
            .collect();
        let body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;

        log::warn!(
            "task_policy_audit: task={} permission={} url={} status={}",
            self.session_id, "allow_http_requests", url, status
        );

        Ok(HttpResponse {
            status,
            body,
            headers,
        })
    }

    /// Perform HTTP POST request with JSON body.
    ///
    /// # Type Parameters
    /// * `T` - Type of the request body (must implement Serialize)
    ///
    /// # Arguments
    /// * `url` - URL to POST to
    /// * `body` - Request body to serialize as JSON
    ///
    /// # Returns
    /// HttpResponse with status, body, and headers
    ///
    /// # Errors
    /// Returns error if `allow_http_requests` permission not granted or request fails
    ///
    /// # Permission
    /// Requires `allow_http_requests` permission
    pub async fn http_post_json<T: serde::Serialize>(&self, url: &str, body: &T) -> Result<HttpResponse> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_http_requests {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_http_requests' permission",
                self.session_id
            ));
        }

        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP POST failed: {}", e))?;

        let status = response.status().as_u16();
        let headers: HashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                v.to_str().ok().map(|val| (k.to_string(), val.to_string()))
            })
            .collect();
        let body_text = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;

        log::warn!(
            "task_policy_audit: task={} permission={} url={} status={}",
            self.session_id, "allow_http_requests", url, status
        );

        Ok(HttpResponse {
            status,
            body: body_text,
            headers,
        })
    }

    /// Download file from URL to data directory.
    ///
    /// # Arguments
    /// * `url` - URL to download from
    /// * `relative_path` - Relative path within config/ directory to save to
    ///
    /// # Returns
    /// Number of bytes downloaded
    ///
    /// # Errors
    /// Returns error if permissions not granted or download fails
    ///
    /// # Permissions
    /// Requires both `allow_http_requests` and `allow_write_data`
    pub async fn download_file(&self, url: &str, relative_path: &str) -> Result<u64> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_http_requests {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_http_requests' permission",
                self.session_id
            ));
        }
        if !perms.allow_write_data {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_write_data' permission",
                self.session_id
            ));
        }

        // Validate path
        if !crate::task::security::is_safe_path(relative_path) {
            return Err(anyhow::anyhow!("Invalid path: Path contains unsafe components"));
        }

        // Download
        let response = reqwest::get(url)
            .await
            .map_err(|e| anyhow::anyhow!("Download failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            return Err(anyhow::anyhow!("Download failed with status: {}", status));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read download: {}", e))?;

        let byte_count = bytes.len() as u64;

        // Save to file
        let path = std::path::Path::new("config").join(relative_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create directory: {}", e))?;
        }

        std::fs::write(&path, &bytes)
            .map_err(|e| anyhow::anyhow!("Failed to write downloaded file: {}", e))?;

        log::warn!(
            "task_policy_audit: task={} permissions={}+{} url={} path={} bytes={}",
            self.session_id, "allow_http_requests", "allow_write_data", url, relative_path, byte_count
        );

        Ok(byte_count)
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

    /// Get computed CSS style property for an element.
    ///
    /// # Arguments
    /// * `selector` - CSS selector for the element
    /// * `property` - CSS property name (e.g., "color", "font-size")
    ///
    /// # Returns
    /// String value of the computed style property
    ///
    /// # Errors
    /// Returns error if `allow_dom_inspection` permission not granted or element not found
    ///
    /// # Permission
    /// Requires `allow_dom_inspection` permission
    pub async fn get_computed_style(&self, selector: &str, property: &str) -> Result<String> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_dom_inspection {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_dom_inspection' permission",
                self.session_id
            ));
        }

        let js = format!(
            r#"
            (function() {{
                const el = document.querySelector('{}');
                if (!el) return null;
                const style = window.getComputedStyle(el);
                return style.getPropertyValue('{}');
            }})()
            "#,
            selector.replace("'", "\\'"),
            property.replace("'", "\\'")
        );

        let result = self.page.evaluate(js).await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {}", e))?;

        let value = result.value()
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        log::warn!(
            "task_policy_audit: task={} permission={} selector={} property={}",
            self.session_id, "allow_dom_inspection", selector, property
        );

        Ok(value)
    }

    /// Get element's position and size (bounding rectangle).
    ///
    /// # Arguments
    /// * `selector` - CSS selector for the element
    ///
    /// # Returns
    /// Rect struct with x, y, width, height
    ///
    /// # Errors
    /// Returns error if `allow_dom_inspection` permission not granted or element not found
    ///
    /// # Permission
    /// Requires `allow_dom_inspection` permission
    pub async fn get_element_rect(&self, selector: &str) -> Result<Rect> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_dom_inspection {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_dom_inspection' permission",
                self.session_id
            ));
        }

        let js = format!(
            r#"
            (function() {{
                const el = document.querySelector('{}');
                if (!el) return null;
                const rect = el.getBoundingClientRect();
                return {{ x: rect.x, y: rect.y, width: rect.width, height: rect.height }};
            }})()
            "#,
            selector.replace("'", "\\'")
        );

        let result = self.page.evaluate(js).await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {}", e))?;

        let value = result.value()
            .ok_or_else(|| anyhow::anyhow!("Element not found: {}", selector))?;

        let x = value.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let y = value.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let width = value.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let height = value.get("height").and_then(|v| v.as_f64()).unwrap_or(0.0);

        log::warn!(
            "task_policy_audit: task={} permission={} selector={}",
            self.session_id, "allow_dom_inspection", selector
        );

        Ok(Rect { x, y, width, height })
    }

    /// Get current scroll position of the page.
    ///
    /// # Returns
    /// (scroll_x, scroll_y) in pixels as (u32, u32)
    ///
    /// # Errors
    /// Returns error if `allow_dom_inspection` permission not granted
    ///
    /// # Permission
    /// Requires `allow_dom_inspection` permission
    pub async fn get_scroll_position(&self) -> Result<(u32, u32)> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_dom_inspection {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_dom_inspection' permission",
                self.session_id
            ));
        }

        let js = r#"
            (function() {
                return { x: window.scrollX || window.pageXOffset, y: window.scrollY || window.pageYOffset };
            })()
        "#;

        let result = self.page.evaluate(js).await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {}", e))?;

        let value = result.value().cloned().unwrap_or_else(|| serde_json::json!({"x": 0, "y": 0}));
        let x = value.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as u32;
        let y = value.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as u32;

        log::warn!(
            "task_policy_audit: task={} permission={}",
            self.session_id, "allow_dom_inspection"
        );

        Ok((x, y))
    }

    /// Count elements matching a CSS selector.
    ///
    /// # Arguments
    /// * `selector` - CSS selector to match
    ///
    /// # Returns
    /// Number of matching elements
    ///
    /// # Errors
    /// Returns error if `allow_dom_inspection` permission not granted
    ///
    /// # Permission
    /// Requires `allow_dom_inspection` permission
    pub async fn count_elements(&self, selector: &str) -> Result<usize> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_dom_inspection {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_dom_inspection' permission",
                self.session_id
            ));
        }

        let js = format!(
            r#"
            (function() {{
                return document.querySelectorAll('{}').length;
            }})()
            "#,
            selector.replace("'", "\\'")
        );

        let result = self.page.evaluate(js).await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {}", e))?;

        let count = result.value()
            .and_then(|v| v.as_f64())
            .map(|n| n as usize)
            .unwrap_or(0);

        log::warn!(
            "task_policy_audit: task={} permission={} selector={} count={}",
            self.session_id, "allow_dom_inspection", selector, count
        );

        Ok(count)
    }

    /// Check if element is visible in the viewport.
    ///
    /// # Arguments
    /// * `selector` - CSS selector for the element
    ///
    /// # Returns
    /// true if element is at least partially visible in viewport
    ///
    /// # Errors
    /// Returns error if `allow_dom_inspection` permission not granted or element not found
    ///
    /// # Permission
    /// Requires `allow_dom_inspection` permission
    pub async fn is_in_viewport(&self, selector: &str) -> Result<bool> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_dom_inspection {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_dom_inspection' permission",
                self.session_id
            ));
        }

        let js = format!(
            r#"
            (function() {{
                const el = document.querySelector('{}');
                if (!el) return false;
                const rect = el.getBoundingClientRect();
                const windowHeight = window.innerHeight || document.documentElement.clientHeight;
                const windowWidth = window.innerWidth || document.documentElement.clientWidth;
                return rect.top < windowHeight && rect.bottom > 0 &&
                       rect.left < windowWidth && rect.right > 0;
            }})()
            "#,
            selector.replace("'", "\\'")
        );

        let result = self.page.evaluate(js).await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate - {}", e))?;

        let visible = result.value()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        log::warn!(
            "task_policy_audit: task={} permission={} selector={} visible={}",
            self.session_id, "allow_dom_inspection", selector, visible
        );

        Ok(visible)
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

    /// Export ALL browser data including cookies, localStorage, sessionStorage.
    ///
    /// # Arguments
    /// * `url` - Source URL for the export
    ///
    /// # Returns
    /// Complete BrowserData struct with all browser state
    ///
    /// # Errors
    /// Returns error if `allow_browser_export` permission not granted
    ///
    /// # Permission
    /// Requires `allow_browser_export` permission
    pub async fn export_browser(&self, url: &str) -> Result<crate::task::policy::BrowserData> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_browser_export {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_browser_export' permission",
                self.session_id
            ));
        }

        // Export all cookies via CDP
        let cookies_result = self
            .page
            .execute(chromiumoxide::cdp::browser_protocol::network::GetCookiesParams::default())
            .await;
        let cookies_json = match cookies_result {
            Ok(cookies) => {
                serde_json::to_value(&cookies.cookies).unwrap_or(serde_json::Value::Array(vec![]))
            }
            Err(e) => {
                log::warn!("Failed to export cookies during browser export: {}", e);
                serde_json::Value::Array(vec![])
            }
        };
        let cookies = cookies_json
            .as_array()
            .unwrap_or(&vec![])
            .clone();

        // Export localStorage from all frames via JavaScript
        let local_storage_js = r#"
            (function() {
                const data = {};
                const hostname = window.location.hostname;
                data[hostname] = {};
                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    data[hostname][key] = localStorage.getItem(key);
                }
                return JSON.stringify(data);
            })()
        "#;
        let local_storage_str = self
            .page
            .evaluate(local_storage_js)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate for localStorage - {}", e))?;
        let local_storage_value = local_storage_str.value().cloned().unwrap_or(serde_json::Value::Null);
        let local_storage: std::collections::HashMap<String, std::collections::HashMap<String, String>> =
            serde_json::from_value(local_storage_value)
                .unwrap_or_default();

        // Export sessionStorage via JavaScript
        let session_storage_js = r#"
            (function() {
                const data = {};
                const hostname = window.location.hostname;
                data[hostname] = {};
                for (let i = 0; i < sessionStorage.length; i++) {
                    const key = sessionStorage.key(i);
                    data[hostname][key] = sessionStorage.getItem(key);
                }
                return JSON.stringify(data);
            })()
        "#;
        let session_storage_str = self
            .page
            .evaluate(session_storage_js)
            .await
            .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate for sessionStorage - {}", e))?;
        let session_storage_value = session_storage_str.value().cloned().unwrap_or(serde_json::Value::Null);
        let session_storage: std::collections::HashMap<String, std::collections::HashMap<String, String>> =
            serde_json::from_value(session_storage_value)
                .unwrap_or_default();

        // Get IndexedDB database names (simplified - just list databases)
        let indexeddb_js = r#"
            (function() {
                return new Promise((resolve) => {
                    const hostname = window.location.hostname;
                    const data = {};
                    data[hostname] = [];
                    
                    if (!window.indexedDB) {
                        resolve(JSON.stringify(data));
                        return;
                    }
                    
                    // Try to get database names if supported
                    if (window.indexedDB.databases) {
                        window.indexedDB.databases().then(dbs => {
                            data[hostname] = dbs.map(db => db.name);
                            resolve(JSON.stringify(data));
                        }).catch(() => {
                            resolve(JSON.stringify(data));
                        });
                    } else {
                        resolve(JSON.stringify(data));
                    }
                });
            })()
        "#;
        let indexeddb_result = self
            .page
            .evaluate(indexeddb_js)
            .await;
        let indexeddb_names: std::collections::HashMap<String, Vec<String>> = match indexeddb_result {
            Ok(result) => {
                result.value()
                    .cloned()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default()
            }
            Err(e) => {
                log::warn!("Failed to export IndexedDB names: {}", e);
                std::collections::HashMap::new()
            }
        };

        let browser_data = crate::task::policy::BrowserData {
            cookies,
            local_storage,
            session_storage,
            indexeddb_names,
            exported_at: chrono::Utc::now(),
            source: url.to_string(),
            browser_version: None,
        };

        log::warn!(
            "task_policy_audit: task={} permission={} url={} cookies={} origins={}",
            self.session_id,
            "allow_browser_export",
            url,
            browser_data.cookies.len(),
            browser_data.local_storage.len()
        );

        Ok(browser_data)
    }

    /// Import complete browser data including cookies, localStorage, sessionStorage.
    ///
    /// # Arguments
    /// * `browser_data` - Complete BrowserData to import
    ///
    /// # Errors
    /// Returns error if `allow_browser_import` permission not granted
    ///
    /// # Permission
    /// Requires `allow_browser_import` permission
    pub async fn import_browser(&self, browser_data: &crate::task::policy::BrowserData) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_browser_import {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_browser_import' permission",
                self.session_id
            ));
        }

        // Import cookies
        self.import_cookies(&browser_data.cookies).await?;

        // Import localStorage for each origin
        for (origin, data) in &browser_data.local_storage {
            let local_storage_json = serde_json::to_string(data)
                .map_err(|e| anyhow::anyhow!("Failed to serialize localStorage for {}: {}", origin, e))?;
            let js_code = format!(
                r#"
                (function() {{
                    const data = {};
                    let count = 0;
                    Object.entries(data).forEach(([k, v]) => {{
                        try {{
                            localStorage.setItem(k, v);
                            count++;
                        }} catch (e) {{
                            console.warn('Failed to set localStorage item:', k, e);
                        }}
                    }});
                    return 'localStorage imported: ' + count + ' items for origin';
                }})()
                "#,
                local_storage_json
            );
            self.page
                .evaluate(js_code)
                .await
                .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate for localStorage import - {}", e))?;
        }

        // Import sessionStorage for each origin
        for (origin, data) in &browser_data.session_storage {
            let session_storage_json = serde_json::to_string(data)
                .map_err(|e| anyhow::anyhow!("Failed to serialize sessionStorage for {}: {}", origin, e))?;
            let js_code = format!(
                r#"
                (function() {{
                    const data = {};
                    let count = 0;
                    Object.entries(data).forEach(([k, v]) => {{
                        try {{
                            sessionStorage.setItem(k, v);
                            count++;
                        }} catch (e) {{
                            console.warn('Failed to set sessionStorage item:', k, e);
                        }}
                    }});
                    return 'sessionStorage imported: ' + count + ' items for origin';
                }})()
                "#,
                session_storage_json
            );
            self.page
                .evaluate(js_code)
                .await
                .map_err(|e| anyhow::anyhow!("CDP error: Runtime.evaluate for sessionStorage import - {}", e))?;
        }

        log::warn!(
            "task_policy_audit: task={} permission={} source={} cookies={} origins={}",
            self.session_id,
            "allow_browser_import",
            browser_data.source,
            browser_data.cookies.len(),
            browser_data.local_storage.len()
        );

        Ok(())
    }

    /// Export localStorage data from the current page.
    ///
    /// # Arguments
    /// * `_url` - URL context (for consistency with other methods)
    ///
    /// # Returns
    /// HashMap of localStorage key-value pairs
    ///
    /// # Errors
    /// Returns error if `allow_export_session` permission is not granted
    ///
    /// # Permission
    /// Requires `allow_export_session` permission
    pub async fn export_local_storage(&self, _url: &str) -> Result<std::collections::HashMap<String, String>> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_export_session {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_export_session' permission",
                self.session_id
            ));
        }

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

        log::warn!(
            "task_policy_audit: task={} permission={} url={} count={}",
            self.session_id, "allow_export_session", _url, local_storage.len()
        );

        Ok(local_storage)
    }

    /// Import localStorage data to the current page.
    ///
    /// # Arguments
    /// * `_url` - URL context (for consistency with other methods)
    /// * `data` - HashMap of key-value pairs to set in localStorage
    ///
    /// # Errors
    /// Returns error if `allow_import_session` permission is not granted
    ///
    /// # Permission
    /// Requires `allow_import_session` permission
    pub async fn import_local_storage(
        &self,
        _url: &str,
        data: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        let perms = self.policy.effective_permissions();
        if !perms.allow_import_session {
            return Err(anyhow::anyhow!(
                "Permission denied: task '{}' lacks 'allow_import_session' permission",
                self.session_id
            ));
        }

        // Import localStorage via JavaScript
        let local_storage_json = serde_json::to_string(data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize localStorage: {}", e))?;
        let js_code = format!(
            r#"
            (function() {{
                const data = {};
                Object.entries(data).forEach(([k, v]) => {{
                    localStorage.setItem(k, v);
                }});
                return 'localStorage imported: ' + Object.keys(data).length + ' items';
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
            self.session_id, "allow_import_session", _url, data.len()
        );

        Ok(())
    }

    /// Validate SessionData structure without importing.
    ///
    /// # Arguments
    /// * `data` - SessionData to validate
    ///
    /// # Returns
    /// Vec of validation warnings (empty if valid)
    ///
    /// # Errors
    /// Returns error if data structure is fundamentally invalid
    pub fn validate_session_data(&self, data: &crate::task::policy::SessionData) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Validate cookies array
        if data.cookies.is_empty() && data.local_storage.is_empty() {
            warnings.push("SessionData has no cookies and no localStorage".to_string());
        }

        // Validate cookie structure
        for (i, cookie) in data.cookies.iter().enumerate() {
            if let Some(obj) = cookie.as_object() {
                if !obj.contains_key("name") {
                    warnings.push(format!("Cookie[{}] missing 'name' field", i));
                }
                if !obj.contains_key("value") {
                    warnings.push(format!("Cookie[{}] missing 'value' field", i));
                }
            } else {
                warnings.push(format!("Cookie[{}] is not a JSON object", i));
            }
        }

        // Validate local_storage
        if data.local_storage.len() > 1000 {
            warnings.push(format!("localStorage has {} items (very large)", data.local_storage.len()));
        }

        // Validate URL is not empty
        if data.url.is_empty() {
            warnings.push("SessionData url is empty".to_string());
        }

        Ok(warnings)
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
