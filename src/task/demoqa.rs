//! DemoQA text box demo task.
//!
//! Writes a fixed sample record to the DemoQA text box page and verifies the rendered output.

use anyhow::{bail, Result};
use log::{debug, info, warn};
use serde_json::Value;
use std::time::Duration;
use tokio::time::timeout;

use crate::capabilities::mouse;
use crate::logger::scoped_log_context;
use crate::prelude::TaskContext;
use crate::utils::timing::duration_with_variance;

const DEMO_URL: &str = "https://demoqa.com/text-box";
const DEMO_FULL_NAME: &str = "Demo QA";
const DEMO_EMAIL: &str = "demoqa@example.com";
const DEMO_CURRENT_ADDRESS: &str = "123 Demo Street, Demo City";
const DEMO_PERMANENT_ADDRESS: &str = "456 Demo Avenue, Demo Town";
const SHOW_CURSOR_OVERLAY: bool = true;
pub const DEFAULT_DEMOQA_TASK_DURATION_MS: u64 = 60_000;

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    let duration_ms = task_duration_ms();
    timeout(Duration::from_millis(duration_ms), run_inner(api, payload))
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "[demoqa] Task exceeded duration budget of {}ms",
                duration_ms
            )
        })?
}

fn task_duration_ms() -> u64 {
    duration_with_variance(DEFAULT_DEMOQA_TASK_DURATION_MS, 20)
}

async fn run_inner(api: &TaskContext, payload: Value) -> Result<()> {
    info!("Task started");

    let config = DemoQaConfig::from_payload(&payload)?;
    info!("Opening: {}", config.url);
    info!("Writing demo content:");
    info!("- Full Name: {}", config.full_name);
    info!("- Email: {}", config.email);
    info!("- Current Address: {}", config.current_address);
    info!("- Permanent Address: {}", config.permanent_address);
    info!("Task API demo: focus -> keyboard -> nativeclick -> inspect");

    mouse::set_overlay_enabled(SHOW_CURSOR_OVERLAY);

    api.navigate(&config.url, 30_000).await?;
    if SHOW_CURSOR_OVERLAY {
        api.sync_cursor_overlay().await?;
    }

    if let Err(e) = api.wait_for_visible("#userName", 15_000).await {
        warn!("DemoQA text box did not become visible in time: {}", e);
    }

    let title = api.title().await.unwrap_or_else(|_| "unknown".to_string());
    info!("Title: {}", title);
    api.pause(1500).await;

    fill_text_field(api, "#userName", &config.full_name).await?;
    fill_text_field(api, "#userEmail", &config.email).await?;
    fill_text_field(api, "#currentAddress", &config.current_address).await?;
    fill_text_field(api, "#permanentAddress", &config.permanent_address).await?;

    let user_name = api.value("#userName").await?.unwrap_or_default();
    let user_email = api.value("#userEmail").await?.unwrap_or_default();
    let current_address = api.value("#currentAddress").await?.unwrap_or_default();
    let permanent_address = api.value("#permanentAddress").await?.unwrap_or_default();

    info!("userName value: {}", user_name);
    info!("userEmail value: {}", user_email);
    info!("currentAddress value: {}", current_address);
    info!("permanentAddress value: {}", permanent_address);

    info!("Clicking submit button");
    let session_id = api.session_id().to_string();
    {
        let mut ctx = crate::logger::get_log_context();
        ctx.session_id = Some(session_id.clone());
        let _guard = scoped_log_context(ctx);
        info!(
            "[task-api] submit nativeclick session={} selector=#submit phase=before",
            session_id
        );
    }
    info!("Submit nativeclick phase=before selector=#submit");
    warm_nativecursor(api).await?;
    let submit_click = match timeout(Duration::from_secs(12), api.nativeclick("#submit")).await {
        Ok(Ok(outcome)) => outcome,
        Ok(Err(e)) => {
            warn!("Submit click failed: {}", e);
            return Err(e);
        }
        Err(_) => {
            let e = anyhow::anyhow!("Submit click timed out after 12s");
            warn!("{}", e);
            return Err(e);
        }
    };
    if let (Some(screen_x), Some(screen_y)) = (submit_click.screen_x, submit_click.screen_y) {
        info!(
            "Submit nativeclick screen point: ({}, {}) phase=after",
            screen_x, screen_y
        );
    }
    {
        let mut ctx = crate::logger::get_log_context();
        ctx.session_id = Some(session_id.clone());
        let _guard = scoped_log_context(ctx);
        info!(
            "[task-api] submit nativeclick session={} selector=#submit point=({:.1},{:.1}) phase=after",
            session_id,
            submit_click.x,
            submit_click.y
        );
    }
    api.pause(7500).await;
    let output_exists_before_wait = api.exists("#output").await?;
    info!(
        "Output precheck before wait: exists={}",
        output_exists_before_wait
    );
    info!("Waiting for output panel");
    if !api.wait_for_visible("#output", 15_000).await? {
        let output_exists_after_wait = api.exists("#output").await?;
        let output_visible_after_wait = api.visible("#output").await?;
        warn!(
            "DemoQA output wait failed: exists={} visible={}",
            output_exists_after_wait, output_visible_after_wait
        );
        if !output_exists_after_wait {
            bail!("DemoQA output element did not exist after submit");
        }
        bail!("DemoQA output did not become visible after submit");
    }

    assert_contains(
        "name",
        api.text("#output #name").await?,
        &format!("Name:{}", config.full_name),
    )?;
    assert_contains(
        "email",
        api.text("#output #email").await?,
        &format!("Email:{}", config.email),
    )?;
    assert_contains(
        "current address",
        api.text("#output #currentAddress").await?,
        &config.current_address,
    )?;
    assert_contains(
        "permanent address",
        api.text("#output #permanentAddress").await?,
        &config.permanent_address,
    )?;

    info!("Task completed");
    Ok(())
}

async fn fill_text_field(api: &TaskContext, selector: &str, value: &str) -> Result<()> {
    warm_nativecursor(api).await?;
    let _click = api.nativeclick(selector).await?;
    if SHOW_CURSOR_OVERLAY {
        api.sync_cursor_overlay().await?;
    }
    api.clear(selector).await?;
    api.keyboard(selector, value).await?;
    Ok(())
}

async fn warm_nativecursor(api: &TaskContext) -> Result<()> {
    let _cursor = match api.nativecursor_query("button").await {
        Ok(outcome) => outcome,
        Err(err) => {
            debug!(
                "nativecursor button warm-up unavailable, falling back to any visible element: {}",
                err
            );
            api.nativecursor().await?
        }
    };
    Ok(())
}

fn assert_contains(label: &str, actual: Option<String>, expected: &str) -> Result<()> {
    let actual = actual.unwrap_or_default();
    if !actual.contains(expected) {
        bail!(
            "DemoQA {} mismatch: expected '{}' in '{}';",
            label,
            expected,
            actual
        );
    }
    info!("Verified {}: {}", label, actual);
    Ok(())
}

struct DemoQaConfig {
    url: String,
    full_name: String,
    email: String,
    current_address: String,
    permanent_address: String,
}

impl DemoQaConfig {
    fn from_payload(payload: &Value) -> Result<Self> {
        Ok(Self {
            url: extract_url_from_payload(payload)?,
            full_name: read_string(payload, "full_name", DEMO_FULL_NAME),
            email: read_string(payload, "email", DEMO_EMAIL),
            current_address: read_string(payload, "current_address", DEMO_CURRENT_ADDRESS),
            permanent_address: read_string(payload, "permanent_address", DEMO_PERMANENT_ADDRESS),
        })
    }
}

fn extract_url_from_payload(payload: &Value) -> Result<String> {
    if let Some(value) = payload.get("url") {
        if let Some(url_str) = value.as_str() {
            return Ok(url_str.to_string());
        }
    }

    if let Some(value) = payload.get("value") {
        if let Some(url_str) = value.as_str() {
            return Ok(url_str.to_string());
        }
    }

    // Check for default_url in payload
    if let Some(default_url) = payload.get("default_url") {
        if let Some(url_str) = default_url.as_str() {
            return Ok(url_str.to_string());
        }
    }

    Ok(DEMO_URL.to_string())
}

fn read_string(payload: &Value, key: &str, default: &str) -> String {
    payload
        .get(key)
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .unwrap_or_else(|| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::{task_duration_ms, DemoQaConfig, DEMO_EMAIL, DEMO_FULL_NAME, DEMO_URL};
    use serde_json::json;

    #[test]
    fn demoqa_uses_defaults() {
        let config = DemoQaConfig::from_payload(&json!({})).unwrap();
        assert_eq!(config.url, DEMO_URL);
        assert_eq!(config.full_name, DEMO_FULL_NAME);
        assert_eq!(config.email, DEMO_EMAIL);
    }

    #[test]
    fn demoqa_accepts_string_overrides() {
        let config = DemoQaConfig::from_payload(&json!({
            "url": "https://demoqa.com/text-box",
            "full_name": "Ada Lovelace",
            "email": "ada@example.com",
            "current_address": "London",
            "permanent_address": "Kent",
        }))
        .unwrap();

        assert_eq!(config.full_name, "Ada Lovelace");
        assert_eq!(config.email, "ada@example.com");
        assert_eq!(config.current_address, "London");
        assert_eq!(config.permanent_address, "Kent");
    }

    #[test]
    fn task_duration_stays_within_bounds() {
        let duration_ms = task_duration_ms();
        assert!((48_000..=72_000).contains(&duration_ms));
    }
}
