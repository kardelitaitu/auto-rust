use chromiumoxide::Page;
use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Result;
use log::info;

use crate::capabilities::{clipboard, keyboard, mouse, navigation, scroll, timing};
use crate::internal::profile::{BrowserProfile, ProfileRuntime};
use crate::internal::page_size::{self, Viewport};
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
            "click:success (1.0,2.0) wait_for:.next visible:true timeout:500ms"
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

    /// Clone the Arc<Page> for sharing ownership (e.g., for GhostCursor).
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
        let settle_variance = action_delay
            .variance_pct
            .round()
            .clamp(10.0, 60.0) as u32;
        timing::human_pause(settle_base, settle_variance).await;

        let settle_ms = timeout_ms.min(3_000);
        let _ = self.wait_for_load(settle_ms).await;
        self.post_interaction_pause().await;

        Ok(())
    }

    pub async fn navigate_to(&self, url: &str, timeout_ms: u64) -> Result<()> {
        self.navigate(url, timeout_ms).await
    }

    pub async fn navigate_to_light(&self, url: &str, timeout_ms: u64) -> Result<()> {
        self.navigate(url, timeout_ms).await
    }

    pub async fn navigate_to_raw(&self, url: &str, timeout_ms: u64) -> Result<()> {
        self.navigate(url, timeout_ms).await
    }

    pub async fn set_user_agent(&self, user_agent: &str) -> Result<()> {
        navigation::set_user_agent(self.page(), user_agent).await
    }

    pub async fn set_extra_http_headers(
        &self,
        headers: &BTreeMap<String, String>,
    ) -> Result<()> {
        navigation::set_extra_http_headers(self.page(), headers).await
    }

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

    pub async fn wait_for_load(&self, timeout_ms: u64) -> Result<()> {
        navigation::wait_for_load(self.page(), timeout_ms).await
    }

    pub async fn wait_for_any_visible_selector(
        &self,
        selectors: &[&str],
        timeout_ms: u64,
    ) -> Result<bool> {
        navigation::wait_for_any_visible_selector(self.page(), selectors, timeout_ms).await
    }

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

    pub async fn move_mouse_to(&self, x: f64, y: f64) -> Result<()> {
        mouse::cursor_move_to(self.page(), x, y).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    pub async fn move_mouse_fast(&self, x: f64, y: f64) -> Result<()> {
        mouse::cursor_move_to_immediate(self.page(), x, y).await
    }

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

    pub async fn sync_cursor_overlay(&self) -> Result<()> {
        mouse::sync_cursor_overlay(self.page()).await
    }

    pub async fn click_at(&self, x: f64, y: f64) -> Result<()> {
        mouse::click_at(self.page(), x, y).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    pub async fn click(&self, selector: &str) -> Result<ClickOutcome> {
        let click = &self.behavior_runtime.click;
        let outcome = mouse::click_selector_human(
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

    pub async fn click_and_wait(
        &self,
        selector: &str,
        next_selector: &str,
        timeout_ms: u64,
    ) -> Result<ClickAndWaitOutcome> {
        let click = self.click(selector).await?;
        let next_visible = self.wait_for_visible(next_selector, timeout_ms).await.unwrap_or(false);
        Ok(ClickAndWaitOutcome {
            click,
            next_selector: next_selector.to_string(),
            next_visible,
            timeout_ms,
        })
    }

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

    pub async fn left_click(&self, x: f64, y: f64) -> Result<()> {
        mouse::left_click_at(self.page(), x, y).await
    }

    pub async fn left_click_fast(&self, x: f64, y: f64) -> Result<()> {
        mouse::left_click_at_without_move(self.page(), x, y).await
    }

    pub async fn right_click_at(&self, x: f64, y: f64) -> Result<()> {
        mouse::right_click_at(self.page(), x, y).await
    }

    pub async fn right_click_fast(&self, x: f64, y: f64) -> Result<()> {
        mouse::right_click_at_without_move(self.page(), x, y).await
    }

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

    pub async fn press(&self, key: &str) -> Result<()> {
        keyboard::press(self.page(), key).await
    }

    pub async fn press_with_modifiers(&self, key: &str, modifiers: &[&str]) -> Result<()> {
        keyboard::press_with_modifiers(self.page(), key, modifiers).await
    }

    pub async fn r#type(&self, selector: &str, text: &str) -> Result<()> {
        info!("[task-api] keyboard {} -> {}", selector, text);
        let _ = self.focus(selector).await?;
        let typing = &self.behavior_runtime.typing;
        keyboard::type_text_profiled(self.page(), text, typing).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    pub async fn keyboard(&self, selector: &str, text: &str) -> Result<()> {
        self.r#type(selector, text).await
    }

    pub async fn type_into(&self, selector: &str, text: &str) -> Result<()> {
        self.r#type(selector, text).await
    }

    pub async fn type_text(&self, text: &str) -> Result<()> {
        let typing = &self.behavior_runtime.typing;
        keyboard::type_text_profiled(self.page(), text, typing).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    pub async fn random_scroll(&self) -> Result<()> {
        scroll::random_scroll(self.page()).await
    }

    pub async fn scroll_to(&self, selector: &str) -> Result<()> {
        scroll::scroll_into_view(self.page(), selector).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    pub async fn scroll_read(
        &self,
        pauses: u32,
        scroll_amount: i32,
        variable_speed: bool,
        back_scroll: bool,
    ) -> Result<()> {
        scroll::read(self.page(), pauses, scroll_amount, variable_speed, back_scroll).await
    }

    pub async fn scroll_read_to(
        &self,
        selector: &str,
        pauses: u32,
        scroll_amount: i32,
        variable_speed: bool,
        back_scroll: bool,
    ) -> Result<()> {
        scroll::scroll_read_to(self.page(), selector, pauses, scroll_amount, variable_speed, back_scroll).await
    }

    pub async fn scroll_back(&self, distance: i32) -> Result<()> {
        scroll::back(self.page(), distance).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    pub async fn scroll_into_view(&self, selector: &str) -> Result<()> {
        self.scroll_to(selector).await
    }

    pub async fn scroll_to_top(&self) -> Result<()> {
        scroll::scroll_to_top(self.page()).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    pub async fn scroll_to_bottom(&self) -> Result<()> {
        scroll::scroll_to_bottom(self.page()).await?;
        self.post_interaction_pause().await;
        Ok(())
    }

    pub async fn copy(&self) -> Result<String> {
        clipboard::copy(self.session_id(), self.page()).await
    }

    pub async fn cut(&self) -> Result<String> {
        clipboard::cut(self.session_id(), self.page()).await
    }

    pub async fn paste(&self) -> Result<String> {
        clipboard::paste_from_clipboard(self.session_id(), self.page()).await
    }

    pub async fn pause(&self, base_ms: u64) {
        timing::uniform_pause(base_ms, 20).await;
    }

    pub async fn pause_with_variance(&self, base_ms: u64, variance_pct: u32) {
        timing::human_pause(base_ms, variance_pct).await;
    }

    pub async fn exists(&self, selector: &str) -> Result<bool> {
        navigation::selector_exists(self.page(), selector).await
    }

    pub async fn visible(&self, selector: &str) -> Result<bool> {
        navigation::selector_is_visible(self.page(), selector).await
    }

    pub async fn text(&self, selector: &str) -> Result<Option<String>> {
        navigation::selector_text(self.page(), selector).await
    }

    pub async fn html(&self, selector: &str) -> Result<Option<String>> {
        navigation::selector_html(self.page(), selector).await
    }

    pub async fn attr(&self, selector: &str, name: &str) -> Result<Option<String>> {
        navigation::selector_attr(self.page(), selector, name).await
    }

    pub async fn value(&self, selector: &str) -> Result<Option<String>> {
        navigation::selector_value(self.page(), selector).await
    }

    pub async fn wait_for(&self, selector: &str, timeout_ms: u64) -> Result<bool> {
        navigation::wait_for_selector(self.page(), selector, timeout_ms).await
    }

    pub async fn wait_for_visible(&self, selector: &str, timeout_ms: u64) -> Result<bool> {
        navigation::wait_for_visible_selector(self.page(), selector, timeout_ms).await
    }

    pub async fn url(&self) -> Result<String> {
        navigation::page_url(self.page()).await
    }

    pub async fn title(&self) -> Result<String> {
        navigation::page_title(self.page()).await
    }

    pub async fn viewport(&self) -> Result<Viewport> {
        page_size::get_viewport(self.page()).await
    }

    pub async fn select_all(&self, selector: &str) -> Result<()> {
        let _ = self.focus(selector).await?;
        self.press_with_modifiers("a", &["Control"]).await
    }

    pub async fn clear(&self, selector: &str) -> Result<()> {
        self.select_all(selector).await?;
        self.press("Backspace").await
    }

    async fn post_interaction_pause(&self) {
        timing::uniform_pause(500, 40).await;
    }
}
