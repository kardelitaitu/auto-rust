# Phase 3: Utils Library (Human-Like Behaviors)

**Duration:** 2-4 days  
**Goal:** Build reusable helpers in `src/utils/` that replicate all human-like behaviors from the Node.js version. Export everything from `src/utils/mod.rs` for easy importing.

---

## 3.1 Math Utilities (`src/utils/math.rs`)

### Reference: Node.js `api/utils/math.js`

The Node.js version provides:
- `gaussian(mean, dev, min, max)` - Box-Muller Transform for normal distribution
- `randomInRange(min, max)` - Uniform random integer
- `roll(threshold)` - Boolean probability check
- `sample(array)` - Random element from array
- `pidStep(state, target, model, dt)` - PID Controller for mouse physics

### Rust Implementation

```rust
use rand::Rng;
use rand_distr::{Distribution, Normal};

/// Generate a random integer within [min, max] inclusive
/// Equivalent to Node.js: mathUtils.randomInRange(min, max)
pub fn random_in_range(min: u64, max: u64) -> u64 {
    if min >= max {
        return min;
    }
    
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}

/// Generate a random float within [min, max] inclusive
pub fn random_in_range_f64(min: f64, max: f64) -> f64 {
    if min >= max {
        return min;
    }
    
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}

/// Box-Muller Transform — generates a number normally distributed around a mean
/// Equivalent to Node.js: mathUtils.gaussian(mean, dev, min, max)
/// 
/// # Example
/// ```
/// let value = gaussian(30000.0, 10000.0, 10000.0, 45000.0);
/// // Returns ~30000 with std_dev 10000, clamped between 10000-45000
/// ```
pub fn gaussian(mean: f64, dev: f64, min: f64, max: f64) -> f64 {
    if dev <= 0.0 {
        return mean.clamp(min, max);
    }

    // Box-Muller Transform
    let mut rng = rand::thread_rng();
    
    let u1: f64 = rng.gen_range(0.0001..=1.0); // Avoid log(0)
    let u2: f64 = rng.gen();
    
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    let mut result = mean + z * dev;
    
    // Clamp to bounds
    result = result.max(min);
    result = result.min(max);
    
    result
}

/// Generate a gaussian-distributed integer
pub fn gaussian_int(mean: f64, dev: f64, min: f64, max: f64) -> i64 {
    gaussian(mean, dev, min, max).round() as i64
}

/// Returns true if a random value is below the threshold (0.0 to 1.0)
/// Equivalent to Node.js: mathUtils.roll(threshold)
pub fn roll(threshold: f64) -> bool {
    let mut rng = rand::thread_rng();
    rng.gen_bool(threshold.clamp(0.0, 1.0))
}

/// Returns a random element from a slice
/// Equivalent to Node.js: mathUtils.sample(array)
pub fn sample<T: Clone>(slice: &[T]) -> Option<T> {
    if slice.is_empty() {
        return None;
    }
    
    let mut rng = rand::thread_rng();
    Some(slice[rng.gen_range(0..slice.len())].clone())
}

/// Returns a random index from a slice
pub fn sample_index(len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    
    let mut rng = rand::thread_rng();
    Some(rng.gen_range(0..len))
}

/// PID Controller state
#[derive(Debug, Clone)]
pub struct PidState {
    pub pos: f64,
    pub integral: f64,
    pub prev_error: f64,
}

impl PidState {
    pub fn new(initial_pos: f64) -> Self {
        Self {
            pos: initial_pos,
            integral: 0.0,
            prev_error: 0.0,
        }
    }
}

/// PID Controller model parameters
#[derive(Debug, Clone)]
pub struct PidModel {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
}

impl Default for PidModel {
    fn default() -> Self {
        Self {
            kp: 1.0,
            ki: 0.1,
            kd: 0.05,
        }
    }
}

/// PID Controller step
/// Equivalent to Node.js: mathUtils.pidStep(state, target, model, dt)
pub fn pid_step(state: &mut PidState, target: f64, model: &PidModel, dt: f64) -> f64 {
    let error = target - state.pos;
    
    // Integral with anti-windup clamping
    state.integral += error * dt;
    state.integral = state.integral.clamp(-10.0, 10.0);
    
    // Derivative
    let derivative = (error - state.prev_error) / dt;
    
    // PID output
    let output = model.kp * error + model.ki * state.integral + model.kd * derivative;
    
    state.prev_error = error;
    state.pos += output;
    
    state.pos
}
```

---

## 3.2 Timing Utilities (`src/utils/timing.rs`)

### Reference: Node.js `api/behaviors/timing.js` + `api/utils/timing.js`

The Node.js version provides:
- `think(ms?)` - Persona-aware "thinking" pause (1-5s Gaussian, performance-aware impatience)
- `delay(ms)` - Humanized delay with Gaussian jitter
- `humanDelay(ms, options)` - Applies jitter to base delay
- `humanTiming` module with Gaussian distribution

### Rust Implementation

```rust
use tokio::time::{sleep, Duration};
use crate::utils::math::{gaussian, random_in_range};
use tracing::debug;

/// Humanized delay with Gaussian jitter
/// Equivalent to Node.js: delay(ms) with humanDelay wrapper
/// 
/// # Example
/// ```
/// human_delay(1000).await; // Delays ~1000ms with Gaussian variance
/// ```
pub async fn human_delay(base_ms: u64) {
    // Apply 20% jitter around the base
    let std_dev = (base_ms as f64 * 0.2) as f64;
    let jittered = gaussian(
        base_ms as f64,
        std_dev,
        (base_ms as f64 * 0.8) as f64,
        (base_ms as f64 * 1.2) as f64,
    );
    
    sleep(Duration::from_millis(jittered as u64)).await;
}

/// Random "thinking" pause. Simulates cognitive decision-making.
/// If no argument: 1-5s with Gaussian distribution.
/// Equivalent to Node.js: think(ms)
/// 
/// # Example
/// ```
/// thinking_pause().await;        // Random 1-5s pause
/// thinking_pause_ms(2000).await; // ~2s pause with jitter
/// ```
pub async fn thinking_pause() {
    let base = random_in_range(1000, 5000);
    human_delay(base).await;
}

/// Thinking pause with specific base duration
pub async fn thinking_pause_ms(base_ms: u64) {
    // Apply performance-aware impatience (Ghost 3.0)
    // In Rust, we can't easily check page performance metrics,
    // but we can still apply persona-driven variance
    let adjusted = (base_ms as f64 * 0.9).round() as u64; // Slightly impatient by default
    human_delay(adjusted).await;
}

/// Human-like pause with configurable range
/// Equivalent to Node.js: wait(randomInRange(min, max))
pub async fn random_pause(min_ms: u64, max_ms: u64) {
    let delay = random_in_range(min_ms, max_ms);
    human_delay(delay).await;
}

/// Persona-driven pause with mean and variance
pub async fn persona_pause(mean_ms: f64, std_dev_ms: f64, min_ms: f64, max_ms: f64) {
    let delay = gaussian(mean_ms, std_dev_ms, min_ms, max_ms);
    sleep(Duration::from_millis(delay as u64)).await;
}

/// Post-action observation pause (200-600ms)
pub async fn post_action_pause() {
    random_pause(200, 600).await;
}

/// Pre-action hesitation (100-400ms)
pub async fn pre_action_hesitation() {
    random_pause(100, 400).await;
}

/// Content-aware pause: longer for text-heavy content
/// This is a simplified version - in production, you'd analyze page content
pub async fn content_aware_pause(is_text_heavy: bool) {
    if is_text_heavy {
        persona_pause(3000.0, 1000.0, 1500.0, 5000.0).await;
    } else {
        persona_pause(1500.0, 500.0, 800.0, 2500.0).await;
    }
}
```

---

## 3.3 Navigation Utilities (`src/utils/navigation.rs`)

### Reference: Node.js `api/interactions/navigation.js`

The Node.js version provides:
- `goto(url, options)` - Navigate with warmup, referrer, banner handling
- `reload(options)` - Reload current page
- `back(options)` - Go back in history with timeout
- `forward()` - Go forward in history

### Rust Implementation

```rust
use chromiumoxide::Page;
use anyhow::Result;
use tracing::{info, debug, warn};
use crate::utils::timing::{human_delay, random_pause};
use crate::utils::mouse::random_wheel_scroll;

/// Navigation options
#[derive(Debug, Clone)]
pub struct NavigationOptions {
    /// Wait until this event: "load", "domcontentloaded", "networkidle"
    pub wait_until: String,
    /// Navigation timeout in milliseconds
    pub timeout_ms: u64,
    /// Referrer URL
    pub referer: Option<String>,
    /// Enable pre-navigation warmup
    pub warmup: bool,
    /// Warmup: random mouse movement
    pub warmup_mouse: bool,
    /// Warmup: pause before navigation
    pub warmup_pause: bool,
    /// Auto-handle cookie banners after load
    pub auto_banners: bool,
}

impl Default for NavigationOptions {
    fn default() -> Self {
        Self {
            wait_until: "domcontentloaded".to_string(),
            timeout_ms: 30000,
            referer: None,
            warmup: true,
            warmup_mouse: true,
            warmup_pause: true,
            auto_banners: true,
        }
    }
}

/// Navigate to a URL with human-like behavior
/// Equivalent to Node.js: api.goto(url, options)
/// 
/// # Example
/// ```
/// goto(page, "https://example.com", NavigationOptions::default()).await?;
/// ```
pub async fn goto(page: &Page, url: &str, options: NavigationOptions) -> Result<()> {
    info!("Navigating to: {}", url);

    // Pre-navigation warmup
    if options.warmup {
        if options.warmup_mouse {
            // Random mouse movement to simulate human presence
            random_pause(200, 500).await;
        }
        
        if options.warmup_pause {
            // "Thinking" pause before action
            human_delay(random_in_range(500, 1500)).await;
        }
    }

    // Navigate
    let mut params = chromiumoxide::cdp::browser_protocol::page::NavigateParams::new(url);
    
    // Set timeout (adjust API based on chromiumoxide version)
    // Note: chromiumoxide may handle timeout differently
    
    let _result = page.navigate(url).await?;

    // Post-navigation: initial scroll to center (human signature)
    // Mirrors Node.js: page.mouse.wheel(0, randomInRange(100, 300))
    random_pause(500, 1500).await;
    
    if rand::random::<f64>() > 0.3 {
        let scroll_amount = random_in_range(100, 300);
        random_wheel_scroll(page, scroll_amount as i64).await?;
    }

    // Auto-handle cookie banners (stub - implement banner detection later)
    if options.auto_banners {
        // TODO: Implement cookie banner detection and auto-accept
        debug!("Cookie banner auto-handling (stub)");
    }

    debug!("Navigation complete: {}", url);
    Ok(())
}

/// Navigate and wait for full page load
pub async fn goto_and_wait(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    let options = NavigationOptions {
        wait_until: "load".to_string(),
        timeout_ms,
        ..Default::default()
    };
    
    goto(page, url, options).await
}

/// Reload the current page
/// Equivalent to Node.js: api.reload(options)
pub async fn reload(page: &Page, timeout_ms: u64) -> Result<()> {
    info!("Reloading page");
    
    // chromiumoxide equivalent to page.reload()
    page.reload().await?;
    
    human_delay(500).await;
    Ok(())
}

/// Go back in browser history with bounded timeout
/// Equivalent to Node.js: api.back(options)
/// Returns true if navigation occurred, false if no history
pub async fn go_back(page: &Page, timeout_ms: u64) -> Result<bool> {
    debug!("Going back in history");
    
    // Note: chromiumoxide API may differ
    // This is a placeholder - adjust based on actual API
    tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        async {
            // page.go_back() equivalent
            // Return false if no history available
            Ok(false)
        }
    )
    .await
    .unwrap_or(Ok(false))
}

/// Go forward in browser history
pub async fn go_forward(page: &Page) -> Result<()> {
    debug!("Going forward in history");
    
    // page.go_forward() equivalent
    Ok(())
}

/// Wait for element to be visible
pub async fn wait_for_selector(page: &Page, selector: &str, timeout_ms: u64) -> Result<()> {
    use tokio::time::{timeout, Duration};
    
    timeout(Duration::from_millis(timeout_ms), async {
        // Poll for element visibility
        loop {
            // Check if element exists and is visible
            // This requires JavaScript evaluation via chromiumoxide
            let visible = page
                .evaluate_expression(format!(
                    "(() => {{
                        const el = document.querySelector('{}');
                        if (!el) return false;
                        const rect = el.getBoundingClientRect();
                        return rect.width > 0 && rect.height > 0;
                    }})()",
                    selector
                ))
                .await
                .unwrap_or(serde_json::Value::Bool(false))
                .as_bool()
                .unwrap_or(false);
            
            if visible {
                return Ok(());
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("Timeout waiting for selector: {}", selector))?
}

use crate::utils::math::random_in_range;
```

---

## 3.4 Scroll Utilities (`src/utils/scroll.rs`)

### Reference: Node.js `api/interactions/scroll.js` (550+ lines)

The Node.js version provides:
- `read(target, options)` - Stop-and-read scroll pattern with persona-aware pauses
- `focus(selector)` - Golden View: center element in viewport
- `focus2(selector)` - Improved golden view with absolute coordinates
- `scroll(distance)` - Blind vertical scroll
- `toTop()`, `toBottom()` - Scroll to edges
- `back(distance)` - Scroll up (re-read)
- RAF-based `_smoothScroll()` with easing
- Content-aware pause weighting
- Micro-drift simulation during pauses
- Lateral sway during scrolls

### Rust Implementation

```rust
use chromiumoxide::Page;
use anyhow::Result;
use tracing::debug;
use crate::utils::timing::{human_delay, random_pause, persona_pause};
use crate::utils::math::{random_in_range, gaussian};

/// Scroll options
#[derive(Debug, Clone)]
pub struct ScrollOptions {
    /// Number of pauses during scroll (higher = slower, more human-like)
    pub pauses: u32,
    /// Amount to scroll per step (pixels)
    pub scroll_amount: u64,
    /// Enable variable speed (slower at start/end, faster in middle)
    pub variable_speed: bool,
    /// Enable back-scroll at end (simulates re-reading)
    pub back_scroll: bool,
}

impl Default for ScrollOptions {
    fn default() -> Self {
        Self {
            pauses: 5,
            scroll_amount: 600,
            variable_speed: true,
            back_scroll: true,
        }
    }
}

/// Human-like scroll: read pattern with pauses
/// Equivalent to Node.js: api.scroll.read(null, options)
/// 
/// Scrolls down the page with random pauses, simulating human reading behavior.
/// 
/// # Example
/// ```
/// human_scroll_read(page, ScrollOptions {
///     pauses: 6,
///     scroll_amount: 800,
///     ..Default::default()
/// }).await?;
/// ```
pub async fn human_scroll_read(page: &Page, options: ScrollOptions) -> Result<()> {
    debug!(
        "Starting scroll read: {} pauses, {}px per step",
        options.pauses, options.scroll_amount
    );

    let total_scrolled: u64 = 0;

    for i in 0..options.pauses {
        // Variable speed: easeOutExpo-like behavior
        let scroll_amount = if options.variable_speed {
            // Slower at start, faster in middle, slower at end
            let progress = i as f64 / options.pauses as f64;
            let speed_factor = if progress < 0.2 {
                0.6 // Start slow
            } else if progress < 0.8 {
                1.2 // Middle: faster
            } else {
                0.7 // End: slow down
            };
            
            (options.scroll_amount as f64 * speed_factor).round() as u64
        } else {
            options.scroll_amount
        };

        // Scroll down
        scroll_down(page, scroll_amount).await?;

        // Content-aware pause
        let pause_base = if options.variable_speed {
            gaussian(2000.0, 800.0, 800.0, 4000.0)
        } else {
            gaussian(1500.0, 500.0, 500.0, 2500.0)
        };

        persona_pause(pause_base, pause_base * 0.3, pause_base * 0.5, pause_base * 1.5).await;

        // Micro-drift: tiny scroll during pause (simulates mouse wheel fidgeting)
        if rand::random::<f64>() > 0.5 {
            let drift = random_in_range(10, 50);
            scroll_down(page, drift).await?;
        }
    }

    // Back-scroll: simulate re-reading
    if options.back_scroll && rand::random::<f64>() > 0.3 {
        let back_amount = random_in_range(200, 600);
        debug!("Back-scrolling {}px (re-read simulation)", back_amount);
        scroll_up(page, back_amount).await?;
        random_pause(1000, 3000).await;
    }

    debug!("Scroll read complete: {} total pauses", options.pauses);
    Ok(())
}

/// Blind vertical scroll (no pauses)
/// Equivalent to Node.js: api.scroll.scroll(distance)
pub async fn scroll_down(page: &Page, distance: u64) -> Result<()> {
    // JavaScript-based scroll (works across all chromiumoxide versions)
    page.evaluate_expression(&format!(
        "window.scrollBy(0, {})",
        distance
    ))
    .await?;
    
    Ok(())
}

/// Scroll up
/// Equivalent to Node.js: api.scroll.back(distance)
pub async fn scroll_up(page: &Page, distance: u64) -> Result<()> {
    page.evaluate_expression(&format!(
        "window.scrollBy(0, -{})",
        distance
    ))
    .await?;
    
    Ok(())
}

/// Scroll to top of page
pub async fn scroll_to_top(page: &Page) -> Result<()> {
    page.evaluate_expression("window.scrollTo(0, 0)").await?;
    human_delay(300).await;
    Ok(())
}

/// Scroll to bottom of page
pub async fn scroll_to_bottom(page: &Page) -> Result<()> {
    page.evaluate_expression(
        "window.scrollTo(0, document.body.scrollHeight)"
    ).await?;
    human_delay(300).await;
    Ok(())
}

/// Golden View: scroll element into center of viewport
/// Equivalent to Node.js: api.scroll.focus(selector)
/// Scrolls until element is visible in the center of the viewport
pub async fn focus_element(page: &Page, selector: &str) -> Result<()> {
    debug!("Focusing element: {}", selector);
    
    // Scroll element into view with centering
    let result = page.evaluate_expression(&format!(
        "(() => {{
            const el = document.querySelector('{selector}');
            if (!el) return {{ success: false, reason: 'not_found' }};
            
            const rect = el.getBoundingClientRect();
            const viewportHeight = window.innerHeight;
            
            // Calculate scroll position to center element
            const targetScrollY = window.scrollY + rect.top - (viewportHeight / 2) + (rect.height / 2);
            
            window.scrollTo({{
                top: targetScrollY,
                behavior: 'smooth'
            }});
            
            return {{ success: true, x: rect.left + rect.width/2, y: rect.top + rect.height/2 }};
        }})()"
    )).await?;
    
    // Check if successful
    // Parse result JSON if needed
    
    human_delay(500).await;
    Ok(())
}

/// Improved golden view with absolute document coordinates
/// Equivalent to Node.js: api.scroll.focus2(selector)
pub async fn focus_element_absolute(page: &Page, selector: &str) -> Result<()> {
    debug!("Focusing element (absolute): {}", selector);
    
    // Get element's absolute position
    let result = page.evaluate_expression(&format!(
        "(() => {{
            const el = document.querySelector('{selector}');
            if (!el) return null;
            
            const rect = el.getBoundingClientRect();
            return {{
                x: rect.left + window.scrollX + rect.width / 2,
                y: rect.top + window.scrollY + rect.height / 2,
                width: rect.width,
                height: rect.height
            }};
        }})()"
    )).await?;
    
    // Scroll to position
    // Result parsing depends on chromiumoxide's return type
    
    human_delay(500).await;
    Ok(())
}

/// Smooth scroll with easing (easeOutQuart)
async fn smooth_scroll(page: &Page, from: u64, to: u64, duration_ms: u64) -> Result<()> {
    // JavaScript-based smooth scroll with easing
    page.evaluate_expression(&format!(
        "(() => {{
            const start = window.scrollY;
            const end = {};
            const duration = {};
            const startTime = performance.now();
            
            function easeOutQuart(t) {{
                return 1 - Math.pow(1 - t, 4);
            }}
            
            function step(currentTime) {{
                const elapsed = currentTime - startTime;
                const progress = Math.min(elapsed / duration, 1);
                const easedProgress = easeOutQuart(progress);
                
                window.scrollTo(0, start + (end - start) * easedProgress);
                
                if (progress < 1) {{
                    requestAnimationFrame(step);
                }}
            }}
            
            requestAnimationFrame(step);
        }})()",
        to, duration_ms
    )).await?;
    
    // Wait for animation to complete
    tokio::time::sleep(std::time::Duration::from_millis(duration_ms)).await;
    
    Ok(())
}

use rand;
```

---

## 3.5 Mouse Utilities (`src/utils/mouse.rs`)

### Reference: Node.js `api/interactions/cursor.js` + `api/utils/ghostCursor.js`

The Node.js version provides:
- `GhostCursor` class with Bezier arc paths
- Fitts's Law duration calculation: `T = a + b * log2(2D/W)`
- Overshoot + correction patterns (20% chance for >500px moves)
- Hover with micro-drift noise
- Path styles: bezier, arc, zigzag, overshoot, stopped, muscle (PID-driven)
- `startFidgeting()` / `stopFidgeting()` - Idle tremors

### Rust Implementation

```rust
use chromiumoxide::Page;
use anyhow::Result;
use tracing::debug;
use crate::utils::timing::{human_delay, random_pause};
use crate::utils::math::{random_in_range, gaussian, PidState, PidModel, pid_step};

/// Move mouse to coordinates with human-like Bezier path
/// Equivalent to Node.js: cursor.move(x, y)
/// 
/// Uses Bezier curves and Fitts's Law for natural movement
pub async fn mouse_move_to(page: &Page, target_x: f64, target_y: f64) -> Result<()> {
    // Get current mouse position (via JS)
    let current_pos = get_mouse_position(page).await?;
    
    let dx = target_x - current_pos.x;
    let dy = target_y - current_pos.y;
    let distance = (dx * dx + dy * dy).sqrt();
    
    // Fitts's Law duration calculation
    // T = a + b * log2(2D/W)
    // where a=0, b=0.1, W=100 (target width)
    let fitts_duration = if distance > 0.0 {
        let width = 100.0; // Assumed target width
        0.0 + 0.1 * (2.0 * distance / width).log2() * 1000.0 // Convert to ms
    } else {
        200.0
    };
    
    // Clamp duration
    let duration_ms = fitts_duration.clamp(200.0, 2000.0);
    
    // Generate Bezier path points
    let points = generate_bezier_path(
        current_pos.x, current_pos.y,
        target_x, target_y,
        distance,
    );
    
    // Move through path points
    for (i, point) in points.iter().enumerate() {
        let progress = i as f64 / points.len() as f64;
        
        // Move mouse to point
        dispatch_mouse_event(page, point.x, point.y).await?;
        
        // Variable delay between points (slower at start/end)
        let ease_factor = if progress < 0.2 {
            1.5 // Start slow
        } else if progress > 0.8 {
            1.3 // End slow (precision)
        } else {
            1.0 // Middle: normal speed
        };
        
        let point_delay = (duration_ms / points.len() as f64 * ease_factor) as u64;
        tokio::time::sleep(std::time::Duration::from_millis(point_delay)).await;
    }
    
    debug!("Mouse moved to ({}, {})", target_x, target_y);
    Ok(())
}

/// Mouse position
#[derive(Debug, Clone, Copy)]
pub struct MousePosition {
    pub x: f64,
    pub y: f64,
}

impl MousePosition {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Get current mouse position via JavaScript
async fn get_mouse_position(page: &Page) -> Result<MousePosition> {
    // Note: This requires tracking mouse position via JS events
    // For simplicity, we'll assume center of viewport as starting point
    // In production, you'd add event listeners to track position
    
    let viewport_size = page.evaluate_expression(
        "({ width: window.innerWidth, height: window.innerHeight })"
    ).await?;
    
    // Default to center (improve this with actual tracking)
    Ok(MousePosition {
        x: 400.0,
        y: 300.0,
    })
}

/// Generate Bezier curve path from current to target position
fn generate_bezier_path(
    from_x: f64, from_y: f64,
    to_x: f64, to_y: f64,
    distance: f64,
) -> Vec<MousePosition> {
    let num_points = if distance > 500.0 {
        20 // Long distance: more points
    } else if distance > 200.0 {
        12
    } else {
        8
    };
    
    // Bezier control point: offset perpendicular to line
    let mid_x = (from_x + to_x) / 2.0;
    let mid_y = (from_y + to_y) / 2.0;
    
    // Random arc offset (simulates natural hand movement)
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    let perpendicular_x = -dy;
    let perpendicular_y = dx;
    let perp_length = distance * 0.15 * rand::random::<f64>().signum();
    
    let control_x = mid_x + perpendicular_x / distance * perp_length;
    let control_y = mid_y + perpendicular_y / distance * perp_length;
    
    // Generate points along quadratic Bezier curve
    (0..=num_points)
        .map(|i| {
            let t = i as f64 / num_points as f64;
            let one_minus_t = 1.0 - t;
            
            let x = one_minus_t * one_minus_t * from_x 
                  + 2.0 * one_minus_t * t * control_x 
                  + t * t * to_x;
            let y = one_minus_t * one_minus_t * from_y 
                  + 2.0 * one_minus_t * t * control_y 
                  + t * t * to_y;
            
            MousePosition::new(x, y)
        })
        .collect()
}

/// Dispatch mouse move event via JavaScript
async fn dispatch_mouse_event(page: &Page, x: f64, y: f64) -> Result<()> {
    // Dispatch mousemove event (not visible to page, but affects browser state)
    page.evaluate_expression(&format!(
        "(() => {{
            // This is a no-op for tracking; actual mouse movement 
            // is handled by CDP in chromiumoxide
        }})()"
    )).await?;
    
    // Note: chromiumoxide has native mouse.move() via CDP
    // Use page.mouse().move(x, y) if available in your version
    
    Ok(())
}

/// Human-like click at coordinates
/// Moves cursor to target then clicks
pub async fn mouse_click(page: &Page, x: f64, y: f64) -> Result<()> {
    // Move to target
    mouse_move_to(page, x, y).await?;
    
    // Pre-click pause (human hesitation)
    random_pause(100, 300).await;
    
    // Click via JavaScript (or use chromiumoxide's native click)
    page.evaluate_expression(&format!(
        "(() => {{
            const event = new MouseEvent('click', {{
                view: window,
                bubbles: true,
                cancelable: true,
                clientX: {},
                clientY: {}
            }});
            document.elementFromPoint({}, {})?.dispatchEvent(event);
        }})()",
        x, y, x, y
    )).await?;
    
    // Post-click observation
    random_pause(200, 600).await;
    
    Ok(())
}

/// Hover with micro-drift noise
/// Equivalent to Node.js: cursor.hoverWithDrift()
pub async fn mouse_hover_with_drift(
    page: &Page,
    x: f64, y: f64,
    duration_ms: u64,
) -> Result<()> {
    // Move to target
    mouse_move_to(page, x, y).await?;
    
    // Micro-drift during hover (simulates hand tremor)
    let drift_steps = duration_ms / 100;
    for _ in 0..drift_steps {
        let drift_x = gaussian(0.0, 2.0, -5.0, 5.0);
        let drift_y = gaussian(0.0, 2.0, -5.0, 5.0);
        
        dispatch_mouse_event(page, x + drift_x, y + drift_y).await?;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    
    Ok(())
}

/// Random wheel scroll (for post-navigation scroll)
pub async fn random_wheel_scroll(page: &Page, distance: i64) -> Result<()> {
    page.evaluate_expression(&format!(
        "window.scrollBy(0, {})",
        distance
    )).await?;
    
    Ok(())
}

/// Start fidgeting: idle tremors when page is idle
/// Equivalent to Node.js: startFidgeting()
/// Returns a task that runs until cancelled
pub async fn start_fidgeting(page: &Page, cancel: tokio::sync::watch::Receiver<bool>) -> Result<()> {
    use tokio::time::{interval, Duration};
    
    let mut interval = interval(Duration::from_millis(2000));
    
    loop {
        interval.tick().await;
        
        // Check if cancelled
        if *cancel.borrow() {
            debug!("Fidgeting cancelled");
            break;
        }
        
        // Tiny random movement (simulates idle hand movement)
        let drift_x = gaussian(0.0, 5.0, -15.0, 15.0);
        let drift_y = gaussian(0.0, 5.0, -15.0, 15.0);
        
        dispatch_mouse_event(page, drift_x, drift_y).await?;
    }
    
    Ok(())
}

/// Overshoot + correction pattern (20% chance for >500px moves)
fn apply_overshoot(distance: f64) -> (f64, f64) {
    use rand;
    
    if distance > 500.0 && rand::random::<f64>() < 0.2 {
        // Overshoot by 10-20%
        let overshoot_factor = 1.0 + rand::random::<f64>() * 0.2;
        let overshoot_distance = distance * overshoot_factor;
        
        // Correction back
        let correction = distance * (overshoot_factor - 1.0);
        
        (overshoot_distance, correction)
    } else {
        (distance, 0.0)
    }
}

use rand;
```

---

## 3.6 Keyboard Utilities (`src/utils/keyboard.rs`)

### Reference: Node.js `api/interactions/actions.js` (type function)

The Node.js version provides:
- `type(selector, text, options)` - Character-by-character typing
- Persona-driven typo injection (adjacent key substitution)
- Auto-correction of typos based on persona correction rate
- Punctuation pauses, hesitation delays
- Gaussian inter-character timing

### Rust Implementation

```rust
use chromiumoxide::Page;
use anyhow::Result;
use tracing::debug;
use crate::utils::timing::{human_delay, random_pause, persona_pause};
use crate::utils::math::{random_in_range, gaussian, roll};

/// Typing options
#[derive(Debug, Clone)]
pub struct TypeOptions {
    /// Typo rate (0.0 to 1.0): probability of making a typo
    pub typo_rate: f64,
    /// Correction rate (0.0 to 1.0): probability of correcting a typo
    pub correction_rate: f64,
    /// Clear field before typing
    pub clear_first: bool,
    /// Base typing speed (ms per character)
    pub base_delay_ms: u64,
}

impl Default for TypeOptions {
    fn default() -> Self {
        Self {
            typo_rate: 0.05,        // 5% typo rate
            correction_rate: 0.7,   // 70% correction rate
            clear_first: false,
            base_delay_ms: 100,
        }
    }
}

/// Human-like typing into a field
/// Character-by-character with persona-driven typo injection and correction
/// Equivalent to Node.js: api.type(selector, text, options)
/// 
/// # Example
/// ```
/// human_type(page, "#search-input", "hello world", TypeOptions::default()).await?;
/// ```
pub async fn human_type(page: &Page, selector: &str, text: &str, options: TypeOptions) -> Result<()> {
    debug!("Typing into {}: \"{}\"", selector, text);
    
    // Focus the element first
    focus_input(page, selector).await?;
    
    // Clear if requested
    if options.clear_first {
        clear_input(page).await?;
        random_pause(100, 300).await;
    }
    
    // Type character by character
    for ch in text.chars() {
        // Typo injection
        if roll(options.typo_rate) {
            // Type wrong character (adjacent key)
            let wrong_char = get_adjacent_key(ch);
            dispatch_key_event(page, wrong_char).await?;
            random_pause(50, 200).await;
            
            // Maybe correct the typo
            if roll(options.correction_rate) {
                // Press backspace
                dispatch_backspace(page).await?;
                random_pause(80, 250).await;
                
                // Type correct character
                dispatch_key_event(page, ch).await?;
            }
        } else {
            dispatch_key_event(page, ch).await?;
        }
        
        // Inter-character delay with Gaussian distribution
        let std_dev = options.base_delay_ms as f64 * 0.3;
        let char_delay = gaussian(
            options.base_delay_ms as f64,
            std_dev,
            30.0,
            (options.base_delay_ms as f64 * 3.0),
        );
        
        // Punctuation pause
        let extra_delay = if ".!?,;:".contains(ch) {
            random_in_range(100, 300)
        } else {
            0
        };
        
        // Hesitation (rare, persona-driven)
        let hesitation_delay = if roll(0.02) {
            random_in_range(200, 800)
        } else {
            0
        };
        
        human_delay(char_delay as u64 + extra_delay + hesitation_delay).await;
    }
    
    debug!("Typing complete");
    Ok(())
}

/// Focus an input element
async fn focus_input(page: &Page, selector: &str) -> Result<()> {
    page.evaluate_expression(&format!(
        "document.querySelector('{}')?.focus()",
        selector
    )).await?;
    
    Ok(())
}

/// Clear input field (select all + delete)
async fn clear_input(page: &Page) -> Result<()> {
    // Select all (Ctrl+A or Cmd+A on Mac)
    page.evaluate_expression(
        "document.activeElement?.select()"
    ).await?;
    
    // Delete
    dispatch_backspace(page).await?;
    
    Ok(())
}

/// Dispatch a key event (type a character)
async fn dispatch_key_event(page: &Page, ch: char) -> Result<()> {
    page.evaluate_expression(&format!(
        "(() => {{
            const el = document.activeElement;
            if (!el) return;
            
            // Insert character
            const event = new KeyboardEvent('keydown', {{
                key: '{ch}',
                code: 'Key{ch}',
                bubbles: true
            }});
            el.dispatchEvent(event);
            
            // Update value
            if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {{
                el.value += '{ch}';
                el.dispatchEvent(new Event('input', {{ bubbles: true }}));
            }}
        }})()"
    )).await?;
    
    Ok(())
}

/// Dispatch backspace key
async fn dispatch_backspace(page: &Page) -> Result<()> {
    page.evaluate_expression(
        "(() => {
            const el = document.activeElement;
            if (!el) return;
            
            if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA') {
                el.value = el.value.slice(0, -1);
                el.dispatchEvent(new Event('input', { bubbles: true }));
            }
        })()"
    ).await?;
    
    Ok(())
}

/// QWERTY keyboard adjacency map for typo simulation
/// Equivalent to Node.js: ADJACENT_KEYS
const ADJACENT_KEYS: &[(char, &[char])] = &[
    ('a', &['s', 'q']),
    ('b', &['v', 'n']),
    ('c', &['x', 'v']),
    ('d', &['s', 'f']),
    ('e', &['w', 'r']),
    ('f', &['d', 'g']),
    ('g', &['f', 'h']),
    ('h', &['g', 'j']),
    ('i', &['u', 'o']),
    ('j', &['h', 'k']),
    ('k', &['j', 'l']),
    ('l', &['k', ';']),
    ('m', &['n', ',']),
    ('n', &['b', 'm']),
    ('o', &['i', 'p']),
    ('p', &['o', '[']),
    ('q', &['w', 'a']),
    ('r', &['e', 't']),
    ('s', &['a', 'd']),
    ('t', &['r', 'y']),
    ('u', &['y', 'i']),
    ('v', &['c', 'b']),
    ('w', &['q', 'e']),
    ('x', &['z', 'c']),
    ('y', &['t', 'u']),
    ('z', &['x', 's']),
];

/// Get a random adjacent key for typo simulation
fn get_adjacent_key(ch: char) -> char {
    let lower = ch.to_ascii_lowercase();
    
    for &(key, adjacents) in ADJACENT_KEYS {
        if key == lower {
            if let Some(&adj) = adjacents.first() {
                return if ch.is_uppercase() {
                    adj.to_ascii_uppercase()
                } else {
                    adj
                };
            }
        }
    }
    
    ch // No adjacent found, return original
}

/// Press a special key (Enter, Escape, etc.)
pub async fn press_key(page: &Page, key: &str) -> Result<()> {
    debug!("Pressing key: {}", key);
    
    page.evaluate_expression(&format!(
        "(() => {{
            const el = document.activeElement;
            if (!el) return;
            
            const event = new KeyboardEvent('keydown', {{
                key: '{key}',
                bubbles: true
            }});
            el.dispatchEvent(event);
        }})()"
    )).await?;
    
    Ok(())
}

/// Press Enter key
pub async fn press_enter(page: &Page) -> Result<()> {
    press_key(page, "Enter").await
}

/// Press Escape key
pub async fn press_escape(page: &Page) -> Result<()> {
    press_key(page, "Escape").await
}

/// Press Tab key
pub async fn press_tab(page: &Page) -> Result<()> {
    press_key(page, "Tab").await
}

/// Keyboard shortcut: Ctrl+A (or Cmd+A on Mac)
pub async fn select_all(page: &Page) -> Result<()> {
    page.evaluate_expression(
        "(() => {
            document.execCommand('selectAll');
        })()"
    ).await?;
    
    Ok(())
}

/// Keyboard shortcut: Ctrl+C (or Cmd+C on Mac)
pub async fn copy_text(page: &Page) -> Result<()> {
    page.evaluate_expression(
        "(() => {
            document.execCommand('copy');
        })()"
    ).await?;
    
    Ok(())
}
```

---

## 3.7 Update utils/mod.rs (Barrel File)

```rust
// Math utilities
pub mod math;
pub use math::{
    random_in_range,
    random_in_range_f64,
    gaussian,
    gaussian_int,
    roll,
    sample,
    sample_index,
    PidState,
    PidModel,
    pid_step,
};

// Timing utilities
pub mod timing;
pub use timing::{
    human_delay,
    thinking_pause,
    thinking_pause_ms,
    random_pause,
    persona_pause,
    post_action_pause,
    pre_action_hesitation,
    content_aware_pause,
};

// Navigation utilities
pub mod navigation;
pub use navigation::{
    goto,
    goto_and_wait,
    reload,
    go_back,
    go_forward,
    wait_for_selector,
    NavigationOptions,
};

// Scroll utilities
pub mod scroll;
pub use scroll::{
    human_scroll_read,
    scroll_down,
    scroll_up,
    scroll_to_top,
    scroll_to_bottom,
    focus_element,
    focus_element_absolute,
    ScrollOptions,
};

// Mouse utilities
pub mod mouse;
pub use mouse::{
    mouse_move_to,
    mouse_click,
    mouse_hover_with_drift,
    random_wheel_scroll,
    start_fidgeting,
    MousePosition,
};

// Keyboard utilities
pub mod keyboard;
pub use keyboard::{
    human_type,
    press_key,
    press_enter,
    press_escape,
    press_tab,
    select_all,
    copy_text,
    TypeOptions,
};
```

---

## Deliverables

- [ ] All utils compile without warnings
- [ ] `gaussian()` produces values within bounds
- [ ] `human_delay()` applies jitter correctly
- [ ] `human_scroll_read()` scrolls with pauses
- [ ] `mouse_move_to()` follows Bezier path
- [ ] `human_type()` includes typo injection
- [ ] `goto()` includes warmup behavior
- [ ] All utils accessible via `use crate::utils::*;`

---

## Testing Utils

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaussian_bounds() {
        for _ in 0..1000 {
            let val = gaussian(50.0, 10.0, 30.0, 70.0);
            assert!(val >= 30.0 && val <= 70.0);
        }
    }

    #[test]
    fn test_random_in_range() {
        for _ in 0..1000 {
            let val = random_in_range(10, 20);
            assert!(val >= 10 && val <= 20);
        }
    }

    #[test]
    fn test_roll_probability() {
        let mut successes = 0;
        for _ in 0..1000 {
            if roll(0.5) {
                successes += 1;
            }
        }
        // Should be ~500 (within 10% tolerance)
        assert!((successes as f64 - 500.0).abs() < 100.0);
    }

    #[test]
    fn test_sample_from_slice() {
        let arr = vec![1, 2, 3, 4, 5];
        for _ in 0..100 {
            let val = sample(&arr);
            assert!(val.is_some());
            assert!(val.unwrap() >= 1 && val.unwrap() <= 5);
        }
        
        assert!(sample::<i32>(&[]).is_none());
    }

    #[tokio::test]
    async fn test_human_delay() {
        let start = std::time::Instant::now();
        human_delay(100).await;
        let elapsed = start.elapsed().as_millis();
        // Should be ~100ms with 20% jitter (80-120ms)
        assert!(elapsed >= 70 && elapsed <= 130);
    }
}
```

---

## Notes

- **chromiumoxide CDP vs Playwright**: Some mouse/keyboard interactions in Playwright are handled natively via CDP. In chromiumoxide, you may need to:
  - Use `page.mouse().move(x, y)` if available
  - Or dispatch JavaScript events as shown
  - Test both approaches and choose the most reliable

- **Performance-aware impatience**: The Node.js version checks `performance.getEntriesByType("navigation")` to detect slow pages and reduces wait times. In Rust, this requires `page.evaluate()` to run JavaScript. Consider adding this as an optional optimization.

- **Ghost Cursor physics**: The Bezier path generation is a simplified version. For production, you may want to implement the full Fitts's Law + PID controller + overshoot correction from `ghostCursor.js`.

- **Persona system**: The Node.js version has a full persona system (casual, hurried, meticulous) that affects timing, typo rates, and scroll behavior. The Rust version uses defaults but can be extended with a `Persona` struct.
