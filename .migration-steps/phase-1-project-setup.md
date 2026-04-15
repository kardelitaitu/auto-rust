# Phase 1: Project Setup & Skeleton

**Duration:** 1 day  
**Goal:** Create the Rust project structure, add dependencies, and wire up the basic skeleton so it compiles and runs.

---

## 1.1 Create Rust Project

```bash
cargo init rust-orchestrator --name rust-orchestrator
cd rust-orchestrator
```

---

## 1.2 Create Folder Structure

Create the following directory structure exactly as specified:

```
rust-orchestrator/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── .gitignore
├── config.toml                 # Live browser URLs (add to .gitignore)
├── config.toml.example         # Safe template for git
├── task/                       # ← TASK FOLDER AT ROOT
│   ├── mod.rs                  # Task registry + perform_task dispatcher
│   ├── cookiebot.rs            # cookiebot task logic
│   └── pageview.rs             # pageview task logic
├── data/                       # All .txt files (URL lists, etc.)
│   ├── cookiebot.txt           # Copy from nodejs reference codebase/config/popularsite.txt
│   ├── pageview.txt            # Copy from nodejs reference codebase/tasks/pageview.txt
│   └── referrer.txt            # Copy from nodejs reference codebase/data/URLreferrer.txt
├── src/
│   ├── main.rs                 # Thin entry point
│   ├── cli.rs                  # Clap + batch parsing ("then")
│   ├── config.rs               # config.toml loader + auto-scan
│   ├── browser.rs              # chromiumoxide connection logic
│   ├── orchestrator.rs         # Task queue + batch execution
│   ├── session.rs              # Session/worker management
│   ├── utils/                  # ← UTILS FOLDER INSIDE SRC
│   │   ├── mod.rs              # ← Barrel file for easy imports (index.rs equivalent)
│   │   ├── navigation.rs       # goto, wait_for, etc.
│   │   ├── scroll.rs           # random_scroll, human_scroll
│   │   ├── mouse.rs            # human_mouse_move, click
│   │   ├── keyboard.rs         # natural_typing, press_key
│   │   ├── timing.rs           # random_delay, human_pause
│   │   └── math.rs             # gaussian, random_in_range
│   └── tests/                  # ← TESTS FOLDER INSIDE SRC
│       ├── mod.rs
│       └── integration_test.rs
└── target/                     # Auto-generated (gitignored)
```

---

## 1.3 Cargo.toml Dependencies

```toml
[package]
name = "rust-orchestrator"
version = "0.1.0"
edition = "2021"

[dependencies]
# CDP Browser Automation
chromiumoxide = { version = "0.6", features = ["tokio-runtime"] }
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"

# CLI Parsing
clap = { version = "4", features = ["derive"] }

# Configuration
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# HTTP & Networking
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
url = "2"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "json"] }

# Error Handling
anyhow = "1"
thiserror = "1"

# Async Utilities
futures = "0.3"
async-trait = "0.1"

# Random & Probability
rand = "0.8"
rand_distr = "0.4"

# File & Path Utilities
tokio-stream = "0.1"
glob = "0.3"

# Time
chrono = { version = "0.4", features = ["serde"] }
humantime = "2"

# Concurrency
parking_lot = "0.12"
dashmap = "5"

# Regex
regex = "1"

# Lazy Static / Once Cell
once_cell = "1"
```

---

## 1.4 .gitignore

```gitignore
# Rust
/target
# Keep Cargo.lock committed for reproducible binary builds

# Config (contains live browser URLs)
config.toml

# IDE
.idea/
.vscode/
*.swp
*.swo

# OS
.DS_Store
Thumbs.db

# Logs
*.log
logs/
```

---

## 1.5 config.toml.example

```toml
# Browser Configuration Template
# Copy to config.toml and fill in your browser endpoints

[browser]
# Auto-discovery connectors to use (leave empty to scan all)
connectors = ["localChrome", "ixbrowser", "roxybrowser"]

# Connection timeout in milliseconds
connection_timeout_ms = 10000

# Max retries for browser discovery
max_discovery_retries = 3
discovery_retry_delay_ms = 5000

# Circuit breaker settings
[browser.circuit_breaker]
enabled = true
failure_threshold = 5
success_threshold = 3
half_open_time_ms = 30000

# Browser profiles
[[browser.profiles]]
name = "chrome-1"
type = "localChrome"
ws_endpoint = "ws://localhost:9222"

[[browser.profiles]]
name = "ixbrowser-profile-1"
type = "ixbrowser"
ws_endpoint = "ws://localhost:9223"

# Orchestrator settings
[orchestrator]
# Max concurrent tasks across all browsers
max_global_concurrency = 20

# Task timeout in milliseconds
task_timeout_ms = 600000

# Group timeout in milliseconds
group_timeout_ms = 600000

# Worker acquisition timeout in milliseconds
worker_wait_timeout_ms = 10000

# Stuck worker threshold in milliseconds
stuck_worker_threshold_ms = 120000

# Delay between task starts to prevent network spikes
task_stagger_delay_ms = 2000
```

---

## 1.6 Copy Data Files

Copy the following files from the Node.js reference codebase:

| Destination | Source |
|------------|--------|
| `data/cookiebot.txt` | `nodejs reference codebase/config/popularsite.txt` |
| `data/pageview.txt` | `nodejs reference codebase/tasks/pageview.txt` |
| `data/referrer.txt` | `nodejs reference codebase/data/URLreferrer.txt` |

---

## 1.7 Create Skeleton Source Files

### `src/main.rs`

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
use tracing::{info, error};

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
    let sessions = browser::discover_browsers(&config).await?;
    info!("Connected to {} browser(s)", sessions.len());

    if args.tasks.is_empty() {
        info!("No tasks specified. System initialized in idle mode.");
        return Ok(());
    }

    // Parse tasks into groups (separated by "then")
    let groups = cli::parse_task_groups(&args.tasks)?;
    info!(
        "Processing {} task(s) in {} group(s)",
        groups.iter().map(|g| g.len()).sum::<usize>(),
        groups.len()
    );

    // Create orchestrator and run tasks
    let mut orchestrator = orchestrator::Orchestrator::new(config);
    
    for (i, group) in groups.iter().enumerate() {
        info!("Executing group {}/{}", i + 1, groups.len());
        orchestrator.execute_group(group, &sessions).await?;
        info!("Group {} complete", i + 1);
    }

    info!("All tasks completed successfully");
    Ok(())
}
```

### `src/cli.rs`

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rust-orchestrator")]
#[command(about = "Multi-browser automation orchestrator")]
pub struct Args {
    /// Tasks to run, separated by 'then' for sequential groups
    /// Example: cookiebot.js pageview.js then twitterFollow.js
    pub tasks: Vec<String>,
}

pub fn parse_args() -> Args {
    Args::parse()
}

pub fn parse_task_groups(task_args: &[String]) -> anyhow::Result<Vec<Vec<TaskDefinition>>> {
    // TODO: Implement task argument parsing
    // This mirrors the Node.js task-parser.js logic
    todo!()
}

pub struct TaskDefinition {
    pub name: String,
    pub payload: serde_json::Value,
}
```

### `src/config.rs`

```rust
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub browser: BrowserConfig,
    pub orchestrator: OrchestratorConfig,
}

#[derive(Debug, Deserialize)]
pub struct BrowserConfig {
    pub connectors: Vec<String>,
    pub connection_timeout_ms: u64,
    pub max_discovery_retries: u32,
    pub discovery_retry_delay_ms: u64,
    pub circuit_breaker: CircuitBreakerConfig,
    pub profiles: Vec<BrowserProfile>,
}

#[derive(Debug, Deserialize)]
pub struct CircuitBreakerConfig {
    pub enabled: bool,
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub half_open_time_ms: u64,
}

#[derive(Debug, Deserialize)]
pub struct BrowserProfile {
    pub name: String,
    pub r#type: String,
    pub ws_endpoint: String,
}

#[derive(Debug, Deserialize)]
pub struct OrchestratorConfig {
    pub max_global_concurrency: usize,
    pub task_timeout_ms: u64,
    pub group_timeout_ms: u64,
    pub worker_wait_timeout_ms: u64,
    pub stuck_worker_threshold_ms: u64,
    pub task_stagger_delay_ms: u64,
}

pub fn load_config() -> Result<Config> {
    // TODO: Load from config.toml with fallback to defaults
    todo!()
}
```

### `src/browser.rs`

```rust
use crate::config::Config;
use crate::session::Session;
use anyhow::Result;
use tracing::info;

pub async fn discover_browsers(config: &Config) -> Result<Vec<Session>> {
    let mut sessions = Vec::new();

    info!("Starting browser discovery...");

    for attempt in 1..=config.browser.max_discovery_retries {
        info!("Discovery attempt {}/{}", attempt, config.browser.max_discovery_retries);

        for profile in &config.browser.profiles {
            match connect_to_browser(profile, config).await {
                Ok(session) => {
                    info!("Connected to browser: {}", profile.name);
                    sessions.push(session);
                }
                Err(e) => {
                    info!("Failed to connect to {}: {}", profile.name, e);
                }
            }
        }

        if !sessions.is_empty() {
            break;
        }

        if attempt < config.browser.max_discovery_retries {
            tokio::time::sleep(
                std::time::Duration::from_millis(config.browser.discovery_retry_delay_ms)
            ).await;
        }
    }

    Ok(sessions)
}

async fn connect_to_browser(profile: &crate::config::BrowserProfile, config: &Config) -> Result<Session> {
    // TODO: Use chromiumoxide to connect to CDP endpoint
    todo!()
}
```

### `src/orchestrator.rs`

```rust
use crate::config::Config;
use crate::session::Session;
use anyhow::Result;

pub struct Orchestrator {
    config: Config,
    active_tasks: std::sync::atomic::AtomicUsize,
}

impl Orchestrator {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            active_tasks: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub async fn execute_group(
        &mut self,
        group: &[crate::cli::TaskDefinition],
        sessions: &[Session],
    ) -> Result<()> {
        // TODO: Implement group execution logic
        // Tasks within a group run in parallel across available browsers
        todo!()
    }
}
```

### `src/session.rs`

```rust
use chromiumoxide::Browser;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct Session {
    pub id: String,
    pub name: String,
    pub profile_type: String,
    pub browser: Browser,
    pub worker_semaphore: Arc<Semaphore>,
    pub active_workers: std::sync::atomic::AtomicUsize,
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
            browser,
            worker_semaphore: Arc::new(Semaphore::new(max_workers)),
            active_workers: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub async fn acquire_worker(&self) -> Option<tokio::sync::SemaphorePermit<'_>> {
        // TODO: Implement worker acquisition with timeout
        todo!()
    }

    pub async fn release_worker(&self, _permit: tokio::sync::SemaphorePermit<'_>) {
        // Worker released via permit drop
    }
}
```

### `task/mod.rs`

```rust
use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::page::Page;
use serde_json::Value;

pub mod cookiebot;
pub mod pageview;

pub type TaskFn = fn(&Page, Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>;

pub fn get_task(name: &str) -> Option<TaskFn> {
    match name {
        "cookiebot" | "cookiebot.js" => Some(cookiebot::run),
        "pageview" | "pageview.js" => Some(pageview::run),
        _ => None,
    }
}

pub async fn perform_task(page: &Page, name: &str, payload: Value) -> Result<()> {
    let task_fn = get_task(name)
        .ok_or_else(|| anyhow::anyhow!("Unknown task: {}", name))?;
    
    task_fn(page, payload).await
}
```

### `task/cookiebot.rs`

```rust
use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::page::Page;
use serde_json::Value;
use tracing::info;

pub async fn run(page: &Page, payload: Value) -> Result<()> {
    // TODO: Implement cookiebot task logic
    info!("Running cookiebot task");
    Ok(())
}
```

### `task/pageview.rs`

```rust
use anyhow::Result;
use chromiumoxide::cdp::browser_protocol::page::Page;
use serde_json::Value;
use tracing::info;

pub async fn run(page: &Page, payload: Value) -> Result<()> {
    // TODO: Implement pageview task logic
    info!("Running pageview task");
    Ok(())
}
```

### `src/utils/mod.rs`

```rust
pub mod navigation;
pub mod scroll;
pub mod mouse;
pub mod keyboard;
pub mod timing;
pub mod math;

// Re-export everything for easy importing: use crate::utils::*;
pub use navigation::*;
pub use scroll::*;
pub use mouse::*;
pub use keyboard::*;
pub use timing::*;
pub use math::*;
```

### `src/utils/navigation.rs`

```rust
use chromiumoxide::cdp::browser_protocol::page::Page;
use anyhow::Result;

pub async fn goto(page: &Page, url: &str, timeout_ms: u64) -> Result<()> {
    // TODO: Implement navigation with timeout
    todo!()
}

pub async fn wait_for_load(page: &Page, timeout_ms: u64) -> Result<()> {
    // TODO: Wait for page to be fully loaded
    todo!()
}
```

### `src/utils/timing.rs`

```rust
use tokio::time::{sleep, Duration};
use crate::utils::math::random_in_range;

pub async fn random_delay(min_ms: u64, max_ms: u64) {
    let delay = random_in_range(min_ms, max_ms);
    sleep(Duration::from_millis(delay)).await;
}

pub async fn human_pause(base_ms: u64, variance_pct: u32) {
    // TODO: Implement human-like pause with Gaussian distribution
    todo!()
}
```

### `src/utils/math.rs`

```rust
use rand::Rng;

pub fn random_in_range(min: u64, max: u64) -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}

pub fn gaussian(mean: f64, std_dev: f64, min: f64, max: f64) -> f64 {
    // TODO: Implement Gaussian random with bounds
    todo!()
}
```

### `src/utils/scroll.rs`, `src/utils/mouse.rs`, `src/utils/keyboard.rs`

```rust
// Stub implementations for now - will be implemented in Phase 3
```

---

## 1.8 Verify Compilation

```bash
cargo check
cargo build
cargo run
```

Expected: Compiles successfully and starts in idle mode (`No tasks specified`). Task execution is completed in later phases.

---

## Deliverables

- [ ] Project compiles with `cargo build`
- [ ] `cargo run` starts in idle mode without panicking (task execution is completed in later phases)
- [ ] Folder structure matches specification
- [ ] `config.toml.example` created with all browser types
- [ ] Data files copied to `data/` directory
- [ ] All skeleton files created with TODO placeholders

---

## Notes

- The `#[path = "../task/mod.rs"]` attribute in `main.rs` allows the `task/` folder to live at the project root while `src/` contains the engine code
- All task modules in `task/` export an async `run` function with signature: `async fn run(page: &Page, payload: Value) -> Result<()>`
- The `utils/mod.rs` barrel file enables clean imports: `use crate::utils::*;` in any task module
- `chromiumoxide` uses the Chrome DevTools Protocol (CDP) directly, equivalent to Playwright's `connectOverCDP` in the Node.js version
