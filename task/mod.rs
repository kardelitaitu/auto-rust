use anyhow::Result;
use chromiumoxide::Page;
use serde_json::Value;

// Register all task modules
pub mod cookiebot;
pub mod pageview;
// Add more as you migrate tasks:
// pub mod twitter_follow;
// pub mod twitter_tweet;
// pub mod retweet;

/// Execute a task by name
/// This is the main entry point from the orchestrator
pub async fn perform_task(page: &Page, session_id: &str, name: &str, payload: Value) -> Result<()> {
    // Remove .js extension if present
    let clean_name = name.strip_suffix(".js").unwrap_or(name);

    match clean_name {
        "cookiebot" => cookiebot::run(session_id, page, payload).await,
        "pageview" => pageview::run(session_id, page, payload).await,
        // Add more mappings as you migrate tasks
        _ => Err(anyhow::anyhow!("Unknown task: {}. Add it to task/mod.rs", name)),
    }
}