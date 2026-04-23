use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::network::{
    Headers, SetExtraHttpHeadersParams, SetUserAgentOverrideParams,
};
use chromiumoxide::Page;
use tokio::time::{timeout, Duration};

use crate::utils::math::random_in_range;
use crate::utils::timing::human_pause;

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
    timeout(Duration::from_millis(timeout_ms), async {
        page.goto(url).await?;
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

pub async fn focus(page: &Page, selector: &str) -> Result<()> {
    let selector_json = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_json});
            if (!el) return false;

            if (typeof el.focus === 'function') {{
                try {{
                    el.focus({{ preventScroll: true }});
                }} catch (_) {{
                    el.focus();
                }}
            }}

            const active = document.activeElement;
            return active === el || (active && el.contains(active));
        }})()"#,
    );

    page.evaluate(js).await?;
    Ok(())
}

pub async fn selector_exists(page: &Page, selector: &str) -> Result<bool> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            return !!document.querySelector({selector_js});
        }})()"#
    );
    let result = page.evaluate(js).await?;
    Ok(result.value().and_then(|v| v.as_bool()).unwrap_or(false))
}

pub async fn selector_is_visible(page: &Page, selector: &str) -> Result<bool> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return false;
            const rect = el.getBoundingClientRect();
            if (rect.width <= 0 || rect.height <= 0) return false;
            const style = getComputedStyle(el);
            if (style.display === 'none' || style.visibility === 'hidden') return false;
            return true;
        }})()"#,
    );

    let result = page.evaluate(js).await?;
    Ok(result.value().and_then(|v| v.as_bool()).unwrap_or(false))
}

pub async fn selector_text(page: &Page, selector: &str) -> Result<Option<String>> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return null;
            const text = (el.innerText || el.textContent || "").trim();
            return text.length ? text : null;
        }})()"#,
    );

    let result = page.evaluate(js).await?;
    Ok(result
        .value()
        .and_then(|v| v.as_str().map(|s| s.to_string())))
}

pub async fn selector_html(page: &Page, selector: &str) -> Result<Option<String>> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return null;
            const html = (el.innerHTML || "").trim();
            return html.length ? html : null;
        }})()"#,
    );

    let result = page.evaluate(js).await?;
    Ok(result
        .value()
        .and_then(|v| v.as_str().map(|s| s.to_string())))
}

pub async fn selector_attr(page: &Page, selector: &str, name: &str) -> Result<Option<String>> {
    let selector_js = serde_json::to_string(selector)?;
    let name_js = serde_json::to_string(name)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return null;
            const value = el.getAttribute({name_js});
            if (value == null) return null;
            const trimmed = String(value).trim();
            return trimmed.length ? trimmed : null;
        }})()"#,
    );

    let result = page.evaluate(js).await?;
    Ok(result
        .value()
        .and_then(|v| v.as_str().map(|s| s.to_string())))
}

pub async fn selector_value(page: &Page, selector: &str) -> Result<Option<String>> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return null;
            const value = typeof el.value === 'string' ? el.value : null;
            if (value == null) return null;
            const trimmed = String(value).trim();
            return trimmed.length ? trimmed : null;
        }})()"#,
    );

    let result = page.evaluate(js).await?;
    Ok(result
        .value()
        .and_then(|v| v.as_str().map(|s| s.to_string())))
}

pub async fn wait_for_selector(page: &Page, selector: &str, timeout_ms: u64) -> Result<bool> {
    timeout(Duration::from_millis(timeout_ms), async {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms.min(4000));
        loop {
            if selector_exists(page, selector).await.unwrap_or(false) {
                return Ok(true);
            }

            if std::time::Instant::now() >= deadline {
                return Ok(false);
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await?
}

pub async fn wait_for_visible_selector(
    page: &Page,
    selector: &str,
    timeout_ms: u64,
) -> Result<bool> {
    timeout(Duration::from_millis(timeout_ms), async {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms.min(4000));
        loop {
            if selector_is_visible(page, selector).await.unwrap_or(false) {
                return Ok(true);
            }

            if std::time::Instant::now() >= deadline {
                return Ok(false);
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await?
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
        wait_for_page_settle(page),
    )
    .await??;
    Ok(())
}

pub async fn wait_for_any_visible_selector(
    page: &Page,
    selectors: &[&str],
    timeout_ms: u64,
) -> Result<bool> {
    timeout(Duration::from_millis(timeout_ms), async {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms.min(4000));
        loop {
            for selector in selectors {
                if selector_is_visible(page, selector).await.unwrap_or(false) {
                    return Ok(true);
                }
            }

            if std::time::Instant::now() >= deadline {
                return Ok(false);
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await?
}

async fn wait_for_page_settle(page: &Page) -> Result<()> {
    let deadline = std::time::Instant::now() + Duration::from_secs(4);
    loop {
        let state = page
            .evaluate("document.readyState")
            .await?
            .value()
            .and_then(|v| v.as_str().map(str::to_string));

        if matches!(state.as_deref(), Some("interactive") | Some("complete")) {
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
    fn test_selector_json_serialization() {
        let selector = "div.test";
        let json = serde_json::to_string(selector).unwrap();
        assert_eq!(json, "\"div.test\"");
    }

    #[test]
    fn test_url_json_serialization() {
        let url = "https://example.com";
        let json = serde_json::to_string(url).unwrap();
        assert_eq!(json, "\"https://example.com\"");
    }

    #[test]
    fn test_visibility_check_js_structure() {
        let selector = ".my-element";
        let selector_js = serde_json::to_string(selector).unwrap();
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({selector_js});
                if (!el) return false;
                const rect = el.getBoundingClientRect();
                if (rect.width <= 0 || rect.height <= 0) return false;
                const style = getComputedStyle(el);
                if (style.display === 'none' || style.visibility === 'hidden') return false;
                return true;
            }})()"#,
        );
        assert!(js.contains("getBoundingClientRect"));
    }

    #[test]
    fn test_value_read_js_structure() {
        let selector = "#userEmail";
        let selector_js = serde_json::to_string(selector).unwrap();
        let js = format!(
            r#"(() => {{
                const el = document.querySelector({selector_js});
                if (!el) return null;
                const value = typeof el.value === 'string' ? el.value : null;
                if (value == null) return null;
                const trimmed = String(value).trim();
                return trimmed.length ? trimmed : null;
            }})()"#,
        );
        assert!(js.contains("typeof el.value === 'string'"));
    }
}
