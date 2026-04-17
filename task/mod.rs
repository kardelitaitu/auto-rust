//! Task execution module.
//!
//! Provides the core task execution infrastructure:
//! - Generic task runner with retry logic
//! - Task-specific implementations (cookiebot, pageview, etc.)
//! - Error classification and result reporting

use anyhow::Result;
use chromiumoxide::Page;
use serde_json::Value;
use crate::result::{TaskResult, TaskStatus};

pub mod cookiebot;
pub mod pageview;
pub mod demo_keyboard;
pub mod demo_mouse;
// pub mod twitteractivity;

use crate::utils::block_heavy_resources_for_cookiebot;

pub async fn perform_task(
    page: &Page, 
    session_id: &str, 
    name: &str, 
    payload: Value,
    max_retries: u32,
) -> Result<TaskResult> {
    let start = std::time::Instant::now();
    let clean_name = name.strip_suffix(".js").unwrap_or(name);
    let mut attempt = 0;
    let mut last_error = None;

    while attempt < max_retries.max(1) {
        attempt += 1;
        
        let result = execute_single_attempt(page, session_id, clean_name, &payload).await;
        
        match result {
            Ok(()) => {
                return Ok(TaskResult::success(start.elapsed().as_millis() as u64)
                    .with_retry(attempt, max_retries, String::new()));
            }
            Err(e) => {
                last_error = Some(e.to_string());
                if attempt < max_retries {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    let error_msg = last_error.unwrap_or_else(|| "Unknown error".to_string());
    let status = if error_msg.contains("timeout") || error_msg.contains("deadline") {
        TaskStatus::Timeout
    } else {
        TaskStatus::Failed(error_msg.clone())
    };

    Ok(TaskResult {
        status,
        attempt,
        max_retries,
        last_error: Some(error_msg),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

async fn execute_single_attempt(
    page: &Page, 
    session_id: &str, 
    name: &str, 
    payload: &Value,
) -> Result<()> {
    if name == "cookiebot" {
        block_heavy_resources_for_cookiebot(page).await?;
    }

    match name {
        "cookiebot" => cookiebot::run(session_id, page, payload.clone()).await,
        "pageview" => pageview::run(session_id, page, payload.clone()).await,
        "demo-keyboard" => demo_keyboard::run(session_id, page, payload.clone()).await,
        "demo-mouse" => demo_mouse::run(session_id, page, payload.clone()).await,
        _ => Err(anyhow::anyhow!("Unknown task: {name}")),
    }
}
