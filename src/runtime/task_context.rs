use chromiumoxide::Page;
use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Result;
use log::info;

use crate::capabilities::{clipboard, keyboard, mouse, navigation, scroll, timing};
use crate::internal::page_size::{self, Viewport};
use crate::internal::profile::{BrowserProfile, ProfileRuntime};
use crate::state::ClipboardState;
use crate::utils::mouse::{ClickOutcome, CursorMovementConfig, HoverOutcome};

#[derive(Debug, Clone, Copy)]
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

#[derive(Debug, Clone)]
pub struct ClickAndWaitOutcome {
    pub click: ClickOutcome,
    pub next_selector: String,
    pub next_visible: bool,
    pub timeout_ms: u64,
}

impl ClickAndWaitOutcome {
    pub fn summary(&self) -> String {
        format!(
            "{} wait_for:{} visible:{} timeout:{}ms",
            self.click.summary(),
            self.next_selector,
            self.next_visible,
            self.timeout_ms
        )
    }
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
            },
            next_selector: ".next".into(),
            next_visible: true,
            timeout_ms: 500,
        };
        assert_eq!(
            outcome.summary(),
            "Clicked (1.0,2.0) wait_for:.next visible:true timeout:500ms"
        );
    }
}

#[derive(Clone)]
pub struct TaskContext {
    session_id: String,
    page: Arc<Page>,
    clipboard: ClipboardState,
    behavior_profile: BrowserProfile,
    behavior_runtime: ProfileRuntime,
}

impl TaskContext {
    pub fn new(
        session_id: impl Into<String>,
        page: Arc<Page>,
        behavior_profile: BrowserProfile,
        behavior_runtime: ProfileRuntime,
    ) -> Self {
        let session_id = session_id.into();
        let clipboard = ClipboardState::new(session_id.clone());
        Self {
            session_id,
            page,
            clipboard,
            behavior_profile,
            behavior_runtime,
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn page(&self) -> &Page {
        &self.page
    }

    /// Clone the `Arc<Page>` for sharing ownership with helper flows.
    pub fn page_arc(&self) -> Arc<Page> {
        self.page.clone()
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

    /// Navigate to URL with full settle pause. Adds human-like delays before/during navigation.
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

    /// Navigate with standard settle timing. Alias for `navigate()`.
    pub async fn navigate_to(&self, url: &str, timeout_ms: u64) -> Result<()> {
        self.navigate(url, timeout_ms).await
    }

    /// Navigate with minimal pause for faster execution.
    pub async fn navigate_to_light(&self, url: &str, timeout_ms: u64) -> Result<()> {
        self.navigate(url, timeout_ms).await
    }

    /// Raw navigation without timing adjustments.
    pub async fn navigate_to_raw(&self, url: &str, timeout_ms: u64) -> Result<()> {
        self.navigate(url, timeout_ms).await
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

    /// Scroll element into view, focus it, and return center coordinates + focus status.
    pub async fn focus(&self, selector: &str) -> Result<FocusOutcome> {
        scroll::scroll_into_view(self.page(), selector).await?;
        let (x, y) = page_size::get_element_center(self.page(), selector).await?;
        let focus = match navigation::focus(self.page(), selector).await {
            Ok(()) => FocusStatus::Success,
            Err(_) => FocusStatus::Failed,
        };
        self.post_interaction_pause().await;
        Ok(FocusOutcome { focus, x, y })
    }

    /// Human-like hover with configurable delay, variance, and offset.
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
        let (x, y) = page_size::random_position(&viewport, 12.0);
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

    /// Primary click method. Runs selector pipeline with 8s timeout, falls back to coordinate click on failure.
    pub async fn click(&self, selector: &str) -> Result<ClickOutcome> {
        const CLICK_TOTAL_TIMEOUT_SECS: u64 = 8;
        const CLICK_PRIMARY_TIMEOUT_SECS: u64 = 4;
        let click = &self.behavior_runtime.click;

        let click_future = async {
            match tokio::time::timeout(
                std::time::Duration::from_secs(CLICK_PRIMARY_TIMEOUT_SECS),
                mouse::click_selector_human(
                    self.page(),
                    selector,
                    click.reaction_delay_ms,
                    self.behavior_runtime.action_delay.variance_pct.round() as u32,
                    click.offset_px,
                ),
            )
            .await
            {
                Ok(Ok(outcome)) => Ok(outcome),
                Ok(Err(_)) => self.fallback_click(selector).await,
                Err(_) => self.fallback_click(selector).await,
            }
        };

        let outcome = match tokio::time::timeout(
            std::time::Duration::from_secs(CLICK_TOTAL_TIMEOUT_SECS),
            click_future,
        )
        .await
        {
            Ok(Ok(outcome)) => outcome,
            Ok(Err(err)) => return Err(err),
            Err(_) => return Err(anyhow::anyhow!("click timed out for '{}'", selector)),
        };

        self.post_interaction_pause().await;
        Ok(outcome)
    }

    async fn fallback_click(&self, selector: &str) -> Result<ClickOutcome> {
        const FALLBACK_FOCUS_TIMEOUT_SECS: u64 = 2;
        const FALLBACK_CLICK_TIMEOUT_SECS: u64 = 2;
        info!("[task-api] click fallback '{}': focus begin", selector);
        let focus = match tokio::time::timeout(
            std::time::Duration::from_secs(FALLBACK_FOCUS_TIMEOUT_SECS),
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
            std::time::Duration::from_secs(FALLBACK_CLICK_TIMEOUT_SECS),
            self.click_at(focus.x, focus.y),
        )
        .await
        {
            Ok(Ok(())) => {
                info!("[task-api] click fallback '{}': click_at ok", selector);
                Ok(ClickOutcome {
                    click: crate::utils::mouse::ClickStatus::Success,
                    x: focus.x,
                    y: focus.y,
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
        let next_visible = self
            .wait_for_visible(next_selector, timeout_ms)
            .await
            .unwrap_or(false);
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

    /// Type text into focused element with human-like keystroke timing.
    pub async fn r#type(&self, selector: &str, text: &str) -> Result<()> {
        info!("[task-api] keyboard {} -> {}", selector, text);
        let _ = self.focus(selector).await?;
        let typing = &self.behavior_runtime.typing;
        keyboard::type_text_profiled(self.page(), text, typing).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    /// Type text into element. Alias for `r#type()`.
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

    async fn post_interaction_pause(&self) {
        timing::uniform_pause(500, 40).await;
    }
}
