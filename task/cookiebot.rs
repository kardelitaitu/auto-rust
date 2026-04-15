use anyhow::Result;
use chromiumoxide::Page;
use serde_json::Value;
use log::{info, warn, error};
use std::fs;
use crate::utils::{navigation, scroll, timing};
use rand::seq::SliceRandom;

pub async fn run(session_id: &str, page: &Page, _payload: Value) -> Result<()> {
    // Read URLs from data/cookiebot.txt
    let mut urls = read_cookiebot_urls()?;
    if urls.is_empty() {
        warn!("[{}][cookiebot] No URLs found in data/cookiebot.txt", session_id);
        return Ok(());
    }

    // Shuffle URLs randomly
    let mut rng = rand::thread_rng();
    urls.shuffle(&mut rng);

    for (i, url) in urls.iter().enumerate() {
        // Navigate to URL
        if let Err(e) = navigation::goto(page, url, 30000).await {
            warn!("[{}][cookiebot] Failed to navigate to {}: {}", session_id, url, e);
            continue;
        }

        // Wait for page load
        if let Err(e) = navigation::wait_for_load(page, 10000).await {
            warn!("[{}][cookiebot] Failed to wait for page load {}: {}", session_id, url, e);
            continue;
        }

        info!("{}/{} URL loaded {}", i + 1, urls.len(), url);

        // Human-like browsing behavior
        if let Err(e) = perform_browsing_behavior(session_id, page).await {
            error!("[{}][cookiebot] Browsing behavior error: {}", session_id, e);
        }

        // Pause between URLs
        timing::human_pause(3000, 50).await;
    }

    Ok(())
}

async fn perform_browsing_behavior(_session_id: &str, page: &Page) -> Result<()> {
    // Random scroll to simulate reading
    scroll::random_scroll(page).await?;

    // Pause to simulate reading time
    timing::human_pause(5000, 30).await;

    // Maybe scroll to bottom and back to top
    if rand::random::<bool>() {
        scroll::scroll_to_bottom(page).await?;
        timing::human_pause(2000, 40).await;
        scroll::scroll_to_top(page).await?;
    }

    // Final pause before moving to next URL
    timing::human_pause(2000, 50).await;

    Ok(())
}

fn read_cookiebot_urls() -> Result<Vec<String>> {
    let content = fs::read_to_string("data/cookiebot.txt")
        .map_err(|e| anyhow::anyhow!("Failed to read data/cookiebot.txt: {}", e))?;

    let urls: Vec<String> = content
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    Ok(urls)
}