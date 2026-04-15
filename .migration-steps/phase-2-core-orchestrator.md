# Phase 2: Core Orchestrator Engine

**Duration:** 2-3 days  
**Goal:** Implement the full orchestrator engine with CLI parsing, browser discovery, task queue, batching logic, and parallel execution across multiple browsers.

**Critical Success Criteria:** CLI must support exactly: `cargo run cookiebot pageview=www.reddit.com then cookiebot`

---

## 2.1 CLI Argument Parsing (`src/cli.rs`)

### Reference: Node.js `main.js` + `api/utils/task-parser.js`

The Node.js version uses two parsers:
1. **main.js inline parser**: Extracts `--browsers=` flag, treats all non-flag args as task names
2. **task-parser.js**: Splits tasks by `then` keyword into sequential groups

### CLI Behavior Specification

The Rust CLI must support **exactly** these patterns:

```bash
# Single task (no payload)
cargo run cookiebot

# Single task (with .js extension - both must work)
cargo run cookiebot.js

# Multiple tasks in one group (parallel execution)
cargo run cookiebot pageview

# Task with URL payload (auto-prepends https://)
cargo run pageview=www.reddit.com

# Task with explicit URL key
cargo run pageview=url=https://example.com

# Task with numeric payload
cargo run taskname=42

# Sequential groups (using 'then' separator)
cargo run cookiebot then pageview

# Complex example: multiple groups with mixed payloads
cargo run cookiebot pageview=www.reddit.com then cookiebot twitterFollow=x.com/user

# With browser filter option
cargo run -- --browsers=localChrome cookiebot pageview=reddit.com
```

### Parsing Rules

1. **`then` keyword**: Splits tasks into sequential groups (case-insensitive)
2. **Tasks without `=`**: Simple task name, empty payload
3. **Tasks with `=`**: First `=` separates task name from value
   - If value is numeric → payload: `{ "value": "42" }` (string-normalized parser output)
   - If value looks like URL (contains `.` or `localhost`) → payload: `{ "url": "https://value" }`
   - Otherwise → payload: `{ "url": "value" }`
4. **Multiple `=`**: `task=url=https://x.com` → task: `task`, payload: `{ "url": "https://x.com" }`
5. **`.js` extension**: Automatically stripped if present (`cookiebot.js` → `cookiebot`)
6. **Quoted values**: `task="value with spaces"` → quotes removed

### Rust Implementation

Implementation note: parser output in this phase is intentionally string-normalized (`HashMap<String, String>`) to mirror current Node.js compatibility behavior. Convert to typed JSON only at task boundaries if needed.

```rust
use clap::Parser;
use anyhow::{Result, bail};
use std::collections::HashMap;

#[derive(Parser, Debug)]
#[command(name = "rust-orchestrator")]
#[command(about = "Multi-browser automation orchestrator")]
pub struct Args {
    /// Tasks to run, separated by 'then' for sequential groups
    /// Examples:
    ///   cargo run cookiebot
    ///   cargo run cookiebot pageview=www.reddit.com
    ///   cargo run cookiebot then pageview
    ///   cargo run cookiebot.js pageview.js then cookiebot
    #[arg(required = false)]
    pub tasks: Vec<String>,

    /// Comma-separated list of browser types to connect to
    #[arg(long)]
    pub browsers: Option<String>,
}

pub fn parse_args() -> Args {
    Args::parse()
}

/// Represents a single task with its name and payload
#[derive(Debug, Clone)]
pub struct TaskDefinition {
    pub name: String,
    pub payload: HashMap<String, String>,
}

/// Parse CLI args into task groups for sequential execution
/// 
/// Mirrors the Node.js task-parser.js logic:
/// - `then` keyword splits tasks into sequential groups
/// - Tasks within the same group run in parallel
/// - Supports key=value syntax: `follow=x.com`, `retweet=y.com`
/// - Auto-detects numeric values vs URLs (auto-prepends `https://`)
/// - Supports quoted values: `task="value with spaces"`
///
/// # Example
/// ```
/// parse_task_groups(&["follow=x.com", "follow=y.com", "then", "retweet=z.com"])
/// // Returns:
/// // [
/// //   [TaskDefinition { name: "follow", payload: { url: "https://x.com" } },
/// //    TaskDefinition { name: "follow", payload: { url: "https://y.com" } }],
/// //   [TaskDefinition { name: "retweet", payload: { url: "https://z.com" } }]
/// // ]
/// ```
pub fn parse_task_groups(task_args: &[String]) -> Result<Vec<Vec<TaskDefinition>>> {
    let mut groups: Vec<Vec<TaskDefinition>> = Vec::new();
    let mut current_group: Vec<TaskDefinition> = Vec::new();
    let mut current_task: Option<String> = None;
    let mut current_payload: HashMap<String, String> = HashMap::new();

    if task_args.is_empty() {
        return Ok(vec![]);
    }

    for arg in task_args {
        let normalized = arg.to_lowercase();

        // Check for task separator
        if normalized == "then" {
            if let Some(task_name) = current_task.take() {
                current_group.push(TaskDefinition {
                    name: task_name,
                    payload: std::mem::take(&mut current_payload),
                });
            }
            if !current_group.is_empty() {
                groups.push(std::mem::take(&mut current_group));
            }
            continue;
        }

        // Parse key=value or standalone task name
        if let Some(eq_pos) = arg.find('=') {
            let key = &arg[..eq_pos];
            let mut value = &arg[eq_pos + 1..];

            // Handle quoted values
            if value.starts_with('"') && value.ends_with('"') {
                value = &value[1..value.len() - 1];
            }

            // Remove .js extension from task name if present
            let task_name = key.strip_suffix(".js").unwrap_or(key).to_string();

            if current_task.is_none() {
                // First key-value pair: this is the task name
                let formatted_value = if is_numeric(value) {
                    value.to_string()
                } else {
                    format_url(value)
                };
                
                current_task = Some(task_name);
                if is_numeric(value) {
                    current_payload.insert("value".to_string(), value.to_string());
                } else {
                    current_payload.insert("url".to_string(), formatted_value);
                }
            } else if key == current_task.as_ref().unwrap() {
                // Same task name with another value: push current and start new
                current_group.push(TaskDefinition {
                    name: current_task.take().unwrap(),
                    payload: std::mem::take(&mut current_payload),
                });

                let formatted_value = if is_numeric(value) {
                    value.to_string()
                } else {
                    format_url(value)
                };

                current_task = Some(task_name);
                if is_numeric(value) {
                    current_payload.insert("value".to_string(), value.to_string());
                } else {
                    current_payload.insert("url".to_string(), formatted_value);
                }
            } else {
                // Different key: add to payload
                let param_value = if key == "url" {
                    format_url(value)
                } else {
                    value.to_string()
                };
                current_payload.insert(key.to_string(), param_value);
            }
        } else {
            // Standalone argument (task name without =)
            if let Some(task_name) = current_task.take() {
                current_group.push(TaskDefinition {
                    name: task_name,
                    payload: std::mem::take(&mut current_payload),
                });
            }
            let task_name = arg.strip_suffix(".js").unwrap_or(arg);
            current_task = Some(task_name.to_string());
        }
    }

    // Push remaining task
    if let Some(task_name) = current_task.take() {
        current_group.push(TaskDefinition {
            name: task_name,
            payload: current_payload,
        });
    }

    if !current_group.is_empty() {
        groups.push(current_group);
    }

    Ok(groups)
}

fn is_numeric(value: &str) -> bool {
    value.chars().all(|c| c.is_ascii_digit()) && !value.is_empty()
}

fn format_url(value: &str) -> String {
    let trimmed = value.trim();
    
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    
    // Already has protocol
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }

    // Contains dot or is localhost → treat as URL, prepend https://
    let before_port = trimmed.split(':').next().unwrap_or(trimmed);
    if trimmed.contains('.') || before_port == "localhost" {
        return format!("https://{}", trimmed);
    }

    // Not a URL, return as-is
    trimmed.to_string()
}

/// Format parsed task groups for display logging
pub fn format_task_groups(groups: &[Vec<TaskDefinition>]) -> String {
    let total: usize = groups.iter().map(|g| g.len()).sum();
    
    if total == 0 {
        return "No tasks".to_string();
    }

    let parts: Vec<String> = groups
        .iter()
        .enumerate()
        .map(|(i, group)| {
            let names: Vec<&str> = group.iter().map(|t| t.name.as_str()).collect();
            if groups.len() > 1 {
                format!("Group {}: {}", i + 1, names.join(", "))
            } else {
                names.join(", ")
            }
        })
        .collect();

    format!("{} task(s) [{}]", total, parts.join(" | "))
}
```

---

## 2.2 Configuration Loader (`src/config.rs`)

### Reference: Node.js `config/settings.json`, `config/timeouts.json`, `config/browserAPI.json`

The Node.js version loads config from multiple JSON files with `configLoader.js` providing `getSettings()` and `getTimeoutValue()`.

### Rust Implementation

```rust
use anyhow::{Result, Context};
use serde::Deserialize;
use std::path::Path;
use std::collections::HashMap;
use once_cell::sync::Lazy;

static DEFAULT_CONFIG: Lazy<Config> = Lazy::new(|| Config {
    browser: BrowserConfig {
        connectors: vec![],
        connection_timeout_ms: 10000,
        max_discovery_retries: 3,
        discovery_retry_delay_ms: 5000,
        circuit_breaker: CircuitBreakerConfig {
            enabled: true,
            failure_threshold: 5,
            success_threshold: 3,
            half_open_time_ms: 30000,
        },
        profiles: vec![],
    },
    orchestrator: OrchestratorConfig {
        max_global_concurrency: 20,
        task_timeout_ms: 600000,
        group_timeout_ms: 600000,
        worker_wait_timeout_ms: 10000,
        stuck_worker_threshold_ms: 120000,
        task_stagger_delay_ms: 2000,
    },
    browser_concurrency: HashMap::new(),
});

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub browser: BrowserConfig,
    pub orchestrator: OrchestratorConfig,
    #[serde(default)]
    pub browser_concurrency: HashMap<String, usize>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BrowserConfig {
    #[serde(default)]
    pub connectors: Vec<String>,
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_ms: u64,
    #[serde(default = "default_discovery_retries")]
    pub max_discovery_retries: u32,
    #[serde(default = "default_discovery_delay")]
    pub discovery_retry_delay_ms: u64,
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerConfig,
    #[serde(default)]
    pub profiles: Vec<BrowserProfile>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CircuitBreakerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    #[serde(default = "default_success_threshold")]
    pub success_threshold: u32,
    #[serde(default = "default_half_open_time")]
    pub half_open_time_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BrowserProfile {
    pub name: String,
    #[serde(rename = "type")]
    pub profile_type: String,
    pub ws_endpoint: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OrchestratorConfig {
    #[serde(default = "default_max_concurrency")]
    pub max_global_concurrency: usize,
    #[serde(default = "default_task_timeout")]
    pub task_timeout_ms: u64,
    #[serde(default = "default_group_timeout")]
    pub group_timeout_ms: u64,
    #[serde(default = "default_worker_wait")]
    pub worker_wait_timeout_ms: u64,
    #[serde(default = "default_stuck_threshold")]
    pub stuck_worker_threshold_ms: u64,
    #[serde(default = "default_stagger_delay")]
    pub task_stagger_delay_ms: u64,
}

// Default value functions
fn default_connection_timeout() -> u64 { 10000 }
fn default_discovery_retries() -> u32 { 3 }
fn default_discovery_delay() -> u64 { 5000 }
fn default_true() -> bool { true }
fn default_failure_threshold() -> u32 { 5 }
fn default_success_threshold() -> u32 { 3 }
fn default_half_open_time() -> u64 { 30000 }
fn default_max_concurrency() -> usize { 20 }
fn default_task_timeout() -> u64 { 600000 }
fn default_group_timeout() -> u64 { 600000 }
fn default_worker_wait() -> u64 { 10000 }
fn default_stuck_threshold() -> u64 { 120000 }
fn default_stagger_delay() -> u64 { 2000 }

pub fn load_config() -> Result<Config> {
    let config_path = Path::new("config.toml");
    
    if config_path.exists() {
        let content = std::fs::read_to_string(config_path)
            .context("Failed to read config.toml")?;
        
        let config: Config = toml::from_str(&content)
            .context("Failed to parse config.toml")?;
        
        Ok(config)
    } else {
        tracing::warn!("config.toml not found, using defaults");
        Ok(DEFAULT_CONFIG.clone())
    }
}
```

---

## 2.3 Browser Connection (`src/browser.rs`)

### Reference: Node.js `api/core/automator.js` + `api/core/discovery.js`

The Node.js version:
- Uses `chromium.connectOverCDP(wsEndpoint)` from Playwright
- Has `BrowserCircuitBreaker` for connection failure throttling
- Native `disconnected` event listener triggers background reconnection (max 3 attempts)
- Dynamically loads connector modules from `api/connectors/discovery/`

### Rust Implementation

```rust
use crate::config::{Config, BrowserProfile};
use crate::session::Session;
use anyhow::{Result, Context, bail};
use chromiumoxide::{Browser, BrowserConfig};
use std::sync::atomic::{AtomicU32, Ordering};
use std::collections::HashMap;
use tokio::sync::Mutex;
use tracing::{info, warn, error};

/// Circuit breaker for browser connections
/// Mirrors the Node.js BrowserCircuitBreaker
pub struct CircuitBreaker {
    config: crate::config::CircuitBreakerConfig,
    failures: Mutex<HashMap<String, AtomicU32>>,
    successes: Mutex<HashMap<String, AtomicU32>>,
}

impl CircuitBreaker {
    pub fn new(config: crate::config::CircuitBreakerConfig) -> Self {
        Self {
            config,
            failures: Mutex::new(HashMap::new()),
            successes: Mutex::new(HashMap::new()),
        }
    }

    pub async fn check(&self, profile_id: &str) -> bool {
        if !self.config.enabled {
            return true;
        }

        let failures = self.failures.lock().await;
        let count = failures
            .get(profile_id)
            .map(|f| f.load(Ordering::SeqCst))
            .unwrap_or(0);

        if count >= self.config.failure_threshold {
            warn!(
                "Circuit breaker OPEN for {}, rejecting connection",
                profile_id
            );
            return false;
        }

        true
    }

    pub async fn record_success(&self, profile_id: &str) {
        let mut successes = self.successes.lock().await;
        let counter = successes
            .entry(profile_id.to_string())
            .or_insert_with(|| AtomicU32::new(0));
        counter.fetch_add(1, Ordering::SeqCst);

        // Reset failures after enough successes
        if counter.load(Ordering::SeqCst) >= self.config.success_threshold {
            let mut failures = self.failures.lock().await;
            failures.remove(profile_id);
        }
    }

    pub async fn record_failure(&self, profile_id: &str) {
        let mut failures = self.failures.lock().await;
        let counter = failures
            .entry(profile_id.to_string())
            .or_insert_with(|| AtomicU32::new(0));
        counter.fetch_add(1, Ordering::SeqCst);

        let mut successes = self.successes.lock().await;
        successes.remove(profile_id);
    }
}

/// Connect to a browser via its CDP endpoint
/// Equivalent to Playwright's chromium.connectOverCDP(wsEndpoint)
async fn connect_to_browser(
    profile: &BrowserProfile,
    config: &Config,
    circuit_breaker: &CircuitBreaker,
) -> Result<Session> {
    // Check circuit breaker
    if !circuit_breaker.check(&profile.name).await {
        bail!(
            "Circuit breaker open for profile {}",
            profile.name
        );
    }

    info!(
        "Connecting to browser: {} ({}) at {}",
        profile.name, profile.profile_type, profile.ws_endpoint
    );

    // Build browser configuration
    // chromiumoxide equivalent to Playwright.connectOverCDP
    let (browser, mut handler) = Browser::connect(
        &profile.ws_endpoint,
        BrowserConfig::builder()
            .connect_timeout(std::time::Duration::from_millis(
                config.browser.connection_timeout_ms,
            )),
    )
    .await
    .context(format!(
        "Failed to connect to browser: {}",
        profile.ws_endpoint
    ))?;

    // Spawn handler task (runs in background, processes CDP messages)
    let browser_clone = browser.clone();
    tokio::spawn(async move {
        while let Some(h) = handler.next().await {
            if let Err(e) = h {
                error!("CDP handler error: {:?}", e);
                break;
            }
        }
        // Browser disconnected
        warn!("Browser disconnected: {}", profile.name);
    });

    // Test connection
    if !browser.is_connected() {
        bail!("Browser connected but reports not connected");
    }

    circuit_breaker.record_success(&profile.name).await;

    // Determine max workers for this browser type
    let max_workers = config
        .browser_concurrency
        .get(&profile.profile_type)
        .copied()
        .unwrap_or(5); // Default 5 workers per browser

    Ok(Session::new(
        format!("{}-{}", profile.profile_type, profile.name),
        profile.name.clone(),
        profile.profile_type.clone(),
        browser,
        max_workers,
    ))
}

/// Discover and connect to browsers with retry logic
/// Mirrors the Node.js orchestrator.startDiscovery() with retry loop
pub async fn discover_browsers(config: &Config) -> Result<Vec<Session>> {
    let mut sessions = Vec::new();
    let circuit_breaker = CircuitBreaker::new(config.browser.circuit_breaker.clone());

    info!("Starting browser discovery...");

    for attempt in 1..=config.browser.max_discovery_retries {
        info!(
            "Discovery attempt {}/{}",
            attempt, config.browser.max_discovery_retries
        );

        for profile in &config.browser.profiles {
            match connect_to_browser(profile, config, &circuit_breaker).await {
                Ok(session) => {
                    info!("✓ Connected to browser: {} (type: {})", 
                          session.name, session.profile_type);
                    sessions.push(session);
                }
                Err(e) => {
                    circuit_breaker.record_failure(&profile.name).await;
                    error!("✗ Failed to connect to {}: {}", profile.name, e);
                }
            }
        }

        if !sessions.is_empty() {
            break;
        }

        if attempt < config.browser.max_discovery_retries {
            warn!(
                "No browsers found on attempt {}. Retrying in {}ms...",
                attempt, config.browser.discovery_retry_delay_ms
            );
            tokio::time::sleep(std::time::Duration::from_millis(
                config.browser.discovery_retry_delay_ms,
            ))
            .await;
        }
    }

    if sessions.is_empty() {
        warn!("No browsers discovered or connected after all retries");
    } else {
        info!(
            "Successfully connected to {} browser(s)",
            sessions.len()
        );
    }

    Ok(sessions)
}

/// Shutdown all browsers gracefully
pub async fn shutdown_sessions(sessions: Vec<Session>) {
    for session in sessions {
        if session.browser.is_connected() {
            match session.browser.close().await {
                Ok(_) => info!("Closed browser: {}", session.name),
                Err(e) => warn!("Error closing browser {}: {}", session.name, e),
            }
        }
    }
}
```

---

## 2.4 Session Management (`src/session.rs`)

### Reference: Node.js `api/core/sessionManager.js`

The Node.js version:
- Creates sessions with worker pools (default concurrency: 5 per browser)
- Uses `SimpleSemaphore` for worker acquisition with timeout
- Pages are acquired/released per-task via `acquirePage()` / `releasePage()`
- Stuck worker detection (threshold: 600s) with force-release

### Rust Implementation

```rust
use chromiumoxide::{Browser, Page};
use anyhow::{Result, bail};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub struct Session {
    pub id: String,
    pub name: String,
    pub profile_type: String,
    pub browser: Browser,
    
    // Worker semaphore (controls concurrent page access)
    // Equivalent to Node.js SimpleSemaphore
    worker_semaphore: Arc<Semaphore>,
    pub active_workers: std::sync::atomic::AtomicUsize,
    
    // Context for creating pages
    context: Arc<Mutex<Option<chromiumoxide::Context>>>,
}

impl Session {
    pub fn new(
        id: String,
        name: String,
        profile_type: String,
        browser: Browser,
        max_workers: usize,
    ) -> Self {
        Self {
            id,
            name,
            profile_type,
            browser: browser.clone(),
            worker_semaphore: Arc::new(Semaphore::new(max_workers)),
            active_workers: std::sync::atomic::AtomicUsize::new(0),
            context: Arc::new(Mutex::new(None)),
        }
    }

    /// Get or create the browser context
    async fn get_or_create_context(&self) -> Result<chromiumoxide::Context> {
        let mut ctx_guard = self.context.lock().await;
        
        if let Some(ctx) = ctx_guard.as_ref() {
            return Ok(ctx.clone());
        }

        // Create new context
        // Note: chromiumoxide API may differ - adjust accordingly
        let contexts = self.browser.contexts();
        if let Some(ctx) = contexts.first() {
            ctx_guard.replace(ctx.clone());
            Ok(ctx.clone())
        } else {
            // Create default context (adjust API as needed for chromiumoxide version)
            bail!("No browser contexts available");
        }
    }

    /// Acquire a worker permit from the semaphore
    /// Equivalent to Node.js sessionManager.acquireWorker()
    pub async fn acquire_worker(&self, timeout_ms: u64) -> Option<WorkerPermit> {
        use tokio::time::{timeout, Duration};

        match timeout(
            Duration::from_millis(timeout_ms),
            self.worker_semaphore.acquire(),
        )
        .await
        {
            Ok(Ok(permit)) => {
                self.active_workers.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Some(WorkerPermit {
                    _permit: permit,
                    session_id: self.id.clone(),
                    active_workers: &self.active_workers,
                })
            }
            Ok(Err(_)) => {
                warn!("[{}] Semaphore closed, cannot acquire worker", self.id);
                None
            }
            Err(_) => {
                warn!(
                    "[{}] Worker acquisition timeout after {}ms",
                    self.id, timeout_ms
                );
                None
            }
        }
    }

    /// Acquire a page for task execution
    /// Equivalent to Node.js sessionManager.acquirePage()
    pub async fn acquire_page(&self) -> Result<Arc<Page>> {
        let context = self.get_or_create_context().await?;
        
        // Create new page (equivalent to context.newPage())
        let page = context.new_page("").await?;
        
        Ok(Arc::new(page))
    }

    /// Release a page (close it)
    /// Equivalent to Node.js sessionManager.releasePage()
    pub async fn release_page(&self, page: Arc<Page>) {
        if page.is_closed() {
            return;
        }

        // Race condition prevention: timeout the close operation
        use tokio::time::{timeout, Duration};
        
        let page_clone = page.clone();
        if timeout(Duration::from_secs(5), async move {
            page_clone.close().await.ok();
        })
        .await
        .is_err() {
            warn!("[{}] Page close timeout exceeded", self.id);
        }
    }
}

/// Worker permit - released when dropped
pub struct WorkerPermit<'a> {
    _permit: tokio::sync::OwnedSemaphorePermit,
    session_id: String,
    active_workers: &'a std::sync::atomic::AtomicUsize,
}

impl<'a> Drop for WorkerPermit<'a> {
    fn drop(&mut self) {
        self.active_workers.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}
```

---

## 2.5 Orchestrator (`src/orchestrator.rs`)

### Reference: Node.js `api/core/orchestrator.js`

The Node.js version:
- `processTasks()` handles each group by clearing queue, spawning worker loops per session, `Promise.allSettled` waits for all sessions
- `processSharedChecklistForSession()` - worker loop that acquires page, executes task, releases page
- `executeTask()` - wraps task in timeout, creates abort signal, imports task module
- Global concurrency limit (`maxGlobalConcurrency: 20`)
- Task stagger delay to prevent network spikes

### Rust Implementation

```rust
use crate::config::Config;
use crate::session::Session;
use crate::cli::TaskDefinition;
use crate::task;
use anyhow::{Result, bail};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};
use tracing::{info, warn, error};

pub struct Orchestrator {
    config: Config,
    global_active_tasks: Arc<AtomicUsize>,
    global_semaphore: Arc<Semaphore>,
}

impl Orchestrator {
    pub fn new(config: Config) -> Self {
        Self {
            global_active_tasks: Arc::new(AtomicUsize::new(0)),
            global_semaphore: Arc::new(Semaphore::new(config.orchestrator.max_global_concurrency)),
            config,
        }
    }

    /// Execute a group of tasks across all sessions
    /// Tasks within a group run in parallel
    pub async fn execute_group(
        &mut self,
        group: &[TaskDefinition],
        sessions: &[Session],
    ) -> Result<()> {
        if sessions.is_empty() {
            bail!("No active sessions available");
        }

        if group.is_empty() {
            warn!("Empty task group, skipping");
            return Ok(());
        }

        let group_start = std::time::Instant::now();
        info!(
            "Executing group with {} task(s) across {} session(s)",
            group.len(),
            sessions.len()
        );

        // Apply group timeout
        let group_timeout = Duration::from_millis(self.config.orchestrator.group_timeout_ms);

        let task_futures: Vec<_> = group
            .iter()
            .cloned()
            .map(|task_def| {
                let sessions = sessions.to_vec();
                let global_active = self.global_active_tasks.clone();
                let global_sem = self.global_semaphore.clone();
                let config = self.config.clone();

                async move {
                    // Global concurrency throttling
                    let _permit = global_sem.acquire().await?;
                    global_active.fetch_add(1, Ordering::SeqCst);

                    // Stagger task starts to prevent network spikes
                    tokio::time::sleep(Duration::from_millis(
                        config.orchestrator.task_stagger_delay_ms
                    ))
                    .await;

                    // Find an available session and execute the task
                    let result = execute_task_on_session(
                        &task_def,
                        &sessions,
                        &config,
                    )
                    .await;

                    global_active.fetch_sub(1, Ordering::SeqCst);
                    result
                }
            })
            .collect();

        // Execute all tasks in parallel within the group
        let results = timeout(group_timeout, async {
            futures::future::join_all(task_futures).await
        })
        .await;

        match results {
            Ok(results) => {
                let success_count = results.iter().filter(|r| r.is_ok()).count();
                let fail_count = results.len() - success_count;

                info!(
                    "Group complete: {} succeeded, {} failed ({}s)",
                    success_count,
                    fail_count,
                    group_start.elapsed().as_secs_f64()
                );

                if fail_count > 0 {
                    warn!("{} task(s) failed in group", fail_count);
                }

                Ok(())
            }
            Err(_) => {
                bail!(
                    "Group timeout exceeded ({}ms)",
                    self.config.orchestrator.group_timeout_ms
                );
            }
        }
    }
}

/// Execute a single task on an available session
async fn execute_task_on_session(
    task_def: &TaskDefinition,
    sessions: &[Session],
    config: &Config,
) -> Result<()> {
    // Try each session until one succeeds
    for session in sessions {
        match execute_task_with_retry(task_def, session, config).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                warn!(
                    "Task {} failed on session {}: {}",
                    task_def.name, session.name, e
                );
                // Try next session
            }
        }
    }

    bail!("Task {} failed on all sessions", task_def.name);
}

/// Execute a task with retry logic
async fn execute_task_with_retry(
    task_def: &TaskDefinition,
    session: &Session,
    config: &Config,
) -> Result<()> {
    let max_retries = 2; // Mirrors Node.js maxTaskRetries
    let mut attempt = 0;

    loop {
        attempt += 1;

        // Acquire worker permit
        let permit = session
            .acquire_worker(config.orchestrator.worker_wait_timeout_ms)
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to acquire worker"))?;

        // Acquire page
        let page = session.acquire_page().await?;

        let result = async {
            // Execute the task
            let payload_json = serde_json::Value::Object(
                task_def
                    .payload
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                    .collect(),
            );

            task::perform_task(&page, &task_def.name, payload_json).await
        };

        // Apply task timeout
        let task_timeout = Duration::from_millis(config.orchestrator.task_timeout_ms);
        let execution_result = timeout(task_timeout, result).await;

        // Release page
        session.release_page(page).await;

        // Drop permit (releases worker)
        drop(permit);

        match execution_result {
            Ok(Ok(_)) => return Ok(()),
            Ok(Err(e)) => {
                if attempt <= max_retries {
                    warn!(
                        "Task {} attempt {}/{} failed: {}. Retrying...",
                        task_def.name, attempt, max_retries + 1, e
                    );
                    continue;
                } else {
                    bail!(
                        "Task {} failed after {} attempts: {}",
                        task_def.name,
                        attempt,
                        e
                    );
                }
            }
            Err(_) => {
                if attempt <= max_retries {
                    warn!(
                        "Task {} timed out on attempt {}/{}. Retrying...",
                        task_def.name, attempt, max_retries + 1
                    );
                    continue;
                } else {
                    bail!(
                        "Task {} timed out after {} attempts",
                        task_def.name,
                        attempt
                    );
                }
            }
        }
    }
}
```

---

## 2.6 Task Module Dispatcher (`task/mod.rs`)

### Reference: Node.js orchestrator's `_importTaskModule()` + task loading

```rust
use anyhow::{Result, bail};
use chromiumoxide::Page;
use serde_json::Value;

// Register all task modules
pub mod cookiebot;
pub mod pageview;
// Add more as you migrate tasks:
// pub mod twitter_follow;
// pub mod twitter_tweet;
// pub mod retweet;

/// Function pointer type for task functions
type TaskFn = fn(&Page, Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>;

/// Get a task function by name
/// Mirrors Node.js _importTaskModule() with module caching
pub fn get_task(name: &str) -> Option<TaskFn> {
    // Remove .js extension if present
    let clean_name = name.strip_suffix(".js").unwrap_or(name);
    
    match clean_name {
        "cookiebot" => Some(cookiebot::run),
        "pageview" => Some(pageview::run),
        // Add more mappings as you migrate tasks
        _ => None,
    }
}

/// Execute a task by name
/// This is the main entry point from the orchestrator
pub async fn perform_task(page: &Page, name: &str, payload: Value) -> Result<()> {
    let task_fn = get_task(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown task: {}. Add it to task/mod.rs", name))?;
    
    task_fn(page, payload).await
}
```

---

## 2.7 Update main.rs

Now wire everything together:

```rust
mod cli;
mod config;
mod browser;
mod orchestrator;
mod session;

#[path = "../task/mod.rs"]
mod task;

mod utils;

use anyhow::Result;
use tracing::{info, error, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rust_orchestrator=info".parse()?)
        )
        .init();

    info!("Rust Orchestrator - Starting up...");

    // Parse CLI
    let args = cli::parse_args();

    // Load configuration
    let config = config::load_config()?;

    // Discover and connect to browsers
    let sessions = match browser::discover_browsers(&config).await {
        Ok(sessions) => sessions,
        Err(e) => {
            warn!("Browser discovery failed: {}", e);
            vec![]
        }
    };

    if sessions.is_empty() {
        warn!("No browsers discovered or connected");
    } else {
        info!("✓ Connected to {} browser(s)", sessions.len());
    }

    let tasks = args.tasks;
    if tasks.is_empty() {
        info!("No tasks specified. System initialized in idle mode.");
        return Ok(());
    }

    // Parse tasks into groups (separated by "then")
    let groups = match cli::parse_task_groups(&tasks) {
        Ok(g) => g,
        Err(e) => {
            error!("Failed to parse task groups: {}", e);
            return Ok(());
        }
    };

    let total_tasks: usize = groups.iter().map(|g| g.len()).sum();
    info!(
        "Processing {} task(s) in {} group(s)",
        total_tasks,
        groups.len()
    );

    // Create orchestrator and run tasks
    let mut orchestrator = orchestrator::Orchestrator::new(config.clone());

    for (i, group) in groups.iter().enumerate() {
        info!("▶ Executing group {}/{}", i + 1, groups.len());
        
        match orchestrator.execute_group(group, &sessions).await {
            Ok(_) => info!("✓ Group {} complete", i + 1),
            Err(e) => {
                error!("✗ Group {} failed: {}", i + 1, e);
                // Continue with remaining groups or exit?
                // Node.js version continues, so we'll continue too
            }
        }
    }

    info!("All groups processed");

    // Cleanup: close browsers
    browser::shutdown_sessions(sessions).await;

    info!("Shutdown complete");
    Ok(())
}
```

---

## Deliverables

- [ ] `cargo run -- cookiebot.js pageview.js then cookiebot.js` executes correctly
- [ ] Tasks separated by `then` run sequentially
- [ ] Tasks within a group run in parallel across browsers
- [ ] Browser discovery retries 3 times with 5s delay
- [ ] Circuit breaker prevents connections to failing browsers
- [ ] Global concurrency limit enforced (default 20)
- [ ] Task timeout enforced (default 600s)
- [ ] Retry logic works (2 retries per task)
- [ ] Graceful shutdown closes all browsers

---

## Testing Checklist

```bash
# Single task
cargo run -- cookiebot.js

# Parallel tasks (same group)
cargo run -- cookiebot.js pageview.js

# Sequential groups
cargo run -- cookiebot.js then pageview.js

# Multiple tasks per group
cargo run -- cookiebot.js pageview.js then cookiebot.js pageview.js

# With browser filter
cargo run -- --browsers=localChrome cookiebot.js
```

---

## Notes

- **chromiumoxide API differences**: The chromiumoxide crate API may differ from Playwright's Node.js API. Key mappings:
  - `chromium.connectOverCDP(wsEndpoint)` → `Browser::connect(ws_endpoint, config)`
  - `browser.newContext()` → `browser.contexts()` or create default context
  - `context.newPage()` → `context.new_page(url)`
  - `page.goto(url, options)` → `page.navigate(url, params)`
  - `page.close()` → `page.close()`

- **Handler spawning**: chromiumoxide requires spawning a background handler task that processes CDP messages. This is done automatically in the `connect_to_browser` function.

- **RAII cleanup**: Rust's ownership system automatically handles cleanup. When `Session` is dropped, pages and browser connections are closed. No memory leaks possible.
