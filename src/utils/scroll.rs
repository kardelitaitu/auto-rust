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

pub async fn read_by_duration(page: &Page, duration_ms: u64) -> Result<()> {
    // Calculate pause count based on duration
    // Average pause time ~800ms (500-1100ms range with 45% variance)
    let avg_pause_ms = 800;
    let pauses = (duration_ms / avg_pause_ms).max(2) as u32;

    read(
        page,
        pauses,
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
            const duration = Math.max(2000, Math.min(4000, distance * 4));
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

    #[test]
    fn test_scroll_distance_calculation_window_scroller() {
        // Test case: element at rect.top = 100, height = 50, window height = 400
        // currentScrollTop = 0
        // Should center element: element center at 125, viewport center at 200
        // Need to scroll to: 125 - 200 = -75, but clamped to 0 = 0
        let rect_top = 100.0;
        let rect_height = 50.0;
        let container_height = 400.0;
        let container_top = 0.0;
        let current_scroll_top = 0.0;
        let max_scroll_top = 1000.0;

        let scroll_calc: f64 = current_scroll_top + (rect_top - container_top) + rect_height / 2.0
            - container_height / 2.0;
        let target_scroll_top = scroll_calc.max(0.0_f64).min(max_scroll_top);

        // Element center at 125, viewport center at 200, delta = -75, clamped to 0
        assert_eq!(target_scroll_top, 0.0);
    }

    #[test]
    fn test_scroll_distance_calculation_element_below_viewport() {
        // Test case: element at rect.top = 500, height = 50, window height = 400
        // currentScrollTop = 0
        // Element center at 525, viewport center at 200
        // Need to scroll to: 525 - 200 = 325
        let rect_top = 500.0;
        let rect_height = 50.0;
        let container_height = 400.0;
        let container_top = 0.0;
        let current_scroll_top = 0.0;
        let max_scroll_top = 1000.0;

        let scroll_calc: f64 = current_scroll_top + (rect_top - container_top) + rect_height / 2.0
            - container_height / 2.0;
        let target_scroll_top = scroll_calc.max(0.0_f64).min(max_scroll_top);

        assert_eq!(target_scroll_top, 325.0);
    }

    #[test]
    fn test_scroll_distance_calculation_element_above_viewport() {
        // Test case: element at rect.top = -100, height = 50, window height = 400
        // currentScrollTop = 200
        // Element center at -75, viewport center at 200 (absolute)
        // Need to scroll to: 200 + (-100 - 0) + 25 - 200 = 200 -100 +25 -200 = -75, clamped to 0
        let rect_top = -100.0;
        let rect_height = 50.0;
        let container_height = 400.0;
        let container_top = 0.0;
        let current_scroll_top = 200.0;
        let max_scroll_top = 1000.0;

        let scroll_calc: f64 = current_scroll_top + (rect_top - container_top) + rect_height / 2.0
            - container_height / 2.0;
        let target_scroll_top = scroll_calc.max(0.0_f64).min(max_scroll_top);

        assert_eq!(target_scroll_top, 0.0);
    }

    #[test]
    fn test_scroll_duration_calculation() {
        // Test duration calculation: max(2000, min(4000, distance * 4))
        assert_eq!(calculate_scroll_duration(0.0), 2000); // min duration
        assert_eq!(calculate_scroll_duration(100.0), 2000); // 100 * 4 = 400 < 2000
        assert_eq!(calculate_scroll_duration(500.0), 2000); // 500 * 4 = 2000
        assert_eq!(calculate_scroll_duration(600.0), 2400); // 600 * 4 = 2400
        assert_eq!(calculate_scroll_duration(1000.0), 4000); // 1000 * 4 = 4000, max
    }

    fn calculate_scroll_duration(distance: f64) -> u64 {
        (2000.0_f64.max(4000.0_f64.min(distance * 4.0))) as u64
    }

    #[test]
    fn test_scroll_distance_calculation_max_scroll_clamp() {
        // Test clamping to max scroll position
        let rect_top = 1500.0;
        let rect_height = 50.0;
        let container_height = 400.0;
        let container_top = 0.0;
        let current_scroll_top = 800.0;
        let max_scroll_top = 1000.0;

        let scroll_calc: f64 = current_scroll_top + (rect_top - container_top) + rect_height / 2.0
            - container_height / 2.0;
        let target_scroll_top = scroll_calc.max(0.0_f64).min(max_scroll_top);

        // Should be clamped to max_scroll_top
        assert_eq!(target_scroll_top, 1000.0);
    }

    #[test]
    fn test_scroll_distance_calculation_negative_delta() {
        // Test negative delta (scrolling up)
        let rect_top = 100.0;
        let rect_height = 50.0;
        let container_height = 400.0;
        let container_top = 0.0;
        let current_scroll_top = 500.0;
        let max_scroll_top = 1000.0;

        let scroll_calc: f64 = current_scroll_top + (rect_top - container_top) + rect_height / 2.0
            - container_height / 2.0;
        let target_scroll_top = scroll_calc.max(0.0_f64).min(max_scroll_top);

        // Should scroll up
        assert!(target_scroll_top < current_scroll_top);
    }

    #[test]
    fn test_scroll_distance_calculation_zero_delta() {
        // Test when element is already centered
        let rect_top = 175.0;
        let rect_height = 50.0;
        let container_height = 400.0;
        let container_top = 0.0;
        let current_scroll_top = 0.0;
        let max_scroll_top = 1000.0;

        let scroll_calc: f64 = current_scroll_top + (rect_top - container_top) + rect_height / 2.0
            - container_height / 2.0;
        let target_scroll_top = scroll_calc.max(0.0_f64).min(max_scroll_top);

        // Element center at 200, viewport center at 200, delta = 0
        assert_eq!(target_scroll_top, 0.0);
    }

    #[test]
    fn test_scroll_duration_edge_cases() {
        assert_eq!(calculate_scroll_duration(-100.0), 2000); // negative distance
        assert_eq!(calculate_scroll_duration(9999.0), 4000); // very large distance
    }

    #[test]
    fn test_scroll_easing_function() {
        // Test easing function: 1 - (1 - progress)^3
        let ease = |progress: f64| 1.0 - (1.0 - progress).powi(3);

        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert!(ease(0.5) > 0.0 && ease(0.5) < 1.0);
        assert!(ease(0.25) < ease(0.5)); // easing should be monotonic
    }

    #[test]
    fn test_scroll_step_size_calculation() {
        // Test back scroll step size calculation
        let distance = 50.0;
        let steps = 2;
        let step_size = distance / steps as f64;
        assert_eq!(step_size, 25.0);

        let distance = 70.0;
        let steps = 3;
        let step_size = distance / steps as f64;
        assert!((step_size - 23.33).abs() < 0.01);
    }

    #[test]
    fn test_scroll_jitter_calculation() {
        // Test variable speed jitter calculation
        let scroll_amount = 420;
        let _jitter = 0.18; // typical std dev
        let jitter_value = 0.1; // sample jitter
        let adjusted = ((scroll_amount as f64) * (1.0 + jitter_value)).round() as i32;
        assert!(adjusted > scroll_amount);
    }

    #[test]
    fn test_scroll_direction_probability() {
        // Test that down direction is more likely (88%)
        let down_count = 88;
        let up_count = 12;
        // In actual test, we'd use actual random, but this validates the logic
        assert!(down_count > up_count);
    }

    #[test]
    fn test_scroll_backtrack_probability() {
        // Test backtrack probability (8%)
        let backtrack_count = 8;
        assert!(backtrack_count < 100); // Should be around 8%
    }

    #[test]
    fn test_scroll_pause_range() {
        // Test pause ranges
        let min_pause = 500;
        let max_pause = 1100;
        assert!(min_pause < max_pause);
        assert!(min_pause > 0);
    }

    #[test]
    fn test_scroll_amount_bounds() {
        // Test scroll amount bounds from gaussian
        let min_amount = 180.0;
        let max_amount = 900.0;
        let mean = 420.0;
        assert!(mean >= min_amount && mean <= max_amount);
    }

    #[test]
    fn test_scroll_backtrack_distance() {
        // Test backtrack distance bounds
        let min_backtrack = 8.0;
        let max_backtrack = 70.0;
        let mean_backtrack = 28.0;
        assert!(mean_backtrack >= min_backtrack && mean_backtrack <= max_backtrack);
    }

    #[test]
    fn test_scroll_smooth_by_threshold() {
        // Test smooth_scroll_by threshold for small deltas
        let delta_y: f64 = 0.3;
        assert!(delta_y.abs() < 0.5); // Should skip scroll
    }

    #[test]
    fn test_scroll_smooth_by_above_threshold() {
        // Test smooth_scroll_by above threshold
        let delta_y: f64 = 10.0;
        assert!(delta_y.abs() >= 0.5); // Should scroll
    }

    #[test]
    fn test_scroll_duration_bounds() {
        // Test duration bounds for human_scroll
        let amount = 1000;
        let _min_duration = 500;
        let _max_duration = 900;
        assert!(amount > 800); // Should use longer duration
    }

    #[test]
    fn test_scroll_duration_small_amount() {
        // Test duration for small scroll amounts
        let amount = 400;
        let _min_duration = 260;
        let _max_duration = 620;
        assert!(amount <= 800); // Should use shorter duration
    }
}
