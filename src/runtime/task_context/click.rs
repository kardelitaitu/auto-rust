//! Click pipeline methods for TaskContext.

use super::click_learning::{ClickAdaptation, ClickTimingContext};

/// Total timeout for the entire click operation (primary + retries + fallback).
pub(crate) const CLICK_TOTAL_TIMEOUT_SECS: u64 = 12;

/// Maximum number of primary click attempts before fallback.
pub(crate) const CLICK_MAX_ATTEMPTS: u32 = 3;

/// Compute the backoff delay between click retry attempts.
///
/// Formula: `(150 + attempt * 180) + extra_stability_wait_ms / 2`, clamped to [100, 1000].
/// The linear increase gives progressive backoff: 330ms, 510ms, 690ms for attempts 1..=3
/// with zero extra stability wait.
#[must_use]
pub(crate) fn compute_click_retry_backoff(attempt: u32, extra_stability_wait_ms: u64) -> u64 {
    (150 + (u64::from(attempt) * 180))
        .saturating_add(extra_stability_wait_ms / 2)
        .clamp(100, 1_000)
}

/// Compute the reaction delay for a specific click attempt.
///
/// Each subsequent attempt adds 18% more delay to give more settling time.
/// Attempt 1: `reaction_delay_ms * 1.0`
/// Attempt 2: `reaction_delay_ms * 1.18`
/// Attempt 3: `reaction_delay_ms * 1.36`
#[must_use]
pub(crate) fn compute_primary_click_attempt_delay(reaction_delay_ms: u64, attempt: u32) -> u64 {
    (reaction_delay_ms as f64 * (1.0 + (f64::from(attempt.saturating_sub(1)) * 0.18))).round()
        as u64
}

use super::types::{ClickAndWaitOutcome, WaitForVisibleStatus};
use super::{nativeclick_public_log_line, wrapper_timeout_context, TaskContext};
use crate::capabilities::{dom, mouse, timing};
use crate::logger::scoped_log_context;
use crate::metrics::{
    RUN_COUNTER_CLICK_ATTEMPTED, RUN_COUNTER_CLICK_FALLBACK_HIT,
    RUN_COUNTER_CLICK_STRICT_VERIFY_FAILED, RUN_COUNTER_CLICK_SUCCESS,
};
use crate::utils::{ClickOutcome, ClickStatus, CursorMovementConfig};
use anyhow::{Context, Result};
use log::{debug, info, warn};
use std::time::Duration;

impl TaskContext {
    async fn record_click_learning(&self, selector: &str, success: bool) -> Result<()> {
        let mut engine = self.learning_engine.lock().await;
        engine.record(selector, success)?;
        Ok(())
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
    #[allow(clippy::cast_precision_loss)]
    pub async fn click(&self, selector: &str) -> Result<ClickOutcome> {
        let click = &self.behavior_runtime.click;
        self.increment_run_counter(RUN_COUNTER_CLICK_ATTEMPTED, 1);
        if dom::selector_uses_accessibility_locator(selector) {
            let (x, y) = dom::selector_action_point(self.page(), selector).await?;
            mouse::left_click_at(self.page(), x, y).await?;
            let outcome = ClickOutcome {
                click: ClickStatus::Success,
                x,
                y,
                screen_x: None,
                screen_y: None,
            };
            self.increment_run_counter(RUN_COUNTER_CLICK_SUCCESS, 1);
            self.post_interaction_pause().await;
            return Ok(outcome);
        }
        let default_url = String::new();
        let observed_url = self.url().await.unwrap_or(default_url);
        let base_variance = self.behavior_runtime.action_delay.variance_pct.round() as u32;

        let (timing_profile, adaptation, fatigue, recent_success_rate) = {
            let engine = self.learning_engine.lock().await;
            let timing_context = ClickTimingContext::from_observation(
                &observed_url,
                selector,
                engine.interaction_count(),
                engine.recent_success_rate(),
            );
            let adaptation = engine.adaptation_for(selector, &timing_context);
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
                let attempt_delay =
                    compute_primary_click_attempt_delay(timing_profile.reaction_delay_ms, attempt);
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
                            let backoff_ms = compute_click_retry_backoff(
                                attempt,
                                adaptation.extra_stability_wait_ms,
                            );
                            timing::uniform_pause(backoff_ms, 30).await;
                        }
                    }
                }
            }

            if adaptation.prefer_coordinate_fallback {
                warn!(
                    "[task-api] click '{selector}' entering coordinate fallback after retry exhaustion"
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
            tokio::time::timeout(Duration::from_secs(CLICK_TOTAL_TIMEOUT_SECS), click_future)
                .await
                .with_context(|| {
                    wrapper_timeout_context(
                        "click_total",
                        format!("selector={selector} timeout_secs={CLICK_TOTAL_TIMEOUT_SECS}"),
                    )
                })?;
        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(err) => {
                if let Err(persist_err) = self.record_click_learning(selector, false).await {
                    warn!("[task-api] click learning persistence failed: {persist_err}");
                }
                return Err(err);
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
                    warn!("[task-api] click learning persistence failed: {persist_err}");
                }
                self.increment_run_counter(RUN_COUNTER_CLICK_FALLBACK_HIT, 1);
                match self
                    .fallback_click_with_adaptation(selector, &adaptation)
                    .await
                {
                    Ok(fallback_outcome) => {
                        if let Err(persist_err) = self.record_click_learning(selector, true).await {
                            warn!("[task-api] click learning persistence failed: {persist_err}");
                        }
                        self.increment_run_counter(RUN_COUNTER_CLICK_SUCCESS, 1);
                        self.post_interaction_pause_with_budget(timing_profile.post_click_pause_ms)
                            .await;
                        return Ok(fallback_outcome);
                    }
                    Err(err) => {
                        return Err(anyhow::anyhow!(
                            "[task-api] strict click verification failed for '{selector}': {err}"
                        ));
                    }
                }
            }
        }

        {
            if let Err(persist_err) = self.record_click_learning(selector, true).await {
                warn!("[task-api] click learning persistence failed: {persist_err}");
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
            Err(_) => Err(anyhow::anyhow!(wrapper_timeout_context(
                "click_primary",
                format!("selector={selector} timeout_ms={timeout_ms}"),
            ))),
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

        info!("[task-api] click fallback '{selector}': focus begin");
        let focus = match tokio::time::timeout(
            Duration::from_secs(FALLBACK_FOCUS_TIMEOUT_SECS),
            self.focus(selector),
        )
        .await
        {
            Ok(Ok(focus)) => focus,
            Ok(Err(err)) => {
                return Err(anyhow::anyhow!(
                    "[task-api] fallback focus failed for '{selector}': {err}"
                ));
            }
            Err(_) => {
                return Err(anyhow::anyhow!(wrapper_timeout_context(
                    "click_fallback_focus",
                    format!("selector={selector} timeout_secs={FALLBACK_FOCUS_TIMEOUT_SECS}"),
                )));
            }
        };
        info!(
            "[task-api] click fallback '{}': focus ok at ({:.1},{:.1})",
            selector, focus.x, focus.y
        );

        info!("[task-api] click fallback '{selector}': click_at begin");
        match tokio::time::timeout(
            Duration::from_secs(FALLBACK_CLICK_TIMEOUT_SECS),
            self.click_at(focus.x, focus.y),
        )
        .await
        {
            Ok(Ok(())) => {
                info!("[task-api] click fallback '{selector}': click_at ok");
                let verified = self.verify_selector_hit(selector, focus.x, focus.y).await?;
                if adaptation.require_strict_verification && !verified {
                    return Err(anyhow::anyhow!(
                        "[task-api] fallback click target verification failed for '{selector}'"
                    ));
                }
                if !verified {
                    warn!("[task-api] fallback click verification inconclusive for '{selector}'");
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
                "[task-api] fallback click_at failed for '{selector}': {err}"
            )),
            Err(_) => Err(anyhow::anyhow!(wrapper_timeout_context(
                "click_fallback_click",
                format!("selector={selector} timeout_secs={FALLBACK_CLICK_TIMEOUT_SECS}"),
            ))),
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
        if dom::selector_uses_accessibility_locator(selector) {
            let (x, y) = dom::selector_action_point(self.page(), selector).await?;
            mouse::left_click_at(self.page(), x, y).await?;
            timing::human_pause(40, 20).await;
            mouse::left_click_at(self.page(), x, y).await?;
            let outcome = ClickOutcome {
                click: ClickStatus::Success,
                x,
                y,
                screen_x: None,
                screen_y: None,
            };
            self.post_interaction_pause().await;
            return Ok(outcome);
        }

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
        if dom::selector_uses_accessibility_locator(selector) {
            let (x, y) = dom::selector_action_point(self.page(), selector).await?;
            mouse::middle_click_at(self.page(), x, y).await?;
            let outcome = ClickOutcome {
                click: ClickStatus::Success,
                x,
                y,
                screen_x: None,
                screen_y: None,
            };
            self.post_interaction_pause().await;
            return Ok(outcome);
        }

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
        if dom::selector_uses_accessibility_locator(selector) {
            return Err(anyhow::anyhow!(
                "locator_unsupported: operation='nativeclick' requires css selector"
            ));
        }

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
                warn!("[task-api] nativeclick failed selector={selector} error={err}");
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
                    "[task-api] nativeclick session={session_id} selector={selector} screen_point=({screen_x}, {screen_y})"
                );
            } else {
                debug!(
                    "[task-api] nativeclick session={session_id} selector={selector} screen_point=(unknown)"
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
        if dom::selector_uses_accessibility_locator(selector) {
            let (x, y) = dom::selector_action_point(self.page(), selector).await?;
            mouse::right_click_at(self.page(), x, y).await?;
            let outcome = ClickOutcome {
                click: ClickStatus::Success,
                x,
                y,
                screen_x: None,
                screen_y: None,
            };
            self.post_interaction_pause().await;
            return Ok(outcome);
        }

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
}

#[cfg(test)]
mod tests {
    use super::{
        compute_click_retry_backoff, compute_primary_click_attempt_delay, CLICK_MAX_ATTEMPTS,
        CLICK_TOTAL_TIMEOUT_SECS,
    };

    // ========================================================================
    // Constant Tests
    // ========================================================================

    #[test]
    fn test_click_constants() {
        assert_eq!(CLICK_TOTAL_TIMEOUT_SECS, 12);
        assert_eq!(CLICK_MAX_ATTEMPTS, 3);
    }

    // ========================================================================
    // compute_click_retry_backoff Tests
    // ========================================================================

    #[test]
    fn test_backoff_attempt_1_no_extra_wait() {
        let backoff = compute_click_retry_backoff(1, 0);
        // (150 + 1*180) + 0/2 = 330, clamped [100, 1000] = 330
        assert_eq!(backoff, 330);
    }

    #[test]
    fn test_backoff_attempt_2_no_extra_wait() {
        let backoff = compute_click_retry_backoff(2, 0);
        // (150 + 2*180) + 0/2 = 510, clamped [100, 1000] = 510
        assert_eq!(backoff, 510);
    }

    #[test]
    fn test_backoff_attempt_3_no_extra_wait() {
        let backoff = compute_click_retry_backoff(3, 0);
        // (150 + 3*180) + 0/2 = 690, clamped [100, 1000] = 690
        assert_eq!(backoff, 690);
    }

    #[test]
    fn test_backoff_with_extra_stability_wait() {
        let backoff = compute_click_retry_backoff(1, 200);
        // (150 + 1*180) + 200/2 = 430, clamped [100, 1000] = 430
        assert_eq!(backoff, 430);
    }

    #[test]
    fn test_backoff_large_stability_wait_capped() {
        let backoff = compute_click_retry_backoff(1, 5000);
        // (150 + 1*180) + 5000/2 = 2830, clamped [100, 1000] = 1000
        assert_eq!(backoff, 1000);
    }

    #[test]
    fn test_backoff_attempt_0_returns_clamped() {
        let backoff = compute_click_retry_backoff(0, 0);
        // (150 + 0*180) + 0/2 = 150, clamped [100, 1000] = 150
        assert_eq!(backoff, 150);
    }

    #[test]
    fn test_backoff_minimum_clamp() {
        // Use backoff_min to verify clamp floor (but formula always >= 150 for attempt >= 0)
        let backoff = compute_click_retry_backoff(0, 0);
        assert!(backoff >= 100);
    }

    // ========================================================================
    // compute_primary_click_attempt_delay Tests
    // ========================================================================

    #[test]
    fn test_attempt_delay_attempt_1_same_as_base() {
        let delay = compute_primary_click_attempt_delay(300, 1);
        // 300 * (1.0 + 0 * 0.18) = 300.0, rounded = 300
        assert_eq!(delay, 300);
    }

    #[test]
    fn test_attempt_delay_attempt_2_increased() {
        let delay = compute_primary_click_attempt_delay(300, 2);
        // 300 * (1.0 + 1 * 0.18) = 354.0, rounded = 354
        assert_eq!(delay, 354);
    }

    #[test]
    fn test_attempt_delay_attempt_3_further_increased() {
        let delay = compute_primary_click_attempt_delay(300, 3);
        // 300 * (1.0 + 2 * 0.18) = 408.0, rounded = 408
        assert_eq!(delay, 408);
    }

    #[test]
    fn test_attempt_delay_zero_base() {
        let delay = compute_primary_click_attempt_delay(0, 1);
        assert_eq!(delay, 0);
    }

    #[test]
    fn test_attempt_delay_attempt_0_treated_as_attempt_1() {
        // attempt 0: saturating_sub(1) = 0, so same as attempt 1
        let delay = compute_primary_click_attempt_delay(300, 0);
        assert_eq!(delay, 300);
    }

    #[test]
    fn test_attempt_delay_rounding() {
        // 100 * (1.0 + 1 * 0.18) = 118.0, rounded = 118
        let delay = compute_primary_click_attempt_delay(100, 2);
        assert_eq!(delay, 118);

        // 100 * (1.0 + 2 * 0.18) = 136.0, rounded = 136
        let delay = compute_primary_click_attempt_delay(100, 3);
        assert_eq!(delay, 136);
    }

    #[test]
    fn test_attempt_delay_larger_base() {
        let delay = compute_primary_click_attempt_delay(500, 2);
        // 500 * 1.18 = 590.0
        assert_eq!(delay, 590);
    }

    #[test]
    fn test_attempt_delay_high_attempt_still_reasonable() {
        let delay = compute_primary_click_attempt_delay(200, 10);
        // 200 * (1.0 + 9 * 0.18) = 200 * 2.62 = 524.0, rounded = 524
        assert_eq!(delay, 524);
    }
}
