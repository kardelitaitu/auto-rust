//! Task execution module.
//!
//! Provides the core task execution infrastructure:
//! - Generic task runner with retry logic
//! - Task-specific implementations (cookiebot, pageview, etc.)
//! - Error classification and result reporting
//!
//! Task authors should import `crate::prelude::*` and accept `api: &TaskContext`.

use anyhow::Result;
use serde_json::Value;

use crate::prelude::TaskContext;
use crate::result::{TaskErrorKind, TaskResult, TaskStatus};

pub mod cookiebot;
pub mod demo_keyboard;
pub mod demo_mouse;
pub mod demoqa;
pub mod pageview;
pub mod task_example;
pub mod twitteractivity;
pub mod twitterdive;
pub mod twitterfollow;
pub mod twitterlike;
pub mod twitterquote;
pub mod twitterreply;
pub mod twitterretweet;
pub mod twittertest;

pub async fn perform_task(
    api: &TaskContext,
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

        let result = execute_single_attempt(api, clean_name, &payload).await;

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

async fn execute_single_attempt(api: &TaskContext, name: &str, payload: &Value) -> Result<()> {
    match name {
        "cookiebot" => cookiebot::run(api, payload.clone()).await,
        "pageview" => pageview::run(api, payload.clone()).await,
        "demo-keyboard" => demo_keyboard::run(api, payload.clone()).await,
        "demo-mouse" => demo_mouse::run(api, payload.clone()).await,
        "demoqa" => demoqa::run(api, payload.clone()).await,
        "task-example" => task_example::run(api, payload.clone()).await,
        "twitteractivity" => twitteractivity::run(api, payload.clone()).await,
        "twitterdive" => twitterdive::run(api, payload.clone()).await,
        "twitterfollow" => twitterfollow::run(api, payload.clone()).await,
        "twitterlike" => twitterlike::run(api, payload.clone()).await,
        "twitterquote" => twitterquote::run(api, payload.clone()).await,
        "twitterreply" => twitterreply::run(api, payload.clone()).await,
        "twitterretweet" => twitterretweet::run(api, payload.clone()).await,
        "twittertest" => twittertest::run(api, payload.clone()).await,
        _ => Err(anyhow::anyhow!("Unknown task: {name}")),
    }
}
