use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::network;

fn blocked_patterns() -> Vec<String> {
    vec![
        // Images
        "*.png",
        "*.png*",
        "*.jpg",
        "*.jpg*",
        "*.jpeg",
        "*.jpeg*",
        "*.gif",
        "*.gif*",
        "*.webp",
        "*.webp*",
        "*.svg",
        "*.svg*",
        "*.bmp",
        "*.bmp*",
        "*.ico",
        "*.ico*",
        "data:image/*",
        // Video/Audio
        "*.mp4",
        "*.mp4*",
        "*.webm",
        "*.webm*",
        "*.m3u8",
        "*.m3u8*",
        "*.ts",
        "*.ts*",
        "*.mkv",
        "*.mkv*",
        "*.avi",
        "*.avi*",
        "*.mov",
        "*.mov*",
        "*.flv",
        "*.flv*",
        "*.mp3",
        "*.mp3*",
        "*.wav",
        "*.wav*",
        "*.aac",
        "*.aac*",
        "*.ogg",
        "*.ogg*",
        "*.opus",
        "*.opus*",
        "*.mpd",
        "*.mpd*",
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

pub async fn block_heavy_resources(page: &chromiumoxide::Page) -> Result<()> {
    // Network-level blocking is the primary bandwidth/resource saver.
    page.execute(network::EnableParams::default()).await?;
    page.execute(network::SetBlockedUrLsParams::new(blocked_patterns()))
        .await?;

    // DOM cleanup is a fallback for already-present media elements.
    page.evaluate(dom_cleanup_js()).await?;

    Ok(())
}

pub async fn block_heavy_resources_for_cookiebot(page: &chromiumoxide::Page) -> Result<()> {
    // Network-level blocking (same as general)
    page.execute(network::EnableParams::default()).await?;
    page.execute(network::SetBlockedUrLsParams::new(blocked_patterns()))
        .await?;

    // DOM cleanup PLUS fetch/XHR override (separated by semicolon to avoid invocation)
    let script = format!("{};\n{}", dom_cleanup_js(), cookiebot_block_js());
    page.evaluate(script).await?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blocked_patterns_not_empty() {
        let patterns = blocked_patterns();
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_blocked_patterns_contains_image_extensions() {
        let patterns = blocked_patterns();
        assert!(patterns.iter().any(|p| p.contains("png")));
        assert!(patterns.iter().any(|p| p.contains("jpg")));
        assert!(patterns.iter().any(|p| p.contains("jpeg")));
        assert!(patterns.iter().any(|p| p.contains("gif")));
        assert!(patterns.iter().any(|p| p.contains("webp")));
    }

    #[test]
    fn test_blocked_patterns_contains_video_extensions() {
        let patterns = blocked_patterns();
        assert!(patterns.iter().any(|p| p.contains("mp4")));
        assert!(patterns.iter().any(|p| p.contains("webm")));
        assert!(patterns.iter().any(|p| p.contains("m3u8")));
        assert!(patterns.iter().any(|p| p.contains("mkv")));
    }

    #[test]
    fn test_blocked_patterns_contains_audio_extensions() {
        let patterns = blocked_patterns();
        assert!(patterns.iter().any(|p| p.contains("mp3")));
        assert!(patterns.iter().any(|p| p.contains("wav")));
        assert!(patterns.iter().any(|p| p.contains("aac")));
        assert!(patterns.iter().any(|p| p.contains("ogg")));
    }

    #[test]
    fn test_blocked_patterns_contains_data_image() {
        let patterns = blocked_patterns();
        assert!(patterns.iter().any(|p| p == "data:image/*"));
    }

    #[test]
    fn test_dom_cleanup_js_not_empty() {
        let js = dom_cleanup_js();
        assert!(!js.is_empty());
    }

    #[test]
    fn test_dom_cleanup_js_contains_key_operations() {
        let js = dom_cleanup_js();
        assert!(js.contains("querySelectorAll"));
        assert!(js.contains("img"));
        assert!(js.contains("video"));
        assert!(js.contains("audio"));
        assert!(js.contains("src"));
        assert!(js.contains("remove"));
    }

    #[test]
    fn test_cookiebot_block_js_not_empty() {
        let js = cookiebot_block_js();
        assert!(!js.is_empty());
    }

    #[test]
    fn test_cookiebot_block_js_contains_fetch_override() {
        let js = cookiebot_block_js();
        assert!(js.contains("window.fetch"));
        assert!(js.contains("OriginalFetch"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_xhr_override() {
        let js = cookiebot_block_js();
        assert!(js.contains("XMLHttpRequest"));
        assert!(js.contains("OriginalXHR"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_websocket_override() {
        let js = cookiebot_block_js();
        assert!(js.contains("WebSocket"));
        assert!(js.contains("OrigWS"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_block_extensions() {
        let js = cookiebot_block_js();
        assert!(js.contains("BLOCK_EXTENSIONS"));
        assert!(js.contains("mp4"));
        assert!(js.contains("m3u8"));
    }

    #[test]
    fn test_blocked_patterns_count() {
        let patterns = blocked_patterns();
        // Should have a reasonable number of patterns
        assert!(patterns.len() > 20);
    }

    #[test]
    fn test_blocked_patterns_contains_svg() {
        let patterns = blocked_patterns();
        assert!(patterns.iter().any(|p| p.contains("svg")));
    }

    #[test]
    fn test_blocked_patterns_contains_bmp() {
        let patterns = blocked_patterns();
        assert!(patterns.iter().any(|p| p.contains("bmp")));
    }

    #[test]
    fn test_blocked_patterns_contains_ico() {
        let patterns = blocked_patterns();
        assert!(patterns.iter().any(|p| p.contains("ico")));
    }

    #[test]
    fn test_blocked_patterns_contains_avi() {
        let patterns = blocked_patterns();
        assert!(patterns.iter().any(|p| p.contains("avi")));
    }

    #[test]
    fn test_blocked_patterns_contains_mov() {
        let patterns = blocked_patterns();
        assert!(patterns.iter().any(|p| p.contains("mov")));
    }

    #[test]
    fn test_blocked_patterns_contains_flv() {
        let patterns = blocked_patterns();
        assert!(patterns.iter().any(|p| p.contains("flv")));
    }

    #[test]
    fn test_blocked_patterns_contains_opus() {
        let patterns = blocked_patterns();
        assert!(patterns.iter().any(|p| p.contains("opus")));
    }

    #[test]
    fn test_blocked_patterns_contains_mpd() {
        let patterns = blocked_patterns();
        assert!(patterns.iter().any(|p| p.contains("mpd")));
    }

    #[test]
    fn test_blocked_patterns_wildcard_variants() {
        let patterns = blocked_patterns();
        // Should have both *.ext and *.ext* variants
        assert!(patterns.iter().any(|p| p == "*.png"));
        assert!(patterns.iter().any(|p| p == "*.png*"));
    }

    #[test]
    fn test_dom_cleanup_js_contains_play_override() {
        let js = dom_cleanup_js();
        assert!(js.contains("HTMLVideoElement.prototype.play"));
        assert!(js.contains("HTMLAudioElement.prototype.play"));
    }

    #[test]
    fn test_dom_cleanup_js_contains_srcset() {
        let js = dom_cleanup_js();
        assert!(js.contains("srcset"));
    }

    #[test]
    fn test_dom_cleanup_js_contains_pause() {
        let js = dom_cleanup_js();
        assert!(js.contains("pause"));
    }

    #[test]
    fn test_dom_cleanup_js_contains_background_image() {
        let js = dom_cleanup_js();
        assert!(js.contains("background-image"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_should_block() {
        let js = cookiebot_block_js();
        assert!(js.contains("shouldBlock"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_console_log() {
        let js = cookiebot_block_js();
        assert!(js.contains("console.log"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_blocked_message() {
        let js = cookiebot_block_js();
        assert!(js.contains("Blocked by cookiebot"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_type_error() {
        let js = cookiebot_block_js();
        assert!(js.contains("TypeError"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_url_parsing() {
        let js = cookiebot_block_js();
        assert!(js.contains("new URL"));
    }

    #[test]
    fn test_blocked_patterns_all_strings() {
        let patterns = blocked_patterns();
        // All patterns should be strings (verified by type system)
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_blocked_patterns_no_empty_strings() {
        let patterns = blocked_patterns();
        // No pattern should be empty
        for pattern in &patterns {
            assert!(!pattern.is_empty());
        }
    }

    #[test]
    fn test_dom_cleanup_js_contains_source_element() {
        let js = dom_cleanup_js();
        assert!(js.contains("source"));
    }

    #[test]
    fn test_dom_cleanup_js_contains_picture_element() {
        let js = dom_cleanup_js();
        assert!(js.contains("picture"));
    }

    #[test]
    fn test_dom_cleanup_js_contains_data_check() {
        let js = dom_cleanup_js();
        assert!(js.contains("data:"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_try_catch() {
        let js = cookiebot_block_js();
        assert!(js.contains("try"));
        assert!(js.contains("catch"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_protocols_param() {
        let js = cookiebot_block_js();
        assert!(js.contains("protocols"));
    }

    #[test]
    fn test_blocked_patterns_exact_count() {
        let patterns = blocked_patterns();
        // Exact count based on the implementation
        assert_eq!(patterns.len(), 45);
    }

    #[test]
    fn test_dom_cleanup_js_is_wrapped_in_iife() {
        let js = dom_cleanup_js();
        assert!(js.contains("(() =>"));
        assert!(js.contains("})()"));
    }

    #[test]
    fn test_cookiebot_block_js_is_wrapped_in_iife() {
        let js = cookiebot_block_js();
        assert!(js.contains("(() =>"));
        assert!(js.contains("})()"));
    }

    #[test]
    fn test_dom_cleanup_js_contains_foreach() {
        let js = dom_cleanup_js();
        assert!(js.contains("forEach"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_some() {
        let js = cookiebot_block_js();
        assert!(js.contains(".some"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_endswith() {
        let js = cookiebot_block_js();
        assert!(js.contains("endsWith"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_pathname() {
        let js = cookiebot_block_js();
        assert!(js.contains("pathname"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_tolowercase() {
        let js = cookiebot_block_js();
        assert!(js.contains("toLowerCase"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_send_override() {
        let js = cookiebot_block_js();
        assert!(js.contains("xhr.send"));
    }

    #[test]
    fn test_blocked_patterns_pattern_format() {
        let patterns = blocked_patterns();
        // All patterns should start with * or data:
        for pattern in &patterns {
            assert!(pattern.starts_with('*') || pattern.starts_with("data:"));
        }
    }

    #[test]
    fn test_dom_cleanup_js_contains_srcobject() {
        let js = dom_cleanup_js();
        assert!(js.contains("srcObject"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_origws() {
        let js = cookiebot_block_js();
        assert!(js.contains("OrigWS"));
    }

    #[test]
    fn test_blocked_patterns_unique() {
        let patterns = blocked_patterns();
        // All patterns should be unique
        let unique_patterns: std::collections::HashSet<_> = patterns.iter().collect();
        assert_eq!(unique_patterns.len(), patterns.len());
    }

    #[test]
    fn test_dom_cleanup_js_contains_style_attribute() {
        let js = dom_cleanup_js();
        assert!(js.contains("[style"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_method_param() {
        let js = cookiebot_block_js();
        assert!(js.contains("method"));
    }

    #[test]
    fn test_cookiebot_block_js_contains_noop_for_blocked() {
        let js = cookiebot_block_js();
        assert!(js.contains("no-op"));
    }
}
