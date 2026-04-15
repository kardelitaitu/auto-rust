use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::network;

fn blocked_patterns() -> Vec<String> {
    vec![
        // Images
        "*.png", "*.png*", "*.jpg", "*.jpg*", "*.jpeg", "*.jpeg*", "*.gif", "*.gif*",
        "*.webp", "*.webp*", "*.svg", "*.svg*", "*.bmp", "*.bmp*", "*.ico", "*.ico*",
        "data:image/*",
        // Video/Audio
        "*.mp4", "*.mp4*", "*.webm", "*.webm*", "*.m3u8", "*.m3u8*", "*.ts", "*.ts*",
        "*.mkv", "*.mkv*", "*.avi", "*.avi*", "*.mov", "*.mov*", "*.flv", "*.flv*",
        "*.mp3", "*.mp3*", "*.wav", "*.wav*", "*.aac", "*.aac*", "*.ogg", "*.ogg*",
        "*.opus", "*.opus*", "*.mpd", "*.mpd*",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn dom_cleanup_js() -> &'static str {
    r#"
        (() => {
            document.querySelectorAll('img, video, audio, source, picture, [style*="background-image"]').forEach(el => {
                if (el.src && !el.src.startsWith('data:')) { el.src = ''; }
                if (el.srcset) { el.srcset = ''; }
                if (el.tagName === 'VIDEO' || el.tagName === 'AUDIO') {
                    try { el.pause(); } catch (_) {}
                    el.srcObject = null;
                    el.remove();
                }
            });
            HTMLVideoElement.prototype.play = () => Promise.resolve();
            HTMLAudioElement.prototype.play = () => Promise.resolve();
        })()
    "#
}

pub async fn block_heavy_resources(page: &chromiumoxide::Page) -> Result<()> {
    // Network-level blocking is the primary bandwidth/resource saver.
    page.execute(network::EnableParams::default()).await?;
    page.execute(network::SetBlockedUrLsParams::new(blocked_patterns())).await?;

    // DOM cleanup is a fallback for already-present media elements.
    page.evaluate(dom_cleanup_js()).await?;

    Ok(())
}

pub async fn block_images(page: &chromiumoxide::Page) -> Result<()> {
    block_heavy_resources(page).await
}

pub async fn block_media(page: &chromiumoxide::Page) -> Result<()> {
    block_heavy_resources(page).await
}

pub async fn unblock_all(_page: &chromiumoxide::Page) -> Result<()> {
    log::info!("   [utils] All URLs unblocked (no-op)");
    Ok(())
}