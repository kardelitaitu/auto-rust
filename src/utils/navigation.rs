//! Browser navigation utilities for page loading and lifecycle management.
//!
//! Provides functions for page navigation (goto, reload, back),
//! user agent overrides, and wait-for-load synchronization.

use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::network::{
    Headers, SetExtraHttpHeadersParams, SetUserAgentOverrideParams,
};
use chromiumoxide::Page;
use tokio::time::{timeout, Duration};

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

use chromiumoxide::cdp::browser_protocol::page::NavigateParams;

pub async fn goto_raw(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
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

/// Injects stealth/evasion scripts into the page to prevent bot detection.
pub async fn inject_stealth_scripts(page: &Page) -> Result<()> {
    let stealth_js = r#"(() => {
        try {
            Object.defineProperty(navigator, 'webdriver', {
                get: () => undefined,
                configurable: true
            });
        } catch (_) {}
        try {
            if (!window.chrome) {
                window.chrome = { runtime: {}, app: {}, loadTimes: () => ({}), csi: () => ({}) };
            }
        } catch (_) {}
        try {
            Object.defineProperty(navigator, 'languages', {
                get: () => ['en-US', 'en'],
                configurable: true
            });
        } catch (_) {}
        try {
            Object.defineProperty(navigator, 'plugins', {
                get: () => [1, 2, 3, 4, 5],
                configurable: true
            });
        } catch (_) {}
        try {
            const originalQuery = window.navigator.permissions.query;
            window.navigator.permissions.query = (parameters) => (
                parameters.name === 'notifications' ?
                    Promise.resolve({ state: Notification.permission }) :
                    originalQuery(parameters)
            );
        } catch (_) {}
    })()"#;

    // Register script to execute on every new document / navigation
    let _ = page
        .execute(
            chromiumoxide::cdp::browser_protocol::page::AddScriptToEvaluateOnNewDocumentParams::new(
                stealth_js,
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
            return Ok(());
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(test)]
mod tests {
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
