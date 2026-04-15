use chromiumoxide::Page;
use anyhow::Result;
use tokio::time::{timeout, Duration};

#[allow(dead_code)]
pub async fn goto(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    // Navigate to URL with timeout
    timeout(
        Duration::from_millis(timeout_ms),
        page.goto(url)
    )
    .await??;

    Ok(())
}

#[allow(dead_code)]
pub async fn wait_for_load(page: &Page, timeout_ms: u64) -> Result<()> {
    // Wait for network idle (no network activity for 500ms)
    timeout(
        Duration::from_millis(timeout_ms),
        page.wait_for_navigation()
    )
    .await??;

    Ok(())
}