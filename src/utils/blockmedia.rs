use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::network;

#[allow(dead_code)]
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

// Aggressive JS-level blocking for cookiebot task.
// Overrides fetch() and XMLHttpRequest to prevent media requests altogether.
fn cookiebot_block_js() -> &'static str {
    r#"
        (() => {
            const BLOCK_EXTENSIONS = ['ts', 'm3u8', 'mp4', 'webm', 'mp3', 'wav', 'opus', 'aac', 'ogg', 'mpd', 'mkv', 'avi', 'mov', 'flv'];
            
            function shouldBlock(url) {
                try {
                    const u = new URL(url, location.href);
                    const pathname = u.pathname.toLowerCase();
                    return BLOCK_EXTENSIONS.some(ext => pathname.endsWith('.' + ext));
                } catch {
                    return false;
                }
            }

            // Override fetch()
            const OriginalFetch = window.fetch;
            window.fetch = function(input, init) {
                if (typeof input === 'string' && shouldBlock(input)) {
                    console.log('[cookiebot-block] Blocked fetch:', input);
                    return Promise.reject(new TypeError('Blocked by cookiebot'));
                }
                if (input instanceof Request && shouldBlock(input.url)) {
                    console.log('[cookiebot-block] Blocked fetch:', input.url);
                    return Promise.reject(new TypeError('Blocked by cookiebot'));
                }
                return OriginalFetch.apply(this, arguments);
            };

            // Override XMLHttpRequest
            const OriginalXHR = window.XMLHttpRequest;
            window.XMLHttpRequest = function() {
                const xhr = new OriginalXHR();
                const origOpen = xhr.open;
                xhr.open = function(method, url, ...args) {
                    if (shouldBlock(url)) {
                        console.log('[cookiebot-block] Blocked XHR:', url);
                        // Replace open with a no-op that throws if send is called
                        xhr.send = () => { throw new TypeError('Blocked by cookiebot'); };
                        return;
                    }
                    return origOpen.apply(this, [method, url, ...args]);
                };
                return xhr;
            };

            // Also block WebSocket (some streams use WS)
            const OrigWS = window.WebSocket;
            window.WebSocket = function(url, protocols) {
                if (shouldBlock(url)) {
                    console.log('[cookiebot-block] Blocked WebSocket:', url);
                    return;
                }
                return new OrigWS(url, protocols);
            };
        })()
    "#
}

#[allow(dead_code)]
pub async fn block_heavy_resources(page: &chromiumoxide::Page) -> Result<()> {
    // Network-level blocking is the primary bandwidth/resource saver.
    page.execute(network::EnableParams::default()).await?;
    page.execute(network::SetBlockedUrLsParams::new(blocked_patterns())).await?;

    // DOM cleanup is a fallback for already-present media elements.
    page.evaluate(dom_cleanup_js()).await?;

    Ok(())
}

#[allow(dead_code)]
pub async fn block_heavy_resources_for_cookiebot(page: &chromiumoxide::Page) -> Result<()> {
    // Network-level blocking (same as general)
    page.execute(network::EnableParams::default()).await?;
    page.execute(network::SetBlockedUrLsParams::new(blocked_patterns())).await?;

    // DOM cleanup PLUS fetch/XHR override (separated by semicolon to avoid invocation)
    let script = format!("{};\n{}", dom_cleanup_js(), cookiebot_block_js());
    page.evaluate(script).await?;

    Ok(())
}

#[allow(dead_code)]
pub async fn block_images(page: &chromiumoxide::Page) -> Result<()> {
    block_heavy_resources(page).await
}

#[allow(dead_code)]
pub async fn block_media(page: &chromiumoxide::Page) -> Result<()> {
    block_heavy_resources(page).await
}

#[allow(dead_code)]
pub async fn unblock_all(_page: &chromiumoxide::Page) -> Result<()> {
    log::info!("   [utils] All URLs unblocked (no-op)");
    Ok(())
}