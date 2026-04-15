use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::network;

pub async fn block_images(page: &chromiumoxide::Page) -> Result<()> {
    let params = network::SetBlockedUrLsParams::new(vec![
        "*.png".into(),
        "*.jpg".into(),
        "*.jpeg".into(),
        "*.gif".into(),
        "*.webp".into(),
        "*.svg".into(),
        "*.bmp".into(),
        "data:image/*".into(),
    ]);
    
    page.execute(params).await?;

    log::info!("   [utils] Images blocked via Network.setBlockedURLs");
    Ok(())
}

pub async fn block_media(page: &chromiumoxide::Page) -> Result<()> {
    let params = network::SetBlockedUrLsParams::new(vec![
        "*.mp4".into(),
        "*.webm".into(),
        "*.m3u8".into(),
        "*.ts".into(),
    ]);
    
    page.execute(params).await?;

    log::info!("   [utils] Media blocked via Network.setBlockedURLs");
    Ok(())
}

pub async fn unblock_all(page: &chromiumoxide::Page) -> Result<()> {
    let params = network::SetBlockedUrLsParams::new(vec![]);
    
    page.execute(params).await?;

    log::info!("   [utils] All URLs unblocked");
    Ok(())
}