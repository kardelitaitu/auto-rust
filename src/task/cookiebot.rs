use crate::error::{OrchestratorError, Result, TaskError};
use crate::internal::blockmedia;
use crate::prelude::TaskContext;
use log::{error, info, warn};
use rand::seq::SliceRandom;
use serde_json::Value;
use std::fs;
use std::time::Instant;

pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    blockmedia::block_heavy_resources_for_cookiebot(api.page()).await?;

    // Get data file path from payload, default to data/cookiebot.txt
    let data_file = payload
        .get("data_file")
        .and_then(|v| v.as_str())
        .unwrap_or("data/cookiebot.txt");

    // Read URLs from data file
    let mut urls = read_cookiebot_urls(data_file)?;
    if urls.is_empty() {
        warn!("No URLs found in data/cookiebot.txt");
        return Ok(());
    }

    // Shuffle URLs randomly
    let mut rng = rand::thread_rng();
    urls.shuffle(&mut rng);

    for (i, url) in urls.iter().enumerate() {
        info!("Processing URL {}/{}: {}", i + 1, urls.len(), url);

        // Phase 1: Navigation with timing (20s total limit)
        let nav_start = Instant::now();

        // Step 1: Navigate to URL (max 15s)
        if let Err(e) = api.navigate(url, 15000).await {
            warn!("Failed to navigate to {}: {}", url, e);
            continue;
        }

        // Step 2: Wait for load (max 5s remaining from 20s budget)
        let elapsed_ms = nav_start.elapsed().as_millis() as u64;
        let remaining_time = 20000u64.saturating_sub(elapsed_ms);

        if let Err(e) = api.wait_for_load(remaining_time).await {
            warn!(
                "Page load timeout after {}ms for {}: {}",
                remaining_time, url, e
            );
            // Continue anyway - page might be partially loaded
        }
        let nav_duration = nav_start.elapsed();

        info!(
            "{} URL loaded {} (nav: {}ms)",
            i + 1,
            url,
            nav_duration.as_millis()
        );

        // Phase 2: Browsing behavior with timing
        let browse_start = Instant::now();
        if let Err(e) = perform_browsing_behavior(api).await {
            error!("Browsing behavior error: {}", e);
        }
        let browse_duration = browse_start.elapsed();

        info!(
            "{} Completed {} (total: {}ms, nav: {}ms, browse: {}ms)",
            i + 1,
            url,
            (nav_duration + browse_duration).as_millis(),
            nav_duration.as_millis(),
            browse_duration.as_millis()
        );

        // Phase 3: Pause between URLs
        api.pause(3000).await;
    }

    Ok(())
}

async fn perform_browsing_behavior(api: &TaskContext) -> Result<()> {
    // Random scroll to simulate reading
    api.random_scroll().await?;

    // Pause to simulate reading time
    api.pause(5000).await;

    // Maybe scroll to bottom and back to top
    if rand::random::<bool>() {
        api.scroll_to_bottom().await?;
        api.pause(2000).await;
        api.scroll_to_top().await?;
    }

    // Final pause before moving to next URL
    api.pause(2000).await;

    Ok(())
}

fn read_cookiebot_urls(data_file: &str) -> Result<Vec<String>> {
    let content = fs::read_to_string(data_file).map_err(|e| {
        OrchestratorError::Task(TaskError::ExecutionFailed {
            task_name: "cookiebot".to_string(),
            reason: format!("Failed to read {}: {e}", data_file),
        })
    })?;

    let urls: Vec<String> = content
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    Ok(urls)
}
