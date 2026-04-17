use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::network::{
    Headers, SetExtraHttpHeadersParams, SetUserAgentOverrideParams,
};
use chromiumoxide::cdp::browser_protocol::page::NavigateParams;
use chromiumoxide::Page;
use tokio::time::{timeout, Duration};

use crate::utils::math::random_in_range;
use crate::utils::mouse::cursor_move_to_immediate;
use crate::utils::page_size::get_viewport;
use crate::utils::timing::human_pause;

pub async fn goto(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    goto_raw(page, url, timeout_ms).await
}

pub async fn goto_with_warmup(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    timeout(Duration::from_millis(timeout_ms), async {
        let _ = timeout(Duration::from_secs(3), warmup_before_navigate(page)).await;
        page.execute(NavigateParams::new(url.to_string())).await?;
        Ok::<(), anyhow::Error>(())
    })
    .await??;

    Ok(())
}

pub async fn goto_light(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    goto_raw(page, url, timeout_ms).await
}

pub async fn goto_raw(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    timeout(Duration::from_millis(timeout_ms), async {
        let js_url = serde_json::to_string(url)?;
        page.evaluate(format!("window.location.href = {js_url};")).await?;
        Ok::<(), anyhow::Error>(())
    })
    .await??;

    Ok(())
}

pub async fn set_user_agent(page: &Page, user_agent: &str) -> Result<()> {
    page.execute(SetUserAgentOverrideParams::new(user_agent)).await?;
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

pub async fn wait_for_load(page: &Page, timeout_ms: u64) -> Result<()> {
    timeout(Duration::from_millis(timeout_ms), wait_for_page_settle(page))
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

async fn warmup_before_navigate(page: &Page) -> Result<()> {
    let viewport = timeout(Duration::from_secs(1), get_viewport(page)).await;
    if let Ok(Ok(viewport)) = viewport {
        let moves = random_in_range(1, 2);
        for _ in 0..moves {
            let x = random_in_range(0, viewport.width.max(1.0) as u64) as f64;
            let y = random_in_range(0, viewport.height.max(1.0) as u64) as f64;
            let _ = timeout(
                Duration::from_millis(700),
                cursor_move_to_immediate(page, x, y),
            )
            .await;
            human_pause(random_in_range(80, 220), 30).await;
        }
    }

    human_pause(random_in_range(150, 420), 35).await;
    Ok(())
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

async fn selector_is_visible(page: &Page, selector: &str) -> Result<bool> {
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

    let result = timeout(Duration::from_secs(2), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("selector visibility check timeout"))??;
    let value = result
        .value()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Failed to read selector visibility"))?;
    Ok(value.as_bool().unwrap_or(false))
}
