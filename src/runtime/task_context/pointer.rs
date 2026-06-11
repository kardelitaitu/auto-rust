//! Pointer and keyboard methods for TaskContext.

use super::interaction;
use super::types::RandomCursorOutcome;
use super::TaskContext;
use crate::capabilities::{dom, keyboard, mouse};
use crate::internal::page_size;
use crate::utils::{HoverOutcome, HoverStatus, NativeCursorOutcome};
use anyhow::Result;
use log::{debug, info, warn};

impl TaskContext {
    // [Moved to submodule: page_nav.rs]

    // --- Permission checking ---

    // [Moved to submodule: page_nav.rs]
    // [Moved to submodule: cookies.rs]
    // [Moved to submodule: clipboard.rs]
    // [Moved to submodule: data_files.rs]
    // [Moved to submodule: cookies.rs]
    // [Moved to submodule: http.rs]
    // [Moved to submodule: session_io.rs]
    // [Moved to submodule: session_io.rs]
    // [Moved to submodule: session_io.rs]

    // [Moved to submodule: session_io.rs]

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
        if dom::selector_uses_accessibility_locator(selector) {
            let (x, y) = dom::selector_action_point(self.page(), selector).await?;
            mouse::cursor_move_to(self.page(), x, y).await?;
            self.post_interaction_pause().await;
            return Ok(HoverOutcome {
                hover: HoverStatus::Success,
                x,
                y,
            });
        }

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

    /// Drag from one selector to another with human-like behavior.
    pub async fn drag(&self, from_selector: &str, to_selector: &str) -> Result<()> {
        if dom::selector_uses_accessibility_locator(from_selector)
            || dom::selector_uses_accessibility_locator(to_selector)
        {
            let (start_x, start_y) = dom::selector_action_point(self.page(), from_selector).await?;
            let (end_x, end_y) = dom::selector_action_point(self.page(), to_selector).await?;
            let click = &self.behavior_runtime.click;
            mouse::drag_between_points_human(
                self.page(),
                start_x,
                start_y,
                end_x,
                end_y,
                click.reaction_delay_ms,
                self.behavior_runtime.action_delay.variance_pct.round() as u32,
            )
            .await?;
            self.post_interaction_pause().await;
            return Ok(());
        }

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
        interaction::press(self.page(), key).await
    }

    /// Press key with modifiers (e.g., Ctrl+C, Shift+A).
    pub async fn press_with_modifiers(&self, key: &str, modifiers: &[&str]) -> Result<()> {
        interaction::press_with_modifiers(self.page(), key, modifiers).await
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
        info!("[task-api] keyboard {selector} -> {text}");

        // Phase2: Verify element exists and is focusable before focusing
        if !self.exists(selector).await? {
            return Err(anyhow::anyhow!(
                "[task-api] keyboard failed: element '{selector}' not found"
            ));
        }

        let _ = self.focus(selector).await?;
        let typing = &self.behavior_runtime.typing;
        keyboard::type_text_profiled(self.page(), text, typing).await?;

        // Phase2: Verify text was entered (check value after typing)
        let verification_js = if dom::selector_uses_accessibility_locator(selector) {
            r"(() => {
                const el = document.activeElement;
                if (!el) return false;
                const value = el.value || el.textContent || '';
                return String(value).length > 0;
            })()"
                .to_string()
        } else {
            format!(
                r"(() => {{
                    const el = document.querySelector({});
                    if (!el) return false;
                    const value = el.value || el.textContent || '';
                    return value.length > 0;
                }})()",
                serde_json::to_string(selector)?
            )
        };
        match self.page().evaluate(verification_js).await {
            Ok(result) => {
                let text_entered = result
                    .value()
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                if !text_entered {
                    warn!("[task-api] keyboard: text may not have been entered for '{selector}'");
                }
            }
            Err(e) => {
                debug!("[task-api] keyboard verification failed for '{selector}': {e}");
            }
        }

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
        // Phase2: Verify something is focused and is editable
        let focus_check_js = r"(() => {
            const el = document.activeElement;
            if (!el) return 'no_focus';
            if (el.readOnly) return 'readonly';
            if (el.disabled) return 'disabled';
            if (el.isContentEditable) return 'editable';
            if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') return 'input';
            return 'not_editable';
        })()";
        let status = match self.page().evaluate(focus_check_js).await {
            Ok(result) => result
                .value()
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            Err(_) => "check_failed".to_string(),
        };
        let status = status.as_str();
        match status {
            "no_focus" => {
                warn!("[task-api] type_text: no element is focused");
            }
            "readonly" => {
                return Err(anyhow::anyhow!(
                    "[task-api] type_text: focused element is readonly"
                ));
            }
            "disabled" => {
                return Err(anyhow::anyhow!(
                    "[task-api] type_text: focused element is disabled"
                ));
            }
            "not_editable" => {
                warn!("[task-api] type_text: focused element may not be editable");
            }
            _ => {}
        }

        let typing = &self.behavior_runtime.typing;
        keyboard::type_text_profiled(self.page(), text, typing).await?;
        self.post_interaction_pause().await;
        Ok(())
    }
}
