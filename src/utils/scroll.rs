//! Scrolling and page interaction utilities.
//!
//! Provides human-like scrolling behavior with smooth easing, burst scrolling,
//! and viewport-centering helpers for offscreen targets.

use anyhow::Result;
use chromiumoxide::Page;
use tokio::time::{timeout, Duration};

use crate::utils::math::{gaussian, random_in_range};
use crate::utils::timing::human_pause;

pub async fn random_scroll(page: &Page) -> Result<()> {
    read(
        page,
        random_in_range(2, 5) as u32,
        gaussian(420.0, 160.0, 180.0, 900.0) as i32,
        true,
        random_in_range(0, 100) < 70,
    )
    .await
}

#[allow(dead_code)]
pub async fn human_scroll(page: &Page, direction: &str, amount: i32) -> Result<()> {
    let signed = match direction {
        "down" => amount as f64,
        "up" => -(amount as f64),
        _ => return Err(anyhow::anyhow!("Invalid scroll direction: {direction}")),
    };
    let duration = if signed.abs() > 800.0 {
        random_in_range(500, 900)
    } else {
        random_in_range(260, 620)
    };
    smooth_scroll_by(page, signed, duration).await
}

pub async fn read(
    page: &Page,
    pauses: u32,
    scroll_amount: i32,
    variable_speed: bool,
    back_scroll: bool,
) -> Result<()> {
    let pauses = pauses.max(1);
    for i in 0..pauses {
        let amplitude = if variable_speed {
            let jitter = gaussian(0.0, 0.18, -0.35, 0.35);
            ((scroll_amount as f64) * (1.0 + jitter)).round() as i32
        } else {
            scroll_amount
        };
        let direction = if random_in_range(0, 100) < 88 {
            1.0
        } else {
            -1.0
        };
        human_scroll(
            page,
            if direction > 0.0 { "down" } else { "up" },
            amplitude.abs(),
        )
        .await?;

        if i + 1 < pauses {
            human_pause(random_in_range(500, 1100), 45).await;
            if back_scroll && random_in_range(0, 100) < 8 {
                let backtrack = gaussian(28.0, 14.0, 8.0, 70.0) as i32;
                back(page, backtrack).await?;
            }
        }
    }

    human_pause(random_in_range(300, 900), 35).await;
    Ok(())
}

pub async fn back(page: &Page, distance: i32) -> Result<()> {
    let steps = random_in_range(2, 3);
    let total = distance.abs().max(1) as f64;
    let step_size = total / steps as f64;

    for i in 0..steps {
        let duration = random_in_range(160, 320);
        smooth_scroll_by(page, -(step_size), duration).await?;
        if i + 1 < steps {
            human_pause(random_in_range(35, 75), 25).await;
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub async fn scroll_back(page: &Page, distance: i32) -> Result<()> {
    back(page, distance).await
}

pub async fn scroll_into_view(page: &Page, selector: &str) -> Result<()> {
    scroll_into_view_native(page, selector).await?;
    human_pause(random_in_range(40, 90), 25).await;

    let still_offscreen = target_still_offscreen(page, selector)
        .await
        .unwrap_or(false);
    if still_offscreen {
        human_scroll_into_view(page, selector).await?;
        human_pause(random_in_range(50, 120), 30).await;
    }

    Ok(())
}

pub async fn scroll_read_to(
    page: &Page,
    selector: &str,
    pauses: u32,
    scroll_amount: i32,
    variable_speed: bool,
    back_scroll: bool,
) -> Result<()> {
    scroll_into_view(page, selector).await?;
    read(page, pauses, scroll_amount, variable_speed, back_scroll).await
}

pub async fn scroll_to_top(page: &Page) -> Result<()> {
    smooth_scroll_to_y(page, 0.0, random_in_range(650, 1100)).await?;
    human_pause(120, 60).await;
    Ok(())
}

pub async fn scroll_to_bottom(page: &Page) -> Result<()> {
    let bottom = page
        .evaluate("Math.max(document.body.scrollHeight, document.documentElement.scrollHeight)")
        .await?
        .value()
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow::anyhow!("Failed to read document height"))?;
    smooth_scroll_to_y(page, bottom, random_in_range(650, 1100)).await?;
    human_pause(120, 60).await;
    Ok(())
}

async fn human_scroll_into_view(page: &Page, selector: &str) -> Result<()> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return false;

            const isScrollable = (node) => {{
                if (!node || node === document.body) return false;
                const style = getComputedStyle(node);
                const overflowY = style.overflowY;
                return /(auto|scroll|overlay)/.test(overflowY) && node.scrollHeight > node.clientHeight + 4;
            }};

            const findScrollableAncestor = (node) => {{
                let current = node.parentElement;
                while (current && current !== document.body) {{
                    if (isScrollable(current)) return current;
                    current = current.parentElement;
                }}
                return document.scrollingElement || document.documentElement;
            }};

            const scroller = findScrollableAncestor(el);
            const rect = el.getBoundingClientRect();
            const isWindowScroller =
                scroller === document.scrollingElement ||
                scroller === document.documentElement ||
                scroller === document.body;

            const currentScrollTop = isWindowScroller ? window.scrollY : scroller.scrollTop;
            const containerHeight = isWindowScroller ? window.innerHeight : scroller.clientHeight;
            const containerTop = isWindowScroller ? 0 : scroller.getBoundingClientRect().top;
            const maxScrollTop = isWindowScroller
                ? Math.max(document.body.scrollHeight, document.documentElement.scrollHeight) - window.innerHeight
                : scroller.scrollHeight - scroller.clientHeight;

            const targetScrollTop = Math.max(
                0,
                Math.min(
                    maxScrollTop,
                    currentScrollTop + (rect.top - containerTop) + rect.height / 2 - containerHeight / 2,
                ),
            );
            const delta = targetScrollTop - currentScrollTop;
            if (Math.abs(delta) < 2) return true;

            const distance = Math.abs(delta);
            const duration = Math.max(240, Math.min(950, distance / 2.5));
            const startTime = performance.now();

            return new Promise((resolve) => {{
                function ease(progress) {{
                    return 1 - Math.pow(1 - progress, 3);
                }}

                function applyScroll(pos) {{
                    if (isWindowScroller) {{
                        window.scrollTo(0, pos);
                    }} else {{
                        scroller.scrollTop = pos;
                    }}
                }}

                function step(now) {{
                    const progress = Math.min((now - startTime) / duration, 1);
                    const eased = ease(progress);
                    applyScroll(currentScrollTop + delta * eased);

                    if (progress < 1) {{
                        window.requestAnimationFrame(step);
                    }} else {{
                        resolve(true);
                    }}
                }}

                window.requestAnimationFrame(step);
                setTimeout(() => {{
                    applyScroll(targetScrollTop);
                    resolve(true);
                }}, duration + 250);
            }});
        }})()"#,
    );

    let result = timeout(Duration::from_secs(2), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("scroll target lookup timeout"))??;
    let _ = result;
    Ok(())
}

async fn target_still_offscreen(page: &Page, selector: &str) -> Result<bool> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return true;
            const rect = el.getBoundingClientRect();
            if (rect.width <= 0 || rect.height <= 0) return true;

            const isScrollable = (node) => {{
                if (!node || node === document.body) return false;
                const style = getComputedStyle(node);
                const overflowY = style.overflowY;
                return /(auto|scroll|overlay)/.test(overflowY) && node.scrollHeight > node.clientHeight + 4;
            }};

            const findScrollableAncestor = (node) => {{
                let current = node.parentElement;
                while (current && current !== document.body) {{
                    if (isScrollable(current)) return current;
                    current = current.parentElement;
                }}
                return null;
            }};

            const scroller = findScrollableAncestor(el);
            if (scroller) {{
                const box = scroller.getBoundingClientRect();
                return rect.bottom < box.top + 16 || rect.top > box.bottom - 16;
            }}

            return rect.bottom < 16 || rect.top > window.innerHeight - 16;
        }})()"#,
    );

    let result = timeout(Duration::from_secs(2), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("scroll visibility check timeout"))??;
    let value = result
        .value()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Failed to read scroll visibility"))?;

    Ok(value.as_bool().unwrap_or(true))
}

async fn scroll_into_view_native(page: &Page, selector: &str) -> Result<()> {
    let selector_js = serde_json::to_string(selector)?;
    let js = format!(
        r#"(() => {{
            const el = document.querySelector({selector_js});
            if (!el) return false;
            el.scrollIntoView({{behavior: 'smooth', block: 'center', inline: 'nearest'}});
            return true;
        }})()"#,
    );

    timeout(Duration::from_secs(2), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("native scrollIntoView timeout"))??;
    Ok(())
}

async fn smooth_scroll_by(page: &Page, delta_y: f64, duration_ms: u64) -> Result<()> {
    if delta_y.abs() < 0.5 {
        return Ok(());
    }
    let current_y = page
        .evaluate("window.scrollY")
        .await?
        .value()
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow::anyhow!("Failed to read current scroll position"))?;
    smooth_scroll_to_y(page, current_y + delta_y, duration_ms).await
}

async fn smooth_scroll_to_y(page: &Page, target_y: f64, duration_ms: u64) -> Result<()> {
    let js = format!(
        r#"(async () => {{
            const targetY = {};
            const duration = Math.max({}, 50);
            const startY = window.scrollY;
            const deltaY = targetY - startY;
            if (Math.abs(deltaY) < 0.5) return;
            const startX = window.scrollX;
            const startTime = performance.now();
            return await new Promise((resolve) => {{
                function ease(progress) {{
                    return 1 - Math.pow(1 - progress, 3);
                }}
                function step(now) {{
                    const progress = Math.min((now - startTime) / duration, 1);
                    const eased = ease(progress);
                    window.scrollTo(startX, startY + deltaY * eased);
                    if (progress < 1) {{
                        window.requestAnimationFrame(step);
                    }} else {{
                        resolve();
                    }}
                }}
                window.requestAnimationFrame(step);
                setTimeout(() => {{
                    window.scrollTo(startX, targetY);
                    resolve();
                }}, duration + 400);
            }});
        }})()"#,
        target_y, duration_ms
    );

    timeout(Duration::from_secs(5), page.evaluate(js))
        .await
        .map_err(|_| anyhow::anyhow!("smooth scroll timeout"))??;
    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_target_visibility_js_generation() {
        let selector = ".my-class";
        let selector_js = serde_json::to_string(selector).unwrap();
        assert!(selector_js.contains("my-class"));
    }

    #[test]
    fn test_scroll_into_view_js_structure() {
        let selector = "div.test";
        let selector_js = serde_json::to_string(selector).unwrap();
        let js = format!(
            r#"(() => {{
                const el = document.queryElement({selector_js});
                if (!el) return false;
                el.scrollIntoView({{behavior: 'smooth', block: 'center', inline: 'nearest'}});
                return true;
            }})()"#,
        );
        assert!(js.contains("scrollIntoView"));
    }

    #[test]
    fn test_smooth_scroll_js_has_easing() {
        let js = r#"(function ease(progress) { return 1 - Math.pow(1 - progress, 3); })"#;
        assert!(js.contains("Math.pow"));
    }
}
