use chromiumoxide::Page;
use anyhow::Result;
use tokio::time::{timeout, Duration};
use crate::utils::block_heavy_resources;

pub async fn goto(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    // Apply a single unified network blocklist before navigation.
    block_heavy_resources(page).await?;

    timeout(
        Duration::from_millis(timeout_ms),
        page.goto(url)
    )
    .await??;

    Ok(())
}

pub async fn wait_for_load(page: &Page, timeout_ms: u64) -> Result<()> {
    timeout(
        Duration::from_millis(timeout_ms),
        page.wait_for_navigation()
    )
    .await??;

    Ok(())
}