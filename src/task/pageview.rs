use crate::internal::profile::{CursorBehavior, ScrollBehavior};
use crate::prelude::TaskContext;
use crate::utils::payload as payload_util;
use crate::utils::timing::duration_with_variance;
use crate::validation::task::resolve_pageview_target;
use anyhow::Result;
use log::info;
use rand::Rng;
use serde_json::Value;
use std::env;
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};

// Keep the task runtime budget aligned with the registry policy.
// Default runtime for the pageview behavior loop.
// Keep this separate from the policy timeout so timeout enforcement can be tested.
const PAGEVIEW_TASK_DURATION_MS: u64 = 300_000;
const DEFAULT_INITIAL_PAUSE_MS: u64 = 1_000;
const DEFAULT_SELECTOR_WAIT_MS: u64 = 10_000;
const DEFAULT_CURSOR_INTERVAL_MIN_MS: u64 = 2_000;
const DEFAULT_CURSOR_INTERVAL_MAX_MS: u64 = 3_000;
const DEFAULT_SCROLL_INTERVAL_MIN_MS: u64 = 1_200;
const DEFAULT_SCROLL_INTERVAL_MAX_MS: u64 = 2_400;
const DEFAULT_OVERLAY_SYNC_MS: u64 = 50;
const DEFAULT_SCROLL_READ_PAUSES: u32 = 2;
const DEFAULT_SCROLL_READ_AMOUNT: i32 = 650;
const DEFAULT_SCROLL_READ_VARIABLE_SPEED: bool = true;
const DEFAULT_SCROLL_READ_BACK_SCROLL: bool = false;
const DEFAULT_ENABLE_CURSOR: bool = true;
const DEFAULT_ENABLE_SCROLL: bool = true;
const DEFAULT_OVERLAY_TEST_MODE: bool = false;
const OVERLAY_TEST_CURSOR_INTERVAL_MIN_MS: u64 = 120;
const OVERLAY_TEST_CURSOR_INTERVAL_MAX_MS: u64 = 260;
const OVERLAY_TEST_OVERLAY_SYNC_MS: u64 = 16;

#[derive(Debug, Clone)]
struct PageviewConfig {
    duration_ms: u64,
    initial_pause_ms: u64,
    selector_wait_ms: u64,
    cursor_interval_min_ms: u64,
    cursor_interval_max_ms: u64,
    scroll_interval_min_ms: u64,
    scroll_interval_max_ms: u64,
    overlay_sync_ms: u64,
    scroll_read_pauses: u32,
    scroll_read_amount: i32,
    scroll_read_variable_speed: bool,
    scroll_read_back_scroll: bool,
    enable_cursor: bool,
    enable_scroll: bool,
    overlay_test_mode: bool,
}

impl Default for PageviewConfig {
    fn default() -> Self {
        Self {
            duration_ms: PAGEVIEW_TASK_DURATION_MS,
            initial_pause_ms: DEFAULT_INITIAL_PAUSE_MS,
            selector_wait_ms: DEFAULT_SELECTOR_WAIT_MS,
            cursor_interval_min_ms: DEFAULT_CURSOR_INTERVAL_MIN_MS,
            cursor_interval_max_ms: DEFAULT_CURSOR_INTERVAL_MAX_MS,
            scroll_interval_min_ms: DEFAULT_SCROLL_INTERVAL_MIN_MS,
            scroll_interval_max_ms: DEFAULT_SCROLL_INTERVAL_MAX_MS,
            overlay_sync_ms: DEFAULT_OVERLAY_SYNC_MS,
            scroll_read_pauses: DEFAULT_SCROLL_READ_PAUSES,
            scroll_read_amount: DEFAULT_SCROLL_READ_AMOUNT,
            scroll_read_variable_speed: DEFAULT_SCROLL_READ_VARIABLE_SPEED,
            scroll_read_back_scroll: DEFAULT_SCROLL_READ_BACK_SCROLL,
            enable_cursor: DEFAULT_ENABLE_CURSOR,
            enable_scroll: DEFAULT_ENABLE_SCROLL,
            overlay_test_mode: DEFAULT_OVERLAY_TEST_MODE,
        }
    }
}

impl PageviewConfig {
    fn from_payload(
        payload: &Value,
        cursor_behavior: CursorBehavior,
        scroll_behavior: ScrollBehavior,
    ) -> Result<Self> {
        let base_scroll_pause = scroll_behavior.pause_ms.max(100);
        let scroll_interval_min_ms = (base_scroll_pause.saturating_mul(4) / 5).max(100);
        let scroll_interval_max_ms =
            (base_scroll_pause.saturating_mul(6) / 5).max(scroll_interval_min_ms);

        let mut config = Self::default();
        config.duration_ms = payload_util::read_u64_or(payload, "duration_ms", config.duration_ms)
            .map_err(|e| anyhow::anyhow!(e))?;
        config.initial_pause_ms =
            payload_util::read_u64_or(payload, "initial_pause_ms", config.initial_pause_ms)
                .map_err(|e| anyhow::anyhow!(e))?;
        config.selector_wait_ms =
            payload_util::read_u64_or(payload, "selector_wait_ms", config.selector_wait_ms)
                .map_err(|e| anyhow::anyhow!(e))?;
        config.cursor_interval_min_ms = payload_util::read_u64_or(
            payload,
            "cursor_interval_min_ms",
            cursor_behavior.interval_min_ms,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        config.cursor_interval_max_ms = payload_util::read_u64_or(
            payload,
            "cursor_interval_max_ms",
            cursor_behavior.interval_max_ms,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        config.scroll_interval_min_ms =
            payload_util::read_u64_or(payload, "scroll_interval_min_ms", scroll_interval_min_ms)
                .map_err(|e| anyhow::anyhow!(e))?;
        config.scroll_interval_max_ms =
            payload_util::read_u64_or(payload, "scroll_interval_max_ms", scroll_interval_max_ms)
                .map_err(|e| anyhow::anyhow!(e))?;
        config.overlay_sync_ms =
            payload_util::read_u64_or(payload, "overlay_sync_ms", config.overlay_sync_ms)
                .map_err(|e| anyhow::anyhow!(e))?;
        config.scroll_read_pauses =
            payload_util::read_u32_or(payload, "scroll_read_pauses", config.scroll_read_pauses)
                .map_err(|e| anyhow::anyhow!(e))?;
        config.scroll_read_amount =
            payload_util::read_i32_or(payload, "scroll_read_amount", scroll_behavior.amount)
                .map_err(|e| anyhow::anyhow!(e))?;
        config.scroll_read_variable_speed = payload_util::read_bool_or(
            payload,
            "scroll_read_variable_speed",
            scroll_behavior.smooth,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        config.scroll_read_back_scroll = payload_util::read_bool_or(
            payload,
            "scroll_read_back_scroll",
            scroll_behavior.back_scroll,
        )
        .map_err(|e| anyhow::anyhow!(e))?;
        config.enable_cursor =
            payload_util::read_bool_or(payload, "enable_cursor", config.enable_cursor)
                .map_err(|e| anyhow::anyhow!(e))?;
        config.enable_scroll =
            payload_util::read_bool_or(payload, "enable_scroll", config.enable_scroll)
                .map_err(|e| anyhow::anyhow!(e))?;
        config.overlay_test_mode =
            payload_util::read_bool_or(payload, "overlay_test_mode", config.overlay_test_mode)
                .map_err(|e| anyhow::anyhow!(e))?;
        config.overlay_test_mode =
            read_env_bool("PAGEVIEW_OVERLAY_TEST_MODE").unwrap_or(config.overlay_test_mode);
        if config.overlay_test_mode {
            config.enable_cursor = true;
            config.cursor_interval_min_ms = OVERLAY_TEST_CURSOR_INTERVAL_MIN_MS;
            config.cursor_interval_max_ms = OVERLAY_TEST_CURSOR_INTERVAL_MAX_MS;
            config.overlay_sync_ms = OVERLAY_TEST_OVERLAY_SYNC_MS;
        }
        Ok(config)
    }

    fn duration(&self) -> Duration {
        Duration::from_millis(duration_with_variance(self.duration_ms, 20))
    }

    fn cursor_interval(&self) -> Duration {
        random_interval(self.cursor_interval_min_ms, self.cursor_interval_max_ms)
    }

    fn scroll_interval(&self) -> Duration {
        random_interval(self.scroll_interval_min_ms, self.scroll_interval_max_ms)
    }

    fn overlay_sync(&self) -> Duration {
        Duration::from_millis(self.overlay_sync_ms)
    }
}

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let url = resolve_pageview_target(&payload)?;
    let profile = api.behavior_runtime();
    let config = PageviewConfig::from_payload(&payload, profile.cursor, profile.scroll)?;
    let duration = config.duration();

    timeout(duration, run_inner(api, url, config, duration))
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "pageview exceeded task duration of {}ms",
                duration.as_millis()
            )
        })?
}

async fn run_inner(
    api: &TaskContext,
    url: String,
    config: PageviewConfig,
    duration: Duration,
) -> Result<()> {
    info!("Task started");
    info!("Visiting URL: {url}");
    if config.overlay_test_mode {
        crate::utils::mouse::set_overlay_enabled(true);
        info!("overlay_test_mode enabled: forcing session overlay on");
    }
    info!(
        "pageview config overlay_test_mode={} overlay_sync_ms={} enable_cursor={} enable_scroll={}",
        config.overlay_test_mode,
        config.overlay_sync_ms,
        config.enable_cursor,
        config.enable_scroll
    );

    api.pause(config.initial_pause_ms).await;
    api.navigate(&url, config.selector_wait_ms).await?;
    let content_selectors = ["main", "[role=\"main\"]", "article", "section"];
    match api
        .wait_for_any_visible_selector(&content_selectors, config.selector_wait_ms)
        .await
    {
        Ok(true) => info!("Visible content detected"),
        Ok(false) => info!("No target selector visible yet, continuing"),
        Err(e) => info!("Selector readiness check skipped: {e}"),
    }

    perform_pageview_behavior(api, &config, duration).await?;

    info!("Task completed successfully for: {url}");
    Ok(())
}

async fn perform_pageview_behavior(
    api: &TaskContext,
    config: &PageviewConfig,
    duration: Duration,
) -> Result<()> {
    let deadline = Instant::now() + duration;
    let cursor_interval = config.cursor_interval();
    let scroll_interval = config.scroll_interval();
    let overlay_interval = config.overlay_sync();

    let mut next_cursor = Instant::now();
    let mut next_scroll = Instant::now();
    let mut next_overlay = Instant::now();

    while Instant::now() < deadline {
        let now = Instant::now();
        let mut progress = false;

        if config.enable_cursor && now >= next_cursor {
            let _ = api.randomcursor().await;
            next_cursor = now + cursor_interval;
            progress = true;
        }

        if config.enable_scroll && now >= next_scroll {
            let _ = api
                .scroll_read(
                    config.scroll_read_pauses,
                    config.scroll_read_amount,
                    config.scroll_read_variable_speed,
                    config.scroll_read_back_scroll,
                )
                .await;
            next_scroll = now + scroll_interval;
            progress = true;
        }

        if now >= next_overlay {
            let _ = api.sync_cursor_overlay().await;
            next_overlay = now + overlay_interval;
        }

        if !progress {
            let next_cursor = if config.enable_cursor {
                next_cursor
                    .saturating_duration_since(now)
                    .min(Duration::from_millis(100))
            } else {
                Duration::from_millis(100)
            };
            let next_scroll = if config.enable_scroll {
                next_scroll
                    .saturating_duration_since(now)
                    .min(Duration::from_millis(100))
            } else {
                Duration::from_millis(100)
            };
            let next_overlay = next_overlay
                .saturating_duration_since(now)
                .min(Duration::from_millis(100));
            let next_tick = next_cursor.min(next_scroll).min(next_overlay);
            let until_deadline = deadline.saturating_duration_since(now);
            sleep(next_tick.min(until_deadline)).await;
        }
    }

    Ok(())
}

fn random_interval(min_ms: u64, max_ms: u64) -> Duration {
    let (min_ms, max_ms) = if min_ms <= max_ms {
        (min_ms, max_ms)
    } else {
        (max_ms, min_ms)
    };
    let ms = rand::thread_rng().gen_range(min_ms..=max_ms);
    Duration::from_millis(ms)
}

fn read_env_bool(key: &str) -> Option<bool> {
    let raw = env::var(key).ok()?;
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pageview_config_defaults_when_missing() {
        let payload = serde_json::json!({"url": "https://example.com"});
        let cursor_behavior = CursorBehavior {
            interval_min_ms: 1_800,
            interval_max_ms: 2_700,
        };
        let scroll_behavior = ScrollBehavior {
            amount: 650,
            pause_ms: 1_800,
            smooth: true,
            back_scroll: false,
        };
        let config =
            PageviewConfig::from_payload(&payload, cursor_behavior, scroll_behavior).unwrap();

        assert_eq!(config.duration_ms, PAGEVIEW_TASK_DURATION_MS);
        assert_eq!(config.initial_pause_ms, DEFAULT_INITIAL_PAUSE_MS);
        assert_eq!(config.selector_wait_ms, DEFAULT_SELECTOR_WAIT_MS);
        assert_eq!(config.cursor_interval_min_ms, 1_800);
        assert_eq!(config.cursor_interval_max_ms, 2_700);
        assert_eq!(config.scroll_interval_min_ms, 1_440);
        assert_eq!(config.scroll_interval_max_ms, 2_160);
        assert_eq!(config.scroll_read_amount, 650);
        assert!(config.scroll_read_variable_speed);
    }

    #[test]
    fn pageview_config_accepts_string_overrides() {
        let payload = serde_json::json!({
            "url": "https://example.com",
            "duration_ms": "90000",
            "cursor_interval_min_ms": "1500",
            "cursor_interval_max_ms": "1800",
            "scroll_interval_min_ms": "1300",
            "scroll_interval_max_ms": "1900",
            "scroll_read_pauses": "3",
            "scroll_read_amount": "450",
            "scroll_read_variable_speed": "false",
            "scroll_read_back_scroll": "true",
            "enable_cursor": "false",
            "enable_scroll": "false"
        });
        let cursor_behavior = CursorBehavior {
            interval_min_ms: 1_800,
            interval_max_ms: 2_700,
        };
        let scroll_behavior = ScrollBehavior {
            amount: 650,
            pause_ms: 1_800,
            smooth: true,
            back_scroll: false,
        };
        let config =
            PageviewConfig::from_payload(&payload, cursor_behavior, scroll_behavior).unwrap();

        assert_eq!(config.duration_ms, 90_000);
        assert_eq!(config.cursor_interval_min_ms, 1_500);
        assert_eq!(config.cursor_interval_max_ms, 1_800);
        assert_eq!(config.scroll_interval_min_ms, 1_300);
        assert_eq!(config.scroll_interval_max_ms, 1_900);
        assert_eq!(config.scroll_read_pauses, 3);
        assert_eq!(config.scroll_read_amount, 450);
        assert!(!config.scroll_read_variable_speed);
        assert!(config.scroll_read_back_scroll);
        assert!(!config.enable_cursor);
        assert!(!config.enable_scroll);
    }

    #[test]
    fn pageview_overlay_test_mode_forces_fast_cursor_sync() {
        let payload = serde_json::json!({
            "url": "https://example.com",
            "overlay_test_mode": true,
            "enable_cursor": false,
            "overlay_sync_ms": 250
        });
        let cursor_behavior = CursorBehavior {
            interval_min_ms: 1_800,
            interval_max_ms: 2_700,
        };
        let scroll_behavior = ScrollBehavior {
            amount: 650,
            pause_ms: 1_800,
            smooth: true,
            back_scroll: false,
        };
        let config =
            PageviewConfig::from_payload(&payload, cursor_behavior, scroll_behavior).unwrap();

        assert!(config.overlay_test_mode);
        assert!(config.enable_cursor);
        assert_eq!(
            config.cursor_interval_min_ms,
            OVERLAY_TEST_CURSOR_INTERVAL_MIN_MS
        );
        assert_eq!(
            config.cursor_interval_max_ms,
            OVERLAY_TEST_CURSOR_INTERVAL_MAX_MS
        );
        assert_eq!(config.overlay_sync_ms, OVERLAY_TEST_OVERLAY_SYNC_MS);
    }
}
