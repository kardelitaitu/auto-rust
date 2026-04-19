//! Task execution module.
//!
//! Provides the core task execution infrastructure:
//! - Generic task runner with retry logic
//! - Task-specific implementations (cookiebot, pageview, etc.)
//! - Error classification and result reporting
//!
//! Task authors should import `crate::prelude::*` and accept `TaskContext`.

use anyhow::Result;
use serde_json::Value;

use crate::result::{TaskErrorKind, TaskResult, TaskStatus};
use crate::prelude::TaskContext;

pub mod cookiebot;
pub mod pageview;
pub mod demo_keyboard;
pub mod demo_mouse;
pub mod twitterfollow;
pub mod twitterreply;
pub mod twitteractivity;

pub async fn perform_task(
    ctx: &TaskContext,
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

        let result = execute_single_attempt(ctx, clean_name, &payload).await;

        match result {
            Ok(()) => {
                return Ok(TaskResult::success(start.elapsed().as_millis() as u64)
                    .with_attempt(attempt, max_retries));
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
    let error_kind = TaskErrorKind::classify(&error_msg);
    let status = if matches!(error_kind, TaskErrorKind::Timeout) {
        TaskStatus::Timeout
    } else {
        TaskStatus::Failed(error_msg.clone())
    };

    Ok(TaskResult {
        status,
        attempt,
        max_retries,
        last_error: Some(error_msg),
        error_kind: Some(error_kind),
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

async fn execute_single_attempt(
    ctx: &TaskContext,
    name: &str,
    payload: &Value,
) -> Result<()> {
    match name {
        "cookiebot" => cookiebot::run(ctx, payload.clone()).await,
        "pageview" => pageview::run(ctx, payload.clone()).await,
        "demo-keyboard" => demo_keyboard::run(ctx, payload.clone()).await,
        "demo-mouse" => demo_mouse::run(ctx, payload.clone()).await,
        "twitterfollow" => twitterfollow::run(ctx, payload.clone()).await,
        "twitterreply" => twitterreply::run(ctx, payload.clone()).await,
        "twitteractivity" => twitteractivity::run(ctx, payload.clone()).await,
        _ => Err(anyhow::anyhow!("Unknown task: {name}")),
    }
}
