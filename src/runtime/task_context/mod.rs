//! Task API context for browser automation.
//!
//! The `TaskContext` provides a high-level, task-facing API for browser automation.
//! Tasks should use this API exclusively rather than accessing internal utilities directly.
//!
//! # Task API Verbs
//!
//! The `TaskContext` provides short, readable verbs for common actions:
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
//! - `pause()` - Uniform-random pause (~20% spread), ends early on task cancel when wired
//! - `pause_with_variance()` - Uniform-random pause with custom spread
//! - `pause_human()` - Gaussian pause for human-like timing
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
#[allow(unused_imports)]
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use log::info;
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::adaptive::LearningEngine;
use crate::capabilities::{dom, mouse, scroll, timing};
use crate::config::{BrowserConfig, NativeInteractionConfig};
use crate::internal::page_size::Viewport;
use crate::logger::scoped_log_context;
use crate::metrics::MetricsCollector;
use crate::task::policy::TaskPolicy;
use crate::utils::profile::{BrowserProfile, ProfileRuntime};
use crate::utils::{ClickOutcome, ClickStatus, HoverOutcome, NativeCursorOutcome};
use crate::ClipboardState;

// Submodules
mod click;
pub mod click_learning;
pub mod clipboard;
pub mod cookies;
pub mod data_files;
pub mod dom_verify;
pub mod frame;
pub mod http;
pub mod interaction;
pub mod interaction_pipeline;
pub mod page_nav;
mod pointer;
pub mod query;
pub mod session_io;
pub mod style;
pub mod types;
pub mod validation;

// Re-export submodule contents for convenient access
pub use click_learning::{
    click_learning_path, load_click_learning, save_click_learning, ClickAdaptation,
    ClickElementPriority, ClickFatigueLevel, ClickLearningState, ClickPageContext,
    ClickTimingContext, ClickTimingProfile, SelectorLearningStats,
};
pub use interaction_pipeline::execute_interaction;
pub use types::{
    ClickAndWaitOutcome, FileMetadata, FocusOutcome, FocusStatus, HttpResponse, InteractionKind,
    InteractionRequest, InteractionResult, RandomCursorOutcome, Rect, WaitForVisibleStatus,
};

// Query and interaction modules are public for standalone use
pub use interaction as actions;
pub use query as dom_query;

pub(crate) fn nativeclick_public_log_line(selector: &str, x: f64, y: f64) -> String {
    format!("[task-api] clicked ({selector}) at {x:.1},{y:.1}")
}

/// Deserialize evaluated JSON value, handling both direct and string-encoded JSON.
pub(crate) fn deserialize_evaluated_json<T: DeserializeOwned>(
    value: serde_json::Value,
) -> Result<T> {
    match value {
        serde_json::Value::String(s) => Ok(serde_json::from_str(&s)?),
        other => Ok(serde_json::from_value(other)?),
    }
}

#[cfg(test)]
mod tests;
#[derive(Clone)]
pub struct TaskContext {
    session_id: String,
    page: Arc<Page>,
    clipboard: ClipboardState,
    behavior_profile: BrowserProfile,
    behavior_runtime: ProfileRuntime,
    native_interaction: NativeInteractionConfig,
    metrics: Option<Arc<MetricsCollector>>,
    learning_engine: Arc<Mutex<LearningEngine>>,
    policy: &'static TaskPolicy,
    /// When set (orchestrated runs), `pause`/`pause_with_variance`/`pause_human` return early on cancel.
    cancel_token: Option<CancellationToken>,
}

const MIN_NAVIGATE_TIMEOUT_MS: u64 = 20_000;

pub(crate) fn wrapper_timeout_context(stage: &str, details: impl Into<String>) -> String {
    format!("wrapper_timeout | stage={} {}", stage, details.into())
}

impl TaskContext {
    /// Creates a new `TaskContext` for browser automation.
    ///
    /// # Arguments
    ///
    /// * `session_id` - The session identifier
    /// * `page` - The browser page to automate
    /// * `behavior_profile` - The behavior profile for human-like interactions
    /// * `behavior_runtime` - The runtime behavior configuration
    /// * `native_interaction` - Native OS input calibration and timing settings
    /// * `cancel_token` - When `Some`, [`Self::pause`] family returns early if the token is cancelled
    ///
    /// # Returns
    ///
    /// A new `TaskContext` instance ready for task execution.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use auto::runtime::task_context::TaskContext;
    /// # use chromiumoxide::Page;
    /// # use std::sync::Arc;
    /// # use auto::internal::profile::{BrowserProfile, ProfileRuntime};
    /// # use auto::config::{NativeInteractionConfig, BrowserConfig};
    /// # use auto::task::policy::DEFAULT_TASK_POLICY;
    /// # async fn example(page: Arc<Page>, profile: BrowserProfile, runtime: ProfileRuntime) {
    /// let browser_config = BrowserConfig::default();
    /// let api = TaskContext::new("session-1", page, profile, runtime, NativeInteractionConfig::default(), &browser_config, &DEFAULT_TASK_POLICY, None);
    /// # }
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: impl Into<String>,
        page: Arc<Page>,
        behavior_profile: BrowserProfile,
        behavior_runtime: ProfileRuntime,
        native_interaction: NativeInteractionConfig,
        browser_config: &BrowserConfig,
        policy: &'static TaskPolicy,
        cancel_token: Option<CancellationToken>,
    ) -> Self {
        let session_id = session_id.into();
        let clipboard = ClipboardState::new(session_id.clone());
        let learning_engine = if browser_config.enable_learning_persistence {
            LearningEngine::new(
                &session_id,
                &behavior_profile,
                true,
                browser_config.learning_ttl_days,
            )
            .unwrap_or_else(|e| {
                log::warn!("Failed to initialize LearningEngine, falling back to disabled: {e}");
                LearningEngine::disabled()
            })
        } else {
            LearningEngine::disabled()
        };
        Self {
            session_id,
            page,
            clipboard,
            behavior_profile,
            behavior_runtime,
            native_interaction,
            metrics: None,
            learning_engine: Arc::new(Mutex::new(learning_engine)),
            policy,
            cancel_token,
        }
    }

    /// Like [`Self::new`] but attaches metrics. Pass the same `cancel_token` as the orchestrator
    /// uses for the task attempt so [`Self::pause`] / [`Self::pause_with_variance`] / [`Self::pause_human`]
    /// return early on group cancellation.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_metrics(
        session_id: impl Into<String>,
        page: Arc<Page>,
        behavior_profile: BrowserProfile,
        behavior_runtime: ProfileRuntime,
        native_interaction: NativeInteractionConfig,
        metrics: Arc<MetricsCollector>,
        browser_config: &BrowserConfig,
        policy: &'static TaskPolicy,
        cancel_token: Option<CancellationToken>,
    ) -> Self {
        let mut ctx = Self::new(
            session_id,
            page,
            behavior_profile,
            behavior_runtime,
            native_interaction,
            browser_config,
            policy,
            cancel_token,
        );
        ctx.metrics = Some(metrics);
        ctx
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(crate) fn page(&self) -> &Page {
        &self.page
    }

    #[must_use]
    pub fn clipboard(&self) -> &ClipboardState {
        &self.clipboard
    }

    #[must_use]
    pub fn behavior_profile(&self) -> &BrowserProfile {
        &self.behavior_profile
    }

    #[must_use]
    pub fn behavior_runtime(&self) -> &ProfileRuntime {
        &self.behavior_runtime
    }

    #[must_use]
    pub fn native_interaction(&self) -> &NativeInteractionConfig {
        &self.native_interaction
    }

    pub fn increment_run_counter(&self, name: &str, amount: usize) {
        if let Some(metrics) = &self.metrics {
            metrics.increment_run_counter(name, amount);
        }
    }

    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn metrics(&self) -> &MetricsCollector {
        self.metrics
            .as_ref()
            .expect("Metrics collector not initialized")
    }

    /// Scroll in a random direction by random amount.
    pub async fn random_scroll(&self) -> Result<()> {
        scroll::random_scroll(self.page()).await
    }

    /// Scroll selector into view with post-scroll pause.
    pub async fn scroll_to(&self, selector: &str) -> Result<()> {
        interaction::scroll_into_view(self.page(), selector).await?;

        // Phase2: Verify element is in viewport after scroll
        if !self.is_in_viewport(selector).await? {
            return Err(anyhow::anyhow!(
                "[task-api] scroll_to: element '{selector}' not in viewport after scroll"
            ));
        }

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
        interaction::back(self.page(), distance).await?;
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
        interaction::copy(self.session_id(), self.page()).await
    }

    /// Select all + cut to clipboard. Returns cut content.
    pub async fn cut(&self) -> Result<String> {
        interaction::cut(self.session_id(), self.page()).await
    }

    /// Paste clipboard content into focused element. Returns pasted content.
    pub async fn paste(&self) -> Result<String> {
        interaction::paste(self.session_id(), self.page()).await
    }

    /// Wait for `base_ms` with **20% uniform** spread (same family as [`Self::pause_with_variance`]).
    ///
    /// When this context was built with a cancel token (orchestrated runs), the sleep ends early
    /// if the task group is cancelled, so shutdown does not wait for the full sampled duration.
    pub async fn pause(&self, base_ms: u64) {
        timing::uniform_pause_with_cancel(self.cancel_token.as_ref(), base_ms, 20).await;
    }

    /// Uniform random wait around `base_ms` with spread `variance_pct` (0–100, e.g. 20 for ±20%).
    ///
    /// Same distribution model as [`Self::pause`] but with a configurable spread. For Gaussian
    /// (human-like) delays, use [`Self::pause_human`].
    pub async fn pause_with_variance(&self, base_ms: u64, variance_pct: u32) {
        timing::uniform_pause_with_cancel(self.cancel_token.as_ref(), base_ms, variance_pct).await;
    }

    /// Gaussian (human-like) pause around `base_ms` with `variance_pct` shaping the distribution.
    ///
    /// Respects the same optional cancel token as [`Self::pause`].
    pub async fn pause_human(&self, base_ms: u64, variance_pct: u32) {
        timing::human_pause_with_cancel(self.cancel_token.as_ref(), base_ms, variance_pct).await;
    }

    /// Check if selector exists in DOM (may be hidden).
    pub async fn exists(&self, selector: &str) -> Result<bool> {
        query::exists(self.page(), selector).await
    }

    /// Check if selector is visible (displayed and not hidden).
    pub async fn visible(&self, selector: &str) -> Result<bool> {
        query::visible(self.page(), selector).await
    }

    /// Get text content of selector. Returns None if not found.
    pub async fn text(&self, selector: &str) -> Result<Option<String>> {
        query::text(self.page(), selector).await
    }

    /// Get inner HTML of selector. Returns None if not found.
    pub async fn html(&self, selector: &str) -> Result<Option<String>> {
        query::html(self.page(), selector).await
    }

    /// Get element attribute by name. Returns None if not found.
    pub async fn attr(&self, selector: &str, name: &str) -> Result<Option<String>> {
        query::attr(self.page(), selector, name).await
    }

    /// Get input/textarea value attribute. Returns None if not found.
    pub async fn value(&self, selector: &str) -> Result<Option<String>> {
        query::value(self.page(), selector).await
    }

    /// Wait for selector to exist in DOM. Returns true if found within timeout.
    pub async fn wait_for(&self, selector: &str, timeout_ms: u64) -> Result<bool> {
        query::wait_for(self.page(), selector, timeout_ms)
            .await
            .with_context(|| {
                wrapper_timeout_context(
                    "wait_for",
                    format!("selector={selector} timeout_ms={timeout_ms}"),
                )
            })
    }

    /// Wait for selector to be visible. Returns true if visible within timeout.
    pub async fn wait_for_visible(&self, selector: &str, timeout_ms: u64) -> Result<bool> {
        query::wait_for_visible(self.page(), selector, timeout_ms)
            .await
            .with_context(|| {
                wrapper_timeout_context(
                    "wait_for_visible",
                    format!("selector={selector} timeout_ms={timeout_ms}"),
                )
            })
    }

    /// Get current page URL.
    pub async fn url(&self) -> Result<String> {
        query::url(self.page()).await
    }

    /// Get page title from DOM.
    pub async fn title(&self) -> Result<String> {
        query::title(self.page()).await
    }

    /// Get viewport dimensions (width, height, `device_scale_factor`).
    pub async fn viewport(&self) -> Result<Viewport> {
        query::viewport(self.page()).await
    }

    /// Select all text in element (Ctrl+A).
    pub async fn select_all(&self, selector: &str) -> Result<()> {
        let _ = self.focus(selector).await?;

        if dom::selector_uses_accessibility_locator(selector) {
            let check_active_js = r"(() => {
                const el = document.activeElement;
                if (!el) return 'not_found';
                if (el.readOnly) return 'readonly';
                if (el.disabled) return 'disabled';
                return 'ok';
            })()";
            let status = match self.page().evaluate(check_active_js).await {
                Ok(result) => result
                    .value()
                    .and_then(|v| v.as_str())
                    .unwrap_or("check_failed")
                    .to_string(),
                Err(_) => "check_failed".to_string(),
            };
            if status == "readonly" {
                return Err(anyhow::anyhow!(
                    "[task-api] select_all: element '{selector}' is readonly"
                ));
            }
            if status == "disabled" {
                return Err(anyhow::anyhow!(
                    "[task-api] select_all: element '{selector}' is disabled"
                ));
            }
            return self.press_with_modifiers("a", &["Control"]).await;
        }

        // Phase2: Check for readonly/disabled before attempting select all
        let check_js = format!(
            r"(() => {{
                const el = document.querySelector({});
                if (!el) return 'not_found';
                if (el.readOnly) return 'readonly';
                if (el.disabled) return 'disabled';
                return 'ok';
            }})()",
            serde_json::to_string(selector)?
        );
        let status = match self.page().evaluate(check_js).await {
            Ok(result) => result
                .value()
                .and_then(|v| v.as_str())
                .unwrap_or("check_failed")
                .to_string(),
            Err(_) => "check_failed".to_string(),
        };
        if status == "readonly" {
            return Err(anyhow::anyhow!(
                "[task-api] select_all: element '{selector}' is readonly"
            ));
        }
        if status == "disabled" {
            return Err(anyhow::anyhow!(
                "[task-api] select_all: element '{selector}' is disabled"
            ));
        }

        let select_js = format!(
            r"(() => {{
                const el = document.querySelector({});
                if (!el) return 'not_found';
                if (typeof el.setSelectionRange === 'function' && typeof el.value === 'string') {{
                    el.setSelectionRange(0, el.value.length);
                    return 'selected';
                }}
                if (el.isContentEditable) {{
                    const range = document.createRange();
                    range.selectNodeContents(el);
                    const selection = window.getSelection();
                    if (!selection) return 'no_selection';
                    selection.removeAllRanges();
                    selection.addRange(range);
                    return 'selected';
                }}
                return 'unsupported';
            }})()",
            serde_json::to_string(selector)?
        );

        match self.page().evaluate(select_js).await {
            Ok(result) => match result.value().and_then(|v| v.as_str()) {
                Some("selected") => Ok(()),
                Some("not_found") => Err(anyhow::anyhow!(
                    "[task-api] select_all: element '{selector}' not found"
                )),
                Some("readonly") => Err(anyhow::anyhow!(
                    "[task-api] select_all: element '{selector}' is readonly"
                )),
                Some("disabled") => Err(anyhow::anyhow!(
                    "[task-api] select_all: element '{selector}' is disabled"
                )),
                _ => self.press_with_modifiers("a", &["Control"]).await,
            },
            Err(_) => self.press_with_modifiers("a", &["Control"]).await,
        }
    }

    /// Clear input by selecting all + pressing Backspace.
    pub async fn clear(&self, selector: &str) -> Result<()> {
        self.select_all(selector).await?;
        self.press("Backspace").await
    }

    /// Execute an interaction through the shared pipeline.
    ///
    /// This method provides a unified interface for browser interactions,
    /// ensuring consistent preflight, execution, verification, and
    /// postflight behavior across all interaction types.
    ///
    /// # Arguments
    ///
    /// * `request` - The interaction request specifying kind, selector, and options
    ///
    /// # Returns
    ///
    /// Returns an `InteractionResult` with success status, coordinates (if applicable),
    /// and error information (if failed).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use auto::runtime::task_context::{TaskContext, InteractionRequest};
    /// # async fn example(api: &TaskContext) -> anyhow::Result<()> {
    /// // Click using the pipeline
    /// let result = api.interact(InteractionRequest::click("#submit")).await?;
    /// assert!(result.is_success());
    ///
    /// // Type using the pipeline
    /// let result = api.interact(InteractionRequest::type_text("#input", "hello")).await?;
    /// assert!(result.is_success());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn interact(&self, request: InteractionRequest) -> Result<InteractionResult> {
        interaction_pipeline::execute_interaction(self, request).await
    }

    async fn verify_selector_hit(&self, selector: &str, x: f64, y: f64) -> Result<bool> {
        let selector_js = serde_json::to_string(selector)?;
        let js = format!(
            r"(() => {{
                const el = document.querySelector({selector_js});
                if (!el) return false;
                const rect = el.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) return false;
                const hit = document.elementFromPoint({x}, {y});
                if (!hit) return false;
                return el === hit || el.contains(hit) || hit.contains(el);
            }})()"
        );
        let eval = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            self.page.evaluate(js),
        )
        .await
        .map_err(|_| {
            anyhow::anyhow!(wrapper_timeout_context(
                "click_fallback_verify",
                format!("selector={selector} timeout_ms=500"),
            ))
        })??;
        Ok(eval
            .value()
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false))
    }

    async fn post_interaction_pause(&self) {
        dom_verify::post_interaction_pause(self).await;
    }

    async fn post_interaction_pause_with_budget(&self, min_budget_ms: u64) {
        let action_delay = &self.behavior_runtime.action_delay;
        let base_ms = action_delay.min_ms.clamp(120, 1_500).max(min_budget_ms);
        let variance_pct = action_delay.variance_pct.round().clamp(10.0, 60.0) as u32;
        timing::uniform_pause(base_ms, variance_pct).await;
    }

    // ============================================================================
    // Internal Pipeline Methods
    // ============================================================================
    // These methods are used by the interaction pipeline. They mirror the public
    // API but allow the pipeline to orchestrate preflight, execution, and
    // postflight consistently across all interaction types.

    /// Internal click method for pipeline use (includes learning/retry logic)
    pub(crate) async fn click_internal(&self, selector: &str) -> Result<ClickOutcome> {
        self.click(selector).await
    }

    /// Internal native click method for pipeline use
    pub(crate) async fn nativeclick_internal(&self, selector: &str) -> Result<ClickOutcome> {
        self.nativeclick(selector).await
    }

    /// Internal focus method for pipeline use
    pub(crate) async fn focus_internal(&self, selector: &str) -> Result<FocusOutcome> {
        self.focus(selector).await
    }

    /// Internal hover method for pipeline use
    pub(crate) async fn hover_internal(&self, selector: &str) -> Result<HoverOutcome> {
        self.hover(selector).await
    }

    /// Internal keyboard method for pipeline use
    pub(crate) async fn keyboard_internal(&self, selector: &str, text: &str) -> Result<()> {
        self.keyboard(selector, text).await
    }

    /// Internal `select_all` method for pipeline use
    pub(crate) async fn select_all_internal(&self, selector: &str) -> Result<()> {
        self.select_all(selector).await
    }

    /// Internal clear method for pipeline use
    pub(crate) async fn clear_internal(&self, selector: &str) -> Result<()> {
        self.clear(selector).await
    }

    /// Coordinate fallback click for pipeline use
    /// Gets element coordinates and clicks at that position directly
    pub(crate) async fn click_coordinate_fallback(&self, selector: &str) -> Result<ClickOutcome> {
        // Get element bounding rect for coordinates
        let js = format!(
            r"(() => {{
                const el = document.querySelector({});
                if (!el) return null;
                const rect = el.getBoundingClientRect();
                return {{ x: rect.left + rect.width/2, y: rect.top + rect.height/2 }};
            }})()",
            serde_json::to_string(selector)?
        );

        let eval = self.page().evaluate(js).await?;
        let coords = eval.value();

        let (x, y) = match coords {
            Some(v) => (
                v.get("x")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
                v.get("y")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
            ),
            None => return Err(anyhow::anyhow!("Element not found for coordinate fallback")),
        };

        self.click_at(x, y).await?;

        Ok(ClickOutcome {
            click: ClickStatus::Success,
            x,
            y,
            screen_x: None,
            screen_y: None,
        })
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
                (Some(x), Some(y)) => format!("({x}, {y})"),
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

#[must_use]
pub fn validate_session_data_impl(data: &crate::task::policy::SessionData) -> Vec<String> {
    validation::validate_session_data_impl(data)
}

#[cfg(test)]
pub(crate) fn validate_session_data_for_tests(
    data: &crate::task::policy::SessionData,
) -> Vec<String> {
    validate_session_data_impl(data)
}
