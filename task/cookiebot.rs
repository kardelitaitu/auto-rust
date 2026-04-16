use anyhow::Result;
use chromiumoxide::Page;
use serde_json::Value;
use log::{info, warn, error};
use std::fs;
use std::time::Instant;
use crate::utils::{navigation, scroll, timing};
use rand::seq::SliceRandom;

pub async fn run(session_id: &str, page: &Page, _payload: Value) -> Result<()> {
    // Read URLs from data/cookiebot.txt
    let mut urls = read_cookiebot_urls()?;
    if urls.is_empty() {
        warn!("[{session_id}][cookiebot] No URLs found in data/cookiebot.txt");
        return Ok(());
    }

    // Shuffle URLs randomly
    let mut rng = rand::thread_rng();
    urls.shuffle(&mut rng);

    for (i, url) in urls.iter().enumerate() {
        info!("[{session_id}][cookiebot] Processing URL {}/{}: {}", i + 1, urls.len(), url);

        // Phase 1: Navigation with timing (20s total limit)
        let nav_start = Instant::now();
        
        // Step 1: Navigate to URL (max 15s)
        if let Err(e) = navigation::goto(page, url, 15000).await {
            warn!("[{session_id}][cookiebot] Failed to navigate to {url}: {e}");
            continue;
        }

        // Step 2: Wait for load (max 5s remaining from 20s budget)
        let elapsed_ms = nav_start.elapsed().as_millis() as u64;
        let remaining_time = 20000u64.saturating_sub(elapsed_ms);
        
        if let Err(e) = navigation::wait_for_load(page, remaining_time).await {
            warn!("[{session_id}][cookiebot] Page load timeout after {}ms for {url}: {e}", remaining_time);
            // Continue anyway - page might be partially loaded
        }
        let nav_duration = nav_start.elapsed();

        info!("[{session_id}][cookiebot] {} URL loaded {} (nav: {}ms)", 
            i + 1, url, nav_duration.as_millis());

        // Phase 2: Browsing behavior with timing
        let browse_start = Instant::now();
        if let Err(e) = perform_browsing_behavior(session_id, page).await {
            error!("[{session_id}][cookiebot] Browsing behavior error: {e}");
        }
        let browse_duration = browse_start.elapsed();

        info!("[{session_id}][cookiebot] {} Completed {} (total: {}ms, nav: {}ms, browse: {}ms)", 
            i + 1, 
            url,
            (nav_duration + browse_duration).as_millis(),
            nav_duration.as_millis(),
            browse_duration.as_millis());

        // Phase 3: Pause between URLs
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
        .map_err(|e| anyhow::anyhow!("Failed to read data/cookiebot.txt: {e}"))?;

    let urls: Vec<String> = content
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    Ok(urls)
}