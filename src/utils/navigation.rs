//! Browser navigation utilities for page loading and lifecycle management.
//!
//! Provides functions for page navigation (goto, reload, back),
//! user agent overrides, and wait-for-load synchronization.

use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::network::{
    EnableParams as NetworkEnableParams, Headers, SetBlockedUrLsParams, SetExtraHttpHeadersParams,
    SetUserAgentOverrideParams,
};
use chromiumoxide::cdp::browser_protocol::page::NavigateParams;
use chromiumoxide::Page;
use tokio::time::{timeout, Duration};

/// Smart defense: blocks in-page JavaScript from port-scanning local CDP ports (127.0.0.1/localhost)
/// when browsing external websites, while automatically allowing local web app testing.
pub async fn apply_port_scan_defense(page: &Page, target_url: &str) -> Result<()> {
    let is_local_dev = target_url.contains("://localhost")
        || target_url.contains("://127.0.0.1")
        || target_url.contains("://0.0.0.0")
        || target_url.starts_with("localhost:")
        || target_url.starts_with("127.0.0.1:");

    if is_local_dev {
        let _ = page.execute(SetBlockedUrLsParams::new(vec![])).await;
        return Ok(());
    }

    let _ = page.execute(NetworkEnableParams::default()).await;
    let blocked_patterns = vec![
        "*://127.0.0.1:*".to_string(),
        "*://localhost:*".to_string(),
        "*://[::1]:*".to_string(),
    ];
    let _ = page
        .execute(SetBlockedUrLsParams::new(blocked_patterns))
        .await;
    Ok(())
}

use crate::utils::math::random_in_range;
use crate::utils::timing::human_pause;

#[allow(clippy::cast_precision_loss)]
pub async fn goto(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    goto_with_trampoline(page, url, timeout_ms).await
}

pub async fn goto_with_trampoline(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    let referrers = [
        "https://www.google.com",
        "https://www.bing.com",
        "https://search.yahoo.com",
        "https://duckduckgo.com",
        "https://www.reddit.com",
        "https://x.com",
        "https://web.telegram.org",
        "https://web.whatsapp.com",
    ];

    let len = referrers.len() as u64;
    let idx = random_in_range(0, len.saturating_sub(1)) as usize;
    let _referrer_hint = referrers[idx];

    if random_in_range(0, 10) < 3 {
        human_pause(random_in_range(150, 500), 20).await;
    } else {
        human_pause(random_in_range(500, 1200), 30).await;
    }

    goto_raw(page, url, timeout_ms).await
}

pub async fn goto_light(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    goto_raw(page, url, timeout_ms).await
}

pub async fn goto_raw(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    let _ = apply_port_scan_defense(page, url).await;
    timeout(Duration::from_millis(timeout_ms), async {
        if let Err(e) = page.execute(NavigateParams::new(url)).await {
            log::debug!("Page.navigate returned {e}, falling back to page.goto");
            page.goto(url).await?;
        }
        Ok::<(), anyhow::Error>(())
    })
    .await??;

    Ok(())
}

pub async fn go_back(page: &Page) -> Result<()> {
    page.evaluate("window.history.back()").await?;
    Ok(())
}

pub async fn set_user_agent(page: &Page, user_agent: &str) -> Result<()> {
    page.execute(SetUserAgentOverrideParams::new(user_agent))
        .await?;
    Ok(())
}

pub async fn set_extra_http_headers(
    page: &Page,
    headers: &std::collections::BTreeMap<String, String>,
) -> Result<()> {
    let json_headers = serde_json::to_value(headers)?;
    page.execute(SetExtraHttpHeadersParams::new(Headers::new(json_headers)))
        .await?;
    Ok(())
}

pub fn build_stealth_script(enable_random_screen: bool) -> String {
    let screen_flag = if enable_random_screen { "true" } else { "false" };

    let stealth_template = r#"(() => {
        // 1. SOTA Function.prototype.toString Cloaking
        let nativeFunctions = new Set();
        try {
            const originalFunctionToString = Function.prototype.toString;
            const nativeToStringFunctionString = 'function toString() { [native code] }';

            Function.prototype.toString = function() {
                if (this === Function.prototype.toString) {
                    return nativeToStringFunctionString;
                }
                if (nativeFunctions.has(this)) {
                    return `function ${this.name || ''}() { [native code] }`;
                }
                return originalFunctionToString.call(this);
            };
            nativeFunctions.add(Function.prototype.toString);
        } catch (_) {}

        const makeNativeFn = (name, fnImpl) => {
            const fn = fnImpl || function() {};
            try {
                Object.defineProperty(fn, 'name', { value: name, configurable: true });
                nativeFunctions.add(fn);
            } catch (_) {}
            return fn;
        };

        // 2. Fix navigator.webdriver prototype leak
        try {
            const proto = Object.getPrototypeOf(navigator) || Navigator.prototype;
            delete proto.webdriver;
            Object.defineProperty(proto, 'webdriver', {
                get: makeNativeFn('get webdriver', () => undefined),
                enumerable: true,
                configurable: true
            });
        } catch (_) {}

        // 3. Realistic Chrome PluginArray & MimeTypeArray
        try {
            const makePlugin = (name, filename, description) => {
                const plugin = {
                    0: { type: 'application/pdf', suffixes: 'pdf', description: 'Portable Document Format', enabledPlugin: null },
                    1: { type: 'text/pdf', suffixes: 'pdf', description: 'Portable Document Format', enabledPlugin: null },
                    description,
                    filename,
                    name,
                    length: 2,
                    item: makeNativeFn('item', function(i) { return this[i] || null; }),
                    namedItem: makeNativeFn('namedItem', function(n) { return (this[0] && this[0].type === n) ? this[0] : (this[1] && this[1].type === n ? this[1] : null); })
                };
                plugin[0].enabledPlugin = plugin;
                plugin[1].enabledPlugin = plugin;
                return plugin;
            };

            const plugins = [
                makePlugin('PDF Viewer', 'internal-pdf-viewer', 'Portable Document Format'),
                makePlugin('Chrome PDF Viewer', 'internal-pdf-viewer', 'Portable Document Format'),
                makePlugin('Chromium PDF Viewer', 'internal-pdf-viewer', 'Portable Document Format'),
                makePlugin('Microsoft Edge PDF Viewer', 'internal-pdf-viewer', 'Portable Document Format'),
                makePlugin('WebKit built-in PDF', 'internal-pdf-viewer', 'Portable Document Format')
            ];

            const pluginArray = Object.assign(plugins, {
                item: makeNativeFn('item', (i) => plugins[i] || null),
                namedItem: makeNativeFn('namedItem', (n) => plugins.find(p => p.name === n) || null),
                refresh: makeNativeFn('refresh', () => {}),
                length: plugins.length
            });
            Object.defineProperty(navigator, 'plugins', {
                get: makeNativeFn('get plugins', () => pluginArray),
                enumerable: true,
                configurable: true
            });

            const mimeTypes = [
                { type: 'application/pdf', suffixes: 'pdf', description: 'Portable Document Format', enabledPlugin: plugins[0] },
                { type: 'text/pdf', suffixes: 'pdf', description: 'Portable Document Format', enabledPlugin: plugins[0] }
            ];
            const mimeTypeArray = Object.assign(mimeTypes, {
                item: makeNativeFn('item', (i) => mimeTypes[i] || null),
                namedItem: makeNativeFn('namedItem', (n) => mimeTypes.find(m => m.type === n) || null),
                length: mimeTypes.length
            });
            Object.defineProperty(navigator, 'mimeTypes', {
                get: makeNativeFn('get mimeTypes', () => mimeTypeArray),
                enumerable: true,
                configurable: true
            });
            Object.defineProperty(navigator, 'pdfViewerEnabled', {
                get: makeNativeFn('get pdfViewerEnabled', () => true),
                enumerable: true,
                configurable: true
            });
        } catch (_) {}

        // 4. Mock window.chrome with complete native prototypes
        try {
            if (!window.chrome) {
                window.chrome = {
                    app: {
                        isInstalled: false,
                        InstallState: { DISABLED: 'disabled', INSTALLED: 'installed', NOT_INSTALLED: 'not_installed' },
                        RunningState: { CANNOT_RUN: 'cannot_run', READY_TO_RUN: 'ready_to_run', RUNNING: 'running' }
                    },
                    runtime: {
                        OnInstalledReason: { CHROME_UPDATE: 'chrome_update', INSTALL: 'install', SHARED_MODULE_UPDATE: 'shared_module_update', UPDATE: 'update' },
                        OnRestartRequiredReason: { APP_UPDATE: 'app_update', OS_UPDATE: 'os_update', PERIODIC: 'periodic', PROFILE_ERROR: 'profile_error' },
                        PlatformArch: { ARM: 'arm', ARM64: 'arm64', MIPS: 'mips', MIPS64: 'mips64', X86_32: 'x86-32', X86_64: 'x86-64' },
                        PlatformNaclArch: { ARM: 'arm', MIPS: 'mips', MIPS64: 'mips64', X86_32: 'x86-32', X86_64: 'x86-64' },
                        PlatformOs: { ANDROID: 'android', CROS: 'cros', LINUX: 'linux', MAC: 'mac', OPENBSD: 'openbsd', WIN: 'win' },
                        connect: makeNativeFn('connect'),
                        sendMessage: makeNativeFn('sendMessage')
                    },
                    csi: makeNativeFn('csi'),
                    loadTimes: makeNativeFn('loadTimes')
                };
            }
        } catch (_) {}

        // 5. Ensure browser languages & hardware concurrency (only if missing/virtualized)
        try {
            if (!navigator.languages || navigator.languages.length === 0) {
                Object.defineProperty(navigator, 'languages', {
                    get: makeNativeFn('get languages', () => ['en-US', 'en']),
                    enumerable: true,
                    configurable: true
                });
            }
            if (!navigator.hardwareConcurrency || navigator.hardwareConcurrency < 2) {
                Object.defineProperty(navigator, 'hardwareConcurrency', {
                    get: makeNativeFn('get hardwareConcurrency', () => 8),
                    enumerable: true,
                    configurable: true
                });
            }
            if (!navigator.deviceMemory || navigator.deviceMemory < 2) {
                Object.defineProperty(navigator, 'deviceMemory', {
                    get: makeNativeFn('get deviceMemory', () => 8),
                    enumerable: true,
                    configurable: true
                });
            }
        } catch (_) {}

        // 6. Mock navigator.userAgentData only if missing
        try {
            if (!navigator.userAgentData) {
                const userAgentData = {
                    brands: [
                        { brand: 'Chromium', version: '124' },
                        { brand: 'Google Chrome', version: '124' },
                        { brand: 'Not-A.Brand', version: '99' }
                    ],
                    mobile: false,
                    platform: 'Windows',
                    getHighEntropyValues: makeNativeFn('getHighEntropyValues', (hints) => Promise.resolve({
                        architecture: 'x86',
                        bitness: '64',
                        brands: [
                            { brand: 'Chromium', version: '124' },
                            { brand: 'Google Chrome', version: '124' },
                            { brand: 'Not-A.Brand', version: '99' }
                        ],
                        mobile: false,
                        model: '',
                        platform: 'Windows',
                        platformVersion: '15.0.0'
                    }))
                };
                Object.defineProperty(navigator, 'userAgentData', {
                    get: makeNativeFn('get userAgentData', () => userAgentData),
                    enumerable: true,
                    configurable: true
                });
            }
        } catch (_) {}

        // 7. WebGL GPU Vendor & Renderer (only fallback if software/headless SwiftShader is detected)
        try {
            const getParameterProxy = function(parameter) {
                // UNMASKED_VENDOR_WEBGL = 37445, UNMASKED_RENDERER_WEBGL = 37446
                if (parameter === 37445 || parameter === 37446) {
                    const rawVal = this.rawGetParameter(parameter) || '';
                    const isSoftware = /SwiftShader|llvmpipe|Mesa|Software/i.test(rawVal);
                    if (isSoftware) {
                        if (parameter === 37445) return 'Google Inc. (NVIDIA)';
                        if (parameter === 37446) return 'ANGLE (NVIDIA, NVIDIA GeForce RTX 3060 Direct3D11 vs_5_0 ps_5_0, D3D11)';
                    }
                    return rawVal;
                }
                return this.rawGetParameter(parameter);
            };

            if (window.WebGLRenderingContext) {
                const origGetParam1 = WebGLRenderingContext.prototype.getParameter;
                WebGLRenderingContext.prototype.rawGetParameter = origGetParam1;
                WebGLRenderingContext.prototype.getParameter = makeNativeFn('getParameter', getParameterProxy);
            }
            if (window.WebGL2RenderingContext) {
                const origGetParam2 = WebGL2RenderingContext.prototype.getParameter;
                WebGL2RenderingContext.prototype.rawGetParameter = origGetParam2;
                WebGL2RenderingContext.prototype.getParameter = makeNativeFn('getParameter', getParameterProxy);
            }
        } catch (_) {}

        // 8. Headless Window Outer Dimensions (only if running headless 0x0)
        try {
            if (window.outerWidth === 0 && window.outerHeight === 0) {
                Object.defineProperty(window, 'outerWidth', {
                    get: makeNativeFn('get outerWidth', () => window.innerWidth),
                    enumerable: true,
                    configurable: true
                });
                Object.defineProperty(window, 'outerHeight', {
                    get: makeNativeFn('get outerHeight', () => window.innerHeight + 85),
                    enumerable: true,
                    configurable: true
                });
            }
        } catch (_) {}

        // 9. Permissions API consistency
        try {
            const originalQuery = window.navigator.permissions.query;
            window.navigator.permissions.query = makeNativeFn('query', (parameters) => (
                parameters && parameters.name === 'notifications' ?
                    Promise.resolve({ state: Notification.permission }) :
                    originalQuery(parameters)
            ));
        } catch (_) {}

        // 10. Screen Dimensions & Available Resolution Emulation (20 Popular Variants)
        try {
            const enableRandomScreen = __ENABLE_RANDOM_SCREEN__;
            const currentW = window.screen.width;
            const currentH = window.screen.height;
            const isDefaultOrSmall = (currentW === 1280 && currentH === 800) || (currentW === 800 && currentH === 600) || (currentW === 0);

            if (enableRandomScreen && isDefaultOrSmall) {
                const popularScreens = [
                    { w: 1920, h: 1080, dpr: 1.0, availHDiff: 40 },  // 1080p Desktop (#1 most popular, 23%)
                    { w: 1366, h: 768,  dpr: 1.0, availHDiff: 40 },  // Standard Laptop (#2, 14%)
                    { w: 1536, h: 864,  dpr: 1.25, availHDiff: 40 }, // Windows 125% scaling (#3, 10%)
                    { w: 1440, h: 900,  dpr: 1.0, availHDiff: 40 },  // MacBook / 16:10 standard (6%)
                    { w: 1600, h: 900,  dpr: 1.0, availHDiff: 40 },  // HD+ Desktop (4%)
                    { w: 2560, h: 1440, dpr: 1.0, availHDiff: 48 },  // 2K QHD Gaming (4%)
                    { w: 1680, h: 1050, dpr: 1.0, availHDiff: 40 },  // WSXGA+ Widescreen (2%)
                    { w: 1920, h: 1200, dpr: 1.0, availHDiff: 40 },  // WUXGA 16:10 (2%)
                    { w: 2560, h: 1600, dpr: 2.0, availHDiff: 0 },   // Retina 13" / 16:10 (2%)
                    { w: 3840, h: 2160, dpr: 1.5, availHDiff: 48 },  // 4K UHD 150% (1.5%)
                    { w: 1280, h: 1024, dpr: 1.0, availHDiff: 40 },  // 5:4 Standard LCD (1.5%)
                    { w: 1440, h: 960,  dpr: 1.5, availHDiff: 40 },  // Surface 3:2 Display (1%)
                    { w: 2880, h: 1800, dpr: 2.0, availHDiff: 0 },   // Retina 15" Display (1%)
                    { w: 1360, h: 768,  dpr: 1.0, availHDiff: 40 },  // 768p Desktop (1%)
                    { w: 1792, h: 1120, dpr: 2.0, availHDiff: 0 },   // MacBook Pro 16" (1%)
                    { w: 2256, h: 1504, dpr: 1.5, availHDiff: 40 },  // Surface Laptop (1%)
                    { w: 3440, h: 1440, dpr: 1.0, availHDiff: 48 },  // UltraWide 21:9 (1%)
                    { w: 2304, h: 1440, dpr: 2.0, availHDiff: 0 },   // MacBook 12" (1%)
                    { w: 1280, h: 720,  dpr: 1.0, availHDiff: 40 },  // 720p HD (1%)
                    { w: 1920, h: 1080, dpr: 1.25, availHDiff: 40 }  // 1080p with 125% scaling (1%)
                ];

                const chosen = popularScreens[Math.floor(Math.random() * popularScreens.length)];

                Object.defineProperty(window.screen, 'width', {
                    get: makeNativeFn('get width', () => chosen.w),
                    enumerable: true,
                    configurable: true
                });
                Object.defineProperty(window.screen, 'height', {
                    get: makeNativeFn('get height', () => chosen.h),
                    enumerable: true,
                    configurable: true
                });
                Object.defineProperty(window.screen, 'availWidth', {
                    get: makeNativeFn('get availWidth', () => chosen.w),
                    enumerable: true,
                    configurable: true
                });
                Object.defineProperty(window.screen, 'availHeight', {
                    get: makeNativeFn('get availHeight', () => chosen.h - chosen.availHDiff),
                    enumerable: true,
                    configurable: true
                });
                Object.defineProperty(window.screen, 'colorDepth', {
                    get: makeNativeFn('get colorDepth', () => 24),
                    enumerable: true,
                    configurable: true
                });
                Object.defineProperty(window.screen, 'pixelDepth', {
                    get: makeNativeFn('get pixelDepth', () => 24),
                    enumerable: true,
                    configurable: true
                });
                Object.defineProperty(window.devicePixelRatio ? window : Object.getPrototypeOf(window), 'devicePixelRatio', {
                    get: makeNativeFn('get devicePixelRatio', () => chosen.dpr),
                    enumerable: true,
                    configurable: true
                });
            }
        } catch (_) {}
    })()"#;

    stealth_template.replace("__ENABLE_RANDOM_SCREEN__", screen_flag)
}

/// Injects stealth/evasion scripts into the page to prevent bot detection.
pub async fn inject_stealth_scripts(page: &Page) -> Result<()> {
    let enable_random_screen = std::env::var("RANDOM_SCREEN_SIZE_BRAVE__AND_CHROME")
        .or_else(|_| std::env::var("RANDOM_SCREEN_SIZE_BRAVE_AND_CHROME"))
        .map(|v| !v.trim().eq_ignore_ascii_case("false") && v.trim() != "0")
        .unwrap_or(true);

    let stealth_js = build_stealth_script(enable_random_screen);

    // Register script to execute on every new document / navigation
    let _ = page
        .execute(
            chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams::new(
                stealth_js.clone(),
            ),
        )
        .await;

    // Also evaluate in the currently active document
    let _ = page.evaluate(stealth_js).await;
    Ok(())
}

pub async fn page_url(page: &Page) -> Result<String> {
    let result = page.evaluate("window.location.href").await?;
    let value = result
        .value()
        .ok_or_else(|| anyhow::anyhow!("Failed to read page URL"))?;
    Ok(value.as_str().unwrap_or("").to_string())
}

pub async fn page_title(page: &Page) -> Result<String> {
    let result = page.evaluate("document.title").await?;
    let value = result
        .value()
        .ok_or_else(|| anyhow::anyhow!("Failed to read page title"))?;
    Ok(value.as_str().unwrap_or("").to_string())
}

pub async fn wait_for_load(page: &Page, timeout_ms: u64) -> Result<()> {
    timeout(
        Duration::from_millis(timeout_ms),
        wait_for_page_settle(page, timeout_ms),
    )
    .await??;
    Ok(())
}

async fn wait_for_page_settle(page: &Page, timeout_ms: u64) -> Result<()> {
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        let state = match page.evaluate("document.readyState").await {
            Ok(res) => res.value().and_then(|v| v.as_str().map(str::to_string)),
            Err(_) => None, // Ignore transient CDP context destruction during redirects
        };

        if matches!(state.as_deref(), Some("interactive" | "complete")) {
            return Ok(());
        }

        if std::time::Instant::now() >= deadline {
            anyhow::bail!("Page navigation timeout: readyState did not reach interactive/complete");
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_stealth_script_random_screen_true() {
        let script = build_stealth_script(true);
        assert!(script.contains("const enableRandomScreen = true;"));
        assert!(script.contains("1920, h: 1080"));
        assert!(script.contains("isDefaultOrSmall"));
    }

    #[test]
    fn test_build_stealth_script_random_screen_false() {
        let script = build_stealth_script(false);
        assert!(script.contains("const enableRandomScreen = false;"));
    }

    #[test]
    fn test_referrers_array_has_values() {
        let referrers = [
            "https://www.google.com",
            "https://www.bing.com",
            "https://search.yahoo.com",
            "https://duckduckgo.com",
            "https://www.reddit.com",
            "https://x.com",
            "https://web.telegram.org",
            "https://web.whatsapp.com",
        ];
        assert_eq!(referrers.len(), 8);
    }

    #[test]
    fn test_referrer_list_valid_urls() {
        let referrers = [
            "https://www.google.com",
            "https://www.bing.com",
            "https://search.yahoo.com",
            "https://duckduckgo.com",
            "https://www.reddit.com",
            "https://x.com",
            "https://web.telegram.org",
            "https://web.whatsapp.com",
        ];
        for referrer in &referrers {
            assert!(referrer.starts_with("https://"));
            assert!(referrer.contains('.'));
        }
    }

    #[test]
    fn test_page_settle_deadline() {
        let timeout_ms = 10_000u64;
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(timeout_ms);
        assert!(deadline > std::time::Instant::now());
    }

    #[test]
    fn test_headers_serialization() {
        let mut map = std::collections::BTreeMap::new();
        map.insert("X-Custom-Header".to_string(), "Value123".to_string());
        let json_val = serde_json::to_value(&map).expect("serialization works");
        assert_eq!(json_val["X-Custom-Header"], "Value123");
    }

    #[test]
    fn test_ready_state_values() {
        let valid_states = ["loading", "interactive", "complete"];
        for state in &valid_states {
            assert!(!state.is_empty());
        }
    }
}
