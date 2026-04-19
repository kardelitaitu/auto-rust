use chromiumoxide::Page;
use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Result;

use crate::capabilities::{clipboard, keyboard, mouse, navigation, scroll, timing};
use crate::internal::profile::{BrowserProfile, ProfileRuntime};
use crate::internal::page_size::{self, Viewport};
use crate::state::ClipboardState;

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
        navigation::goto(self.page(), url, timeout_ms).await
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

    pub async fn move_mouse_to(&self, x: f64, y: f64) -> Result<()> {
        mouse::cursor_move_to(self.page(), x, y).await
    }

    pub async fn move_mouse_fast(&self, x: f64, y: f64) -> Result<()> {
        mouse::cursor_move_to_immediate(self.page(), x, y).await
    }

    pub async fn sync_cursor_overlay(&self) -> Result<()> {
        mouse::sync_cursor_overlay(self.page()).await
    }

    pub async fn click(&self, x: f64, y: f64) -> Result<()> {
        mouse::click_at(self.page(), x, y).await
    }

    pub async fn click_fast(&self, x: f64, y: f64) -> Result<()> {
        mouse::click_at_without_move(self.page(), x, y).await
    }

    pub async fn left_click(&self, x: f64, y: f64) -> Result<()> {
        mouse::left_click_at(self.page(), x, y).await
    }

    pub async fn left_click_fast(&self, x: f64, y: f64) -> Result<()> {
        mouse::left_click_at_without_move(self.page(), x, y).await
    }

    pub async fn right_click(&self, x: f64, y: f64) -> Result<()> {
        mouse::right_click_at(self.page(), x, y).await
    }

    pub async fn right_click_fast(&self, x: f64, y: f64) -> Result<()> {
        mouse::right_click_at_without_move(self.page(), x, y).await
    }

    pub async fn press(&self, key: &str) -> Result<()> {
        keyboard::press(self.page(), key).await
    }

    pub async fn press_with_modifiers(&self, key: &str, modifiers: &[&str]) -> Result<()> {
        keyboard::press_with_modifiers(self.page(), key, modifiers).await
    }

    pub async fn type_text(&self, text: &str) -> Result<()> {
        keyboard::type_text(self.page(), text).await
    }

    pub async fn random_scroll(&self) -> Result<()> {
        scroll::random_scroll(self.page()).await
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
        scroll::back(self.page(), distance).await
    }

    pub async fn scroll_into_view(&self, selector: &str) -> Result<()> {
        scroll::scroll_into_view(self.page(), selector).await
    }

    pub async fn scroll_to_top(&self) -> Result<()> {
        scroll::scroll_to_top(self.page()).await
    }

    pub async fn scroll_to_bottom(&self) -> Result<()> {
        scroll::scroll_to_bottom(self.page()).await
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

    pub async fn pause(&self, base_ms: u64, variance_pct: u32) {
        timing::human_pause(base_ms, variance_pct).await;
    }

    pub async fn viewport(&self) -> Result<Viewport> {
        page_size::get_viewport(self.page()).await
    }
}
