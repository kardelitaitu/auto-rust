# Phase 4: Task Migration Guide

**Duration:** 1 day per complex task  
**Goal:** Migrate each Node.js task to its own file in `task/`. Reuse utils heavily. Add new tasks without touching other files.

---

## 4.1 Task Migration Overview

### Node.js Task Structure

Every Node.js task file follows this contract:

```javascript
export default async function taskName(page, payload) {
    // payload.browserInfo = session ID
    // payload.abortSignal = AbortController.signal
    // payload.taskId = unique task identifier
    // Must return createSuccessResult() or createFailedResult()
}
```

### Rust Task Structure

Every Rust task file follows this contract:

```rust
use anyhow::Result;
use chromiumoxide::Page;
use serde_json::Value;
use tracing::info;

/// Task function - must be async, takes Page and payload JSON
pub async fn run(page: &Page, payload: Value) -> Result<()> {
    // Extract payload fields
    let browser_info = payload["browserInfo"].as_str().unwrap_or("unknown");
    let task_id = payload["taskId"].as_str().unwrap_or("unknown");
    
    // Implement task logic using utils
    // use crate::utils::*;
    
    Ok(())
}
```

---

## 4.2 Migration Steps Per Task

### Step 1: Create Task File

Create `task/<taskname>.rs`:

```bash
touch task/cookiebot.rs
touch task/pageview.rs
touch task/twitter_follow.rs
# ... etc
```

### Step 2: Register in `task/mod.rs`

```rust
// Add module
pub mod cookiebot;
pub mod pageview;
pub mod twitter_follow;

// Add to dispatcher
pub fn get_task(name: &str) -> Option<TaskFn> {
    let clean_name = name.strip_suffix(".js").unwrap_or(name);
    
    match clean_name {
        "cookiebot" => Some(cookiebot::run),
        "pageview" => Some(pageview::run),
        "twitter_follow" | "twitterFollow" => Some(twitter_follow::run),
        _ => None,
    }
}
```

### Step 3: Migrate Task Logic

Use the side-by-side mapping below to translate Node.js code to Rust.

---

## 4.3 Side-by-Side Migration Examples

### Example 1: Simple Navigation Task (pageview.js)

#### Node.js Original (`tasks/pageview.js`)

```javascript
export default async function pageview(page, payload) {
  const startTime = Date.now();
  const browserInfo = payload.browserInfo || "unknown_profile";
  const logger = createLogger("pageview.js");
  let targetUrl = null;

  try {
    return await api.withPage(
      page,
      async () => {
        // 1. Setup Profile & Persona
        let profile;
        try {
          profile = profileManager.getStarter();
          await api.init(page, {
            logger,
            persona: profile.persona || "casual",
            colorScheme: profile.theme || "dark",
          });
        } catch (e) {
          logger.warn(`Profile load failed: ${e.message}, using defaults`);
          await api.init(page, { logger, colorScheme: "dark" });
        }

        // 2. Determine target URL
        if (payload.url) {
          targetUrl = ensureProtocol(payload.url);
        } else {
          targetUrl = await getRandomUrl();
        }
        logger.info(`Target: ${targetUrl}`);

        // 3. Navigation
        const taskTimeoutMs = 50000;

        await Promise.race([
          (async () => {
            await api.goto(targetUrl, {
              waitUntil: "domcontentloaded",
              timeout: 20000,
              warmup: true,
              warmupMouse: true,
              warmupPause: true,
              referer: ctx.referrer || undefined,
            });

            await api.wait(api.randomInRange(1000, 2000));

            // 4. Reading Simulation
            const readingMs = api.gaussian(30000, 10000, 10000, 45000);
            const pauses = Math.max(1, Math.floor(readingS / 2.2));

            await api.scroll.read(null, {
              pauses,
              scrollAmount: api.randomInRange(600, 1200),
              variableSpeed: true,
              backScroll: true,
            });

            await api.wait(api.randomInRange(1000, 2000));
          })(),
          new Promise((_, reject) =>
            setTimeout(() => reject(new Error("Pageview timeout")), taskTimeoutMs)
          ),
        ]);

        return createSuccessResult('pageview', {
          url: targetUrl,
          referrer: ctx.referrer
        }, { startTime, sessionId: browserInfo });
      },
      { taskName: "pageview", sessionId: browserInfo }
    );
  } catch (error) {
    logger.error(`Pageview error: ${error.message}`);
    return createFailedResult('pageview', error, {
      partialData: { url: targetUrl },
      sessionId: browserInfo
    });
  }
}
```

#### Rust Migration (`task/pageview.rs`)

```rust
use anyhow::{Result, Context};
use chromiumoxide::Page;
use serde_json::{json, Value};
use tracing::{info, warn, error};
use crate::utils::*;
use std::fs;

const URL_FILE: &str = "data/pageview.txt";

/// Load URLs from text file
fn load_urls() -> Result<Vec<String>> {
    let content = fs::read_to_string(URL_FILE)
        .context("Failed to read URL file")?;
    
    Ok(content
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect())
}

/// Get random URL from file
fn get_random_url() -> Result<String> {
    let urls = load_urls()?;
    if urls.is_empty() {
        anyhow::bail!("No URLs found in {}", URL_FILE);
    }
    
    Ok(sample(&urls).unwrap())
}

/// Ensure URL has protocol
fn ensure_protocol(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{}", url)
    }
}

/// Main pageview task
/// Equivalent to Node.js: tasks/pageview.js
pub async fn run(page: &Page, payload: Value) -> Result<()> {
    let start_time = std::time::Instant::now();
    let browser_info = payload["browserInfo"]
        .as_str()
        .unwrap_or("unknown_profile");
    
    info!("Starting pageview task (session: {})", browser_info);

    // Determine target URL
    let target_url = if let Some(url) = payload["url"].as_str() {
        ensure_protocol(url)
    } else {
        get_random_url()?
    };
    
    info!("Target: {}", target_url);

    // Navigation with timeout
    let timeout_duration = std::time::Duration::from_secs(50);
    
    tokio::time::timeout(timeout_duration, async {
        // Navigate with warmup
        let nav_options = navigation::NavigationOptions {
            wait_until: "domcontentloaded".to_string(),
            timeout_ms: 20000,
            warmup: true,
            warmup_mouse: true,
            warmup_pause: true,
            ..Default::default()
        };
        
        navigation::goto(page, &target_url, nav_options).await?;
        
        // Post-navigation pause
        random_pause(1000, 2000).await;

        // Reading simulation
        let reading_ms = gaussian(30000.0, 10000.0, 10000.0, 45000.0);
        let reading_s = reading_ms / 1000.0;
        let pauses = (reading_s / 2.2).max(1.0) as u32;

        info!("Scrolling with {} pauses (reading ~{:.0}s)", pauses, reading_s);

        scroll::human_scroll_read(page, scroll::ScrollOptions {
            pauses,
            scroll_amount: random_in_range(600, 1200),
            variable_speed: true,
            back_scroll: true,
        }).await?;

        // Post-scroll pause
        random_pause(1000, 2000).await;

        Ok::<(), anyhow::Error>(())
    })
    .await
    .map_err(|_| anyhow::anyhow!("Pageview timeout after 50s"))??;

    let duration = start_time.elapsed();
    info!("Pageview task completed in {:.2}s", duration.as_secs_f64());
    
    Ok(())
}
```

---

### Example 2: Loop Navigation Task (cookiebot.js)

#### Node.js Original (`tasks/cookiebot.js`)

```javascript
export default async function cookieBotRandom(page, payload) {
  const startTime = process.hrtime.bigint();
  const browserInfo = payload.browserInfo || "unknown_profile";
  const logger = createLogger("cookiebot.js");

  try {
    await Promise.race([
      api.withPage(page, async () => {
        await api.init(page, {
          logger,
          lite: true,
          blockNotifications: true,
          blockDialogs: true,
          autoBanners: false,
          muteAudio: true,
        });

        // Block Video Load
        await page.route("**/*", (route) => {
          const type = request.resourceType();
          if (type === "media" || /video|googlevideo|youtube/i.test(url)) {
            return route.abort().catch(() => {});
          }
          return route.fallback().catch(() => {});
        });

        const loopCount = api.randomInRange(CONFIG.loopCountMin, CONFIG.loopCountMax);
        const abortSignal = payload.abortSignal;

        for (let i = 0; i < loopCount; i++) {
          if (page.isClosed() || abortSignal?.aborted) break;

          const randomUrl = urls[Math.floor(Math.random() * urls.length)];
          
          try {
            // 1. Navigate
            await api.goto(randomUrl, {
              waitUntil: "domcontentloaded",
              timeout: CONFIG.navigationTimeout,
            });

            // 2. Check responsiveness
            await api.waitFor(
              async () => await api.eval(() => true).catch(() => false),
              { timeout: CONFIG.responsivenessTimeout },
            );
            await api.wait(CONFIG.postLoadDelay);

            // 3. Scroll/Read
            await api.scroll.read(null, {
              pauses: api.randomInRange(CONFIG.scrollPausesMin, CONFIG.scrollPausesMax),
              scrollAmount: api.randomInRange(CONFIG.scrollAmountMin, CONFIG.scrollAmountMax),
            });
            await api.wait(CONFIG.postScrollDelay);
            
            // 4. Simulate reading
            const readTime = api.randomInRange(CONFIG.minReadSecond * 10, CONFIG.maxReadSecond * 10) / 10;
            await api.wait(parseFloat(readTime) * 1000);
          } catch (navError) {
            if (navError.message.includes("interrupted") || navError.message.includes("closed")) {
              break;
            } else if (navError.message.includes("timeout")) {
              logger.warn(`Visit to ${randomUrl} timed out. Skipping.`);
            } else {
              logger.error(`Failed to load ${randomUrl}: ${navError.message}`);
              if (page.isClosed()) break;
            }
          }
        }
      }),
      new Promise((_, reject) =>
        setTimeout(() => reject(new Error(`Cookiebot task exceeded ${CONFIG.taskTimeoutMs}ms limit`)), CONFIG.taskTimeoutMs)
      ),
    ]);
  } catch (error) {
    logger.error(`CRITICAL ERROR in main task loop:`, error);
  } finally {
    try {
      if (page && !page.isClosed()) {
        await Promise.race([page.close(), new Promise((r) => setTimeout(r, 5000))]);
      }
    } catch (closeError) {
      logger.warn(`Error closing page: ${closeError.message}`);
    }
  }
}
```

#### Rust Migration (`task/cookiebot.rs`)

```rust
use anyhow::{Result, Context};
use chromiumoxide::Page;
use serde_json::Value;
use tracing::{info, warn, error};
use crate::utils::*;
use std::fs;

const CONFIG: CookiebotConfig = CookiebotConfig {
    sites_file: "data/cookiebot.txt",
    task_timeout_ms: 240000,
    navigation_timeout: 60000,
    responsiveness_timeout: 15000,
    loop_count_min: 5,
    loop_count_max: 10,
    min_read_second: 3,
    max_read_second: 6,
    scroll_pauses_min: 4,
    scroll_pauses_max: 8,
    scroll_amount_min: 300,
    scroll_amount_max: 600,
    post_load_delay: 1000,
    post_scroll_delay: 1000,
};

struct CookiebotConfig {
    sites_file: &'static str,
    task_timeout_ms: u64,
    navigation_timeout: u64,
    responsiveness_timeout: u64,
    loop_count_min: u32,
    loop_count_max: u32,
    min_read_second: u32,
    max_read_second: u32,
    scroll_pauses_min: u32,
    scroll_pauses_max: u32,
    scroll_amount_min: u64,
    scroll_amount_max: u64,
    post_load_delay: u64,
    post_scroll_delay: u64,
}

/// Load URL list from file
fn load_url_list() -> Result<Vec<String>> {
    let data = fs::read_to_string(CONFIG.sites_file)
        .context("Failed to read sites file")?;
    
    Ok(data
        .lines()
        .map(|line| {
            let mut url = line.trim().to_string();
            // Fix mangled protocols
            if url.starts_with("http_") {
                url = url.replace("http_", "http:");
            }
            if url.starts_with("https_") {
                url = url.replace("https_", "https:");
            }
            url
        })
        .filter(|url| url.starts_with("http"))
        .collect())
}

/// Main cookiebot task
/// Equivalent to Node.js: tasks/cookiebot.js
pub async fn run(page: &Page, payload: Value) -> Result<()> {
    let start_time = std::time::Instant::now();
    let browser_info = payload["browserInfo"]
        .as_str()
        .unwrap_or("unknown_profile");
    
    info!("Starting cookiebot task (session: {})", browser_info);

    // Load URL list
    let urls = load_url_list()?;
    info!("URL list size: {}", urls.len());
    
    if urls.is_empty() {
        anyhow::bail!("URL list is empty. Aborting task.");
    }

    let loop_count = random_in_range(
        CONFIG.loop_count_min as u64,
        CONFIG.loop_count_max as u64,
    ) as u32;
    
    info!("Starting random visits loop for {} times", loop_count);

    // Get abort signal from payload (if provided)
    let aborted = payload.get("aborted")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Main loop
    for i in 0..loop_count {
        // Check if page is closed or task aborted
        if page.is_closed() || aborted {
            info!("Page closed or task aborted, stopping loop");
            break;
        }

        let random_url = sample(&urls).unwrap();
        info!("({}/{}): {}", i + 1, loop_count, random_url);

        // Navigate to URL with timeout
        let nav_result = tokio::time::timeout(
            std::time::Duration::from_millis(CONFIG.navigation_timeout),
            async {
                // Navigate
                let nav_options = navigation::NavigationOptions {
                    wait_until: "domcontentloaded".to_string(),
                    timeout_ms: CONFIG.navigation_timeout,
                    warmup: false, // Lite mode
                    auto_banners: false,
                    ..Default::default()
                };
                
                navigation::goto(page, &random_url, nav_options).await?;

                // Check responsiveness
                tokio::time::timeout(
                    std::time::Duration::from_millis(CONFIG.responsiveness_timeout),
                    async {
                        page.evaluate_expression("(() => true)").await
                    }
                )
                .await
                .map_err(|_| anyhow::anyhow!("Responsiveness check timeout"))??;

                human_delay(CONFIG.post_load_delay).await;

                // Scroll/Read
                scroll::human_scroll_read(page, scroll::ScrollOptions {
                    pauses: random_in_range(
                        CONFIG.scroll_pauses_min as u64,
                        CONFIG.scroll_pauses_max as u64,
                    ) as u32,
                    scroll_amount: random_in_range(
                        CONFIG.scroll_amount_min,
                        CONFIG.scroll_amount_max,
                    ),
                    variable_speed: false,
                    back_scroll: false,
                }).await?;

                human_delay(CONFIG.post_scroll_delay).await;

                // Simulate reading
                let read_time = random_in_range(
                    (CONFIG.min_read_second * 10) as u64,
                    (CONFIG.max_read_second * 10) as u64,
                ) as f64 / 10.0;
                
                info!("Simulating reading for {:.1}s", read_time);
                human_delay((read_time * 1000.0) as u64).await;

                Ok::<(), anyhow::Error>(())
            }
        )
        .await;

        // Handle navigation result
        match nav_result {
            Ok(Ok(_)) => {
                // Success, continue to next URL
            }
            Ok(Err(e)) => {
                let error_msg = e.to_string();
                
                if error_msg.contains("interrupted") || error_msg.contains("closed") {
                    warn!("Navigation interrupted, stopping loop");
                    break;
                } else if error_msg.contains("timeout") || error_msg.contains("Timeout") {
                    warn!("Visit to {} timed out, skipping", random_url);
                } else if error_msg.contains("ERR_") {
                    warn!("Network error visiting {}", random_url);
                } else {
                    error!("Failed to load {}: {}", random_url, error_msg);
                    
                    if page.is_closed() {
                        break;
                    }
                }
            }
            Err(_) => {
                warn!("Navigation to {} timed out, skipping", random_url);
            }
        }
    }

    let duration = start_time.elapsed();
    info!("Total task duration: {:.2}s", duration.as_secs_f64());
    
    Ok(())
}
```

---

## 4.4 Common Migration Patterns

### Node.js → Rust Mapping

| Node.js Concept | Rust Equivalent |
|----------------|----------------|
| `Date.now()` | `std::time::Instant::now()` |
| `setTimeout(fn, ms)` | `tokio::time::sleep(Duration::from_millis(ms))` |
| `Promise.race([p1, p2])` | `tokio::time::timeout(duration, future)` |
| `Promise.allSettled([...])` | `futures::future::join_all([...])` |
| `try/catch/finally` | `match result { Ok(_) => ..., Err(e) => ... }` |
| `process.hrtime.bigint()` | `std::time::Instant::now()` |
| `fs.readFile()` | `std::fs::read_to_string()` |
| `Math.random()` | `rand::thread_rng().gen_range()` |
| `api.randomInRange(min, max)` | `random_in_range(min, max)` |
| `api.gaussian(mean, dev, min, max)` | `gaussian(mean, dev, min, max)` |
| `api.goto(url, options)` | `navigation::goto(page, url, options)` |
| `api.wait(ms)` | `human_delay(ms).await` |
| `api.scroll.read(null, options)` | `scroll::human_scroll_read(page, options)` |
| `page.isClosed()` | `page.is_closed()` |
| `payload.abortSignal.aborted` | `payload.get("aborted").and_then(...)` |
| `createSuccessResult()` | `Ok(())` |
| `createFailedResult()` | `Err(anyhow::anyhow!(...))` |
| `logger.info()` | `info!()` (from tracing) |
| `logger.warn()` | `warn!()` (from tracing) |
| `logger.error()` | `error!()` (from tracing) |

---

## 4.5 Task Payload Handling

### Node.js Pattern

```javascript
const browserInfo = payload.browserInfo || "unknown_profile";
const url = payload.url;
const value = payload.value;
```

### Rust Pattern

```rust
let browser_info = payload["browserInfo"]
    .as_str()
    .unwrap_or("unknown_profile");

let url = payload["url"].as_str();
let value = payload["value"].as_u64();
```

### Complex Payload Extraction

```rust
// Extract optional fields
let timeout = payload["timeoutMs"]
    .as_u64()
    .unwrap_or(60000);

// Extract nested objects
if let Some(options) = payload["options"].as_object() {
    if let Some(retries) = options["maxRetries"].as_u64() {
        // Use retries
    }
}

// Extract array
if let Some(urls) = payload["urlList"].as_array() {
    let url_strings: Vec<String> = urls
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
}
```

---

## 4.6 Error Handling Patterns

### Node.js Pattern

```javascript
try {
    await api.goto(url, { timeout: 20000 });
} catch (navError) {
    if (navError.message.includes("timeout")) {
        logger.warn("Timed out");
    } else {
        logger.error("Failed:", navError.message);
    }
}
```

### Rust Pattern

```rust
match navigation::goto(page, url, options).await {
    Ok(_) => {
        // Success
    }
    Err(e) => {
        let error_msg = e.to_string();
        
        if error_msg.contains("timeout") {
            warn!("Timed out");
        } else {
            error!("Failed: {}", error_msg);
        }
        
        // Optionally re-throw
        return Err(e);
    }
}
```

### Timeout Wrapping

```rust
// Wrap any async operation with timeout
let result = tokio::time::timeout(
    std::time::Duration::from_millis(30000),
    async {
        // Your async code here
        navigation::goto(page, url, options).await
    }
)
.await;

match result {
    Ok(Ok(value)) => { /* Success */ }
    Ok(Err(e)) => { /* Operation failed */ }
    Err(_) => { /* Timeout */ }
}
```

---

## 4.7 File I/O Patterns

### Reading URL Lists

#### Node.js
```javascript
const content = await fs.readFile(URL_FILE, "utf-8");
const urls = content.split("\n").map(line => line.trim()).filter(line => line);
```

#### Rust
```rust
let content = fs::read_to_string(URL_FILE)?;
let urls: Vec<String> = content
    .lines()
    .map(|line| line.trim().to_string())
    .filter(|line| !line.is_empty())
    .collect();
```

### Reading JSON Config

#### Node.js
```javascript
const config = JSON.parse(await fs.readFile("config.json", "utf-8"));
```

#### Rust
```rust
let content = fs::read_to_string("config.json")?;
let config: MyConfig = serde_json::from_str(&content)?;
```

---

## 4.8 Adding a New Task (Template)

### Step 1: Create `task/my_new_task.rs`

```rust
use anyhow::Result;
use chromiumoxide::Page;
use serde_json::Value;
use tracing::{info, warn, error};
use crate::utils::*;

pub async fn run(page: &Page, payload: Value) -> Result<()> {
    let start_time = std::time::Instant::now();
    let browser_info = payload["browserInfo"]
        .as_str()
        .unwrap_or("unknown_profile");
    
    info!("Starting my_new_task (session: {})", browser_info);

    // Extract payload
    let target_url = payload["url"]
        .as_str()
        .unwrap_or("https://example.com");
    
    info!("Target: {}", target_url);

    // Implement task logic
    let options = navigation::NavigationOptions::default();
    navigation::goto(page, target_url, options).await?;
    
    random_pause(1000, 2000).await;
    
    scroll::human_scroll_read(page, scroll::ScrollOptions::default()).await?;

    let duration = start_time.elapsed();
    info!("Task completed in {:.2}s", duration.as_secs_f64());
    
    Ok(())
}
```

### Step 2: Register in `task/mod.rs`

```rust
pub mod my_new_task;

pub fn get_task(name: &str) -> Option<TaskFn> {
    match name {
        // ... existing tasks
        "my_new_task" => Some(my_new_task::run),
        _ => None,
    }
}
```

### Step 3: Test

```bash
cargo run -- my_new_task.js
cargo run -- my_new_task=url=https://example.com
```

---

## 4.9 Task Migration Checklist

For each task:

- [ ] Create `task/<taskname>.rs` file
- [ ] Add module declaration in `task/mod.rs`
- [ ] Add mapping in `get_task()` function
- [ ] Migrate task logic using utils
- [ ] Handle payload extraction correctly
- [ ] Add timeout wrapping for async operations
- [ ] Implement error handling patterns
- [ ] Add logging (info!, warn!, error!)
- [ ] Test with `cargo run -- <taskname>.js`
- [ ] Verify behavior matches Node.js version

---

## 4.10 Tasks to Migrate (Priority Order)

Based on the Node.js reference codebase:

| Priority | Task File | Complexity | Notes |
|----------|-----------|------------|-------|
| 1 | `pageview.js` | Low | Simple navigation + scroll |
| 2 | `cookiebot.js` | Low | Loop navigation, already exemplified |
| 3 | `twitterFollow.js` | Medium | Requires Twitter API interaction |
| 4 | `twitterTweet.js` | Medium | Post tweet, form filling |
| 5 | `retweet.js` | Medium | Click-based interaction |
| 6 | `followback.js` | Low | Similar to twitterFollow |
| 7 | `twitterscroll.js` | Low | Scroll behavior on Twitter |
| 8 | `twitterFollowLikeRetweet.js` | High | Combined actions |
| 9 | `api-twitterActivity.js` | High | AI-driven activity |
| 10 | `owb.js` | High | Game agent wrapper |
| 11 | `agent.js` | High | LLM agent runner |

---

## Deliverables

- [ ] All priority 1-2 tasks migrated and tested
- [ ] All tasks compile without warnings
- [ ] Behavior matches Node.js version
- [ ] New tasks can be added in < 30 minutes
- [ ] Utils heavily reused (no duplicate code)

---

## Notes

- **Keep tasks modular**: Each task is one file. No shared state between tasks.
- **Use utils extensively**: Don't reimplement scrolling, typing, timing in tasks
- **Error handling**: Always wrap navigation in timeout + match pattern
- **Logging**: Use `info!()`, `warn!()`, `error!()` from tracing crate
- **Payload flexibility**: `serde_json::Value` allows any JSON structure
- **RAII cleanup**: Page closure is automatic when dropped. No manual cleanup needed unless explicit timeout required
