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

pub const TASK_NAMES: &[&str] = &[
    "cookiebot",
    "pageview",
    "demo-keyboard",
    "demo-mouse",
    "demoqa",
    "task-example",
    "twitteractivity",
    "twitterdive",
    "twitterfollow",
    "twitterlike",
    "twitterquote",
    "twitterreply",
    "twitterretweet",
    "twittertest",
];

pub fn normalize_task_name(name: &str) -> &str {
    name.strip_suffix(".js").unwrap_or(name)
}

pub fn is_known_task(name: &str) -> bool {
    let clean_name = normalize_task_name(name);
    TASK_NAMES.contains(&clean_name)
}

pub fn known_task_names() -> &'static [&'static str] {
    TASK_NAMES
}

pub async fn perform_task(
    api: &TaskContext,
    name: &str,
    payload: Value,
    _max_retries: u32,
    config: &crate::config::Config,
) -> Result<TaskResult> {
    let start = std::time::Instant::now();
    let clean_name = normalize_task_name(name);

    let result = execute_single_attempt(api, clean_name, &payload, config).await;

    match result {
        Ok(()) => Ok(TaskResult::success(start.elapsed().as_millis() as u64)),
        Err(e) => {
            let error_msg = e.to_string();
            let error_kind = TaskErrorKind::classify(&error_msg);
            let status = if matches!(error_kind, TaskErrorKind::Timeout) {
                TaskStatus::Timeout
            } else {
                TaskStatus::Failed(error_msg.clone())
            };

            Ok(TaskResult {
                status,
                attempt: 1,
                max_retries: 0,
                last_error: Some(error_msg),
                error_kind: Some(error_kind),
                duration_ms: start.elapsed().as_millis() as u64,
            })
        }
    }
}

async fn execute_single_attempt(api: &TaskContext, name: &str, payload: &Value, config: &crate::config::Config) -> Result<()> {
    match name {
        "cookiebot" => cookiebot::run(api, payload.clone())
            .await
            .map_err(|e| anyhow::anyhow!(e)),
        "pageview" => pageview::run(api, payload.clone()).await,
        "demo-keyboard" => demo_keyboard::run(api, payload.clone()).await,
        "demo-mouse" => demo_mouse::run(api, payload.clone()).await,
        "demoqa" => demoqa::run(api, payload.clone()).await,
        "task-example" => task_example::run(api, payload.clone()).await,
        "twitteractivity" => twitteractivity::run(api, payload.clone(), config).await,
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
