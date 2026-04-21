# Auto-Rust Project Context

## Project Overview

**Auto-Rust** is a high-performance, multi-browser automation framework built in Rust. It executes automated tasks across multiple browser sessions with advanced concurrency control, session management, and failure recovery. The project is a Rust port of a Node.js browser automation system, providing better performance and reliability for production workloads.

**Key Technologies:**
- **Language:** Rust 2021 edition
- **Browser Automation:** chromiumoxide (native CDP, no Node.js dependency)
- **Async Runtime:** Tokio
- **HTTP Client:** reqwest with circuit breaker pattern
- **Configuration:** TOML-based config with environment variable overrides
- **CLI:** clap-derived argument parsing

**Supported Browsers:** Brave and RoxyBrowser (other Chromium browsers planned as future connectors)

---

## Project Structure

```
C:\My Script\auto-rust\
├── Cargo.toml              # Project manifest & dependencies
├── Cargo.lock              # Locked dependency versions
├── config.toml.example     # Configuration template
├── .env.example            # Environment variables template
├── README.md               # User-facing documentation
├── AGENTS.md               # MCP tool usage guidelines
├── V2_ROADMAP.md           # Twitter Activity V2 implementation plan
├── LLM_SETUP.md            # LLM (Ollama/OpenRouter) setup guide
├── whitepaper.md           # Migration plan from Node.js
│
├── config/                 # Runtime configuration files
│   └── default.toml        # Active configuration
│
├── data/                   # Task data files
│   ├── cookiebot.txt       # URLs for cookiebot task
│   ├── pageview.txt        # URLs for pageview task
│   └── referrer.txt        # Referrer URLs
│
├── task/                   # Task implementations (one file per task)
│   ├── mod.rs              # Task registry & dispatcher
│   ├── cookiebot.rs        # Cookie consent management
│   ├── pageview.rs         # Human-like page browsing
│   ├── demoqa.rs           # DemoQA form interaction
│   ├── twitteractivity.rs  # Twitter/X engagement simulation
│   ├── twitterfollow.rs    # Twitter follow automation
│   ├── twitterreply.rs     # Twitter reply generation
│   └── ...
│
├── src/                    # Core engine source code
│   ├── main.rs             # Entry point (thin wrapper)
│   ├── lib.rs              # Library exports & prelude
│   ├── cli.rs              # CLI argument parsing & task groups
│   ├── config.rs           # Configuration loader & validator
│   ├── browser.rs          # Browser discovery & connection
│   ├── orchestrator.rs     # Task group execution coordinator
│   ├── worker_pool.rs      # Worker management & health tracking
│   ├── session.rs          # Session lifecycle management
│   ├── metrics.rs          # Metrics collection & export
│   ├── health_monitor.rs   # System health monitoring
│   ├── health_logger.rs    # Periodic health logging
│   ├── result.rs           # Unified task result types
│   ├── logger.rs           # Logging configuration
│   │
│   ├── api/                # HTTP API client layer
│   │   └── client.rs       # ApiClient with circuit breaker & retry
│   │
│   ├── capabilities/       # Browser capability modules
│   │   ├── mouse.rs        # Human-like cursor movement & clicks
│   │   ├── keyboard.rs     # Natural typing with timing profiles
│   │   ├── navigation.rs   # Page navigation & selector queries
│   │   ├── scroll.rs       # Scroll behaviors (random, read, etc.)
│   │   ├── timing.rs       # Human-like pauses & delays
│   │   └── clipboard.rs    # Clipboard operations
│   │
│   ├── internal/           # Framework-internal utilities
│   │   ├── page_size.rs    # Viewport & element geometry
│   │   └── profile.rs      # Browser profile definitions
│   │
│   ├── runtime/            # Runtime task context
│   │   └── task_context.rs # TaskContext API (task authors use this)
│   │
│   ├── state/              # Session-scoped state handles
│   │   └── clipboard.rs    # Clipboard state management
│   │
│   ├── utils/              # Low-level helpers
│   │   └── mouse.rs        # Cursor movement paths (Bezier, Arc, etc.)
│   │
│   ├── validation/         # Task payload validation
│   │
│   └── tests/              # Integration tests
│       └── ...
│
├── tests/                  # Top-level test files
│   ├── graceful_shutdown_integration.rs
│   ├── soak_test.rs
│   └── twitteractivity_integration.rs
│
└── examples/               # Example task implementations
    ├── demo-interaction-keyboard.rs
    └── demo-interaction-mouse.rs
```

---

## Building and Running

### Prerequisites
- **Rust 1.70+** ([Install via rustup](https://rustup.rs/))
- **Browsers:** Brave or RoxyBrowser running with remote debugging enabled

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Build with all features
cargo build --all-features

# Check without building
cargo check
```

### Run Commands

```bash
# Run a single task
cargo run cookiebot

# Run multiple tasks in same group (parallel)
cargo run cookiebot pageview=www.reddit.com

# Run sequential groups (then = new group)
cargo run cookiebot then pageview

# Run with parameters
cargo run pageview=url=https://example.com,duration_ms=120000

# Run with custom config
cargo run -- --config path/to/config.toml cookiebot

# Run Twitter activity with persona
cargo run twitteractivity,profile=Professional,scroll_count=12
```

### Test Commands

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests
cargo test --test graceful_shutdown_integration

# Run specific test
cargo test test_shutdown_channel_signal

# Run tests with output
cargo test -- --nocapture
```

### Linting and Formatting

```bash
# Run clippy linter
cargo clippy --all-targets --all-features

# Auto-fix clippy warnings
cargo clippy --all-targets --all-features --fix

# Format code
cargo fmt

# Check format
cargo fmt --check
```

---

## Task API (For Task Authors)

The `TaskContext` is the primary API for writing tasks. Import via `use rust_orchestrator::prelude::*;`

### Core Verbs

```rust
// Navigation
api.navigate("https://example.com", 30000).await?;
api.wait_for_load(5000).await?;

// Selector queries
api.exists(".button").await?;           // Check if element exists
api.visible(".button").await?;          // Check if element visible
api.text(".title").await?;              // Get text content
api.html(".container").await?;          // Get inner HTML
api.attr(".link", "href").await?;       // Get attribute
api.value("input").await?;              // Get input value

// Waiting
api.wait_for(".element", 5000).await?;         // Wait for selector
api.wait_for_visible(".element", 5000).await?; // Wait for visibility

// Mouse interactions
api.click(".button").await?;                   // Click with selector
api.click_and_wait(".btn", ".next", 3000).await?; // Click + wait
api.double_click(".item").await?;              // Double click
api.right_click(".menu").await?;               // Right click
api.middle_click(".tab").await?;               // Middle click
api.hover(".dropdown").await?;                 // Hover
api.drag(".from", ".to").await?;               // Drag and drop
api.click_at(100.0, 200.0).await?;             // Click at coordinates

// Cursor movement
api.randomcursor().await?;                     // Move to random position
api.move_mouse_to(100.0, 200.0).await?;        // Move to coordinates
api.move_mouse_fast(100.0, 200.0).await?;      // Immediate move

// Keyboard
api.focus("input").await?;                     // Focus element
api.keyboard("input", "Hello").await?;         // Type text
api.r#type("input", "World").await?;           // Alias for keyboard()
api.press("Enter").await?;                     // Press single key
api.press_with_modifiers("C", &["Ctrl"]).await?; // Ctrl+C

// Clipboard
api.copy().await?;                             // Select all + copy
api.paste().await?;                            // Paste into focused
api.cut().await?;                              // Cut to clipboard

// Scrolling
api.random_scroll().await?;                    // Random scroll
api.scroll_to(".section").await?;              // Scroll to element
api.scroll_read(4, 650, true, false).await?;   // Read with pauses
api.scroll_to_top().await?;                    // Scroll to top
api.scroll_to_bottom().await?;                 // Scroll to bottom
api.scroll_back(500).await?;                   // Scroll back

// Timing
api.pause(1000).await;                         // Wait 1s (±20%)
api.pause_with_variance(2000, 30).await;       // Wait 2s (±30%)

// Page info
api.url().await?;                              // Get current URL
api.title().await?;                            // Get page title
```

### Task Template

```rust
use rust_orchestrator::prelude::*;
use serde_json::Value;

pub async fn run(api: &TaskContext, payload: Value) -> anyhow::Result<()> {
    // Extract parameters from payload
    let url = payload.get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing url"))?;

    // Navigate to URL
    api.navigate(url, 30000).await?;

    // Perform actions
    api.focus("input, textarea").await?;
    api.keyboard("input", "Hello World").await?;
    api.pause(1000).await;
    api.click(".submit-button").await?;

    // Verify outcome
    if api.visible(".success-message").await? {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Success message not found"))
    }
}
```

### Registering New Tasks

1. Create `task/my_new_task.rs` with `pub async fn run(api: &TaskContext, payload: Value) -> anyhow::Result<()>`
2. Add module declaration in `task/mod.rs`: `pub mod my_new_task;`
3. Add to `TASK_NAMES` array in `task/mod.rs`
4. Add match arm in `execute_single_attempt()` function

---

## Configuration

### Configuration File (`config/default.toml`)

```toml
# Browser Configuration
[browser]
max_discovery_retries = 3
discovery_retry_delay_ms = 5000

[[browser.profiles]]
name = "brave-local"
type = "localBrave"
ws_endpoint = "ws://localhost:9001"

[[browser.profiles]]
name = "chromium-remote"
type = "chromium"
ws_endpoint = "ws://192.168.1.100:9222"

# Circuit Breaker
[browser.circuit_breaker]
enabled = true
failure_threshold = 5
success_threshold = 3
half_open_time_ms = 30000

# Orchestrator Settings
[orchestrator]
max_global_concurrency = 20      # Max concurrent tasks
task_timeout_ms = 600000         # 10 minutes per task
group_timeout_ms = 600000        # 10 minutes per group
max_retries = 2                  # Retry attempts
retry_delay_ms = 500             # Initial retry delay
task_stagger_delay_ms = 2000     # Delay between task starts
```

### Environment Variables

```bash
# LLM Configuration
LLM_PROVIDER=ollama
OLLAMA_URL=http://localhost:11434
OLLAMA_MODEL=llama3.2:3b

# OpenRouter (alternative)
OPENROUTER_API_KEY=sk-or-v1-...
OPENROUTER_MODEL=anthropic/claude-3-haiku

# Twitter LLM Features
TWITTER_LLM_ENABLED=false
TWITTER_LLM_REPLY_PROBABILITY=0.05
TWITTER_LLM_QUOTE_PROBABILITY=0.15

# Engagement Limits
TWITTER_MAX_LIKES=5
TWITTER_MAX_RETWEETS=3
TWITTER_MAX_FOLLOWS=2
TWITTER_MAX_TOTAL_ACTIONS=10

# Browser
ROXYBROWSER_API_URL=http://127.0.0.1:50000/
ROXYBROWSER_API_KEY=your-api-key
BROWSER_USER_AGENT=Mozilla/5.0...

# Orchestrator
MAX_GLOBAL_CONCURRENCY=20
TASK_TIMEOUT_MS=600000
MAX_RETRIES=2
```

---

## Available Tasks

### Core Tasks

| Task | Description | Parameters |
|------|-------------|------------|
| `cookiebot` | Manage cookie consent dialogs | Reads from `data/cookiebot.txt` |
| `pageview` | Human-like page browsing | `url`, `duration_ms`, `scroll_read_pauses`, `scroll_read_amount` |
| `demoqa` | DemoQA form interaction | None (fixed test data) |

### Twitter/X Tasks

| Task | Description | Parameters |
|------|-------------|------------|
| `twitteractivity` | Persona-based engagement | `duration_ms`, `scroll_count`, `profile`, `weights` |
| `twitterfollow` | Follow users | `url` (profile or tweet URL) |
| `twitterlike` | Like tweets | Internal use |
| `twitterretweet` | Retweet | Internal use |
| `twitterreply` | Reply to tweets | `url` (tweet URL) |
| `twitterquote` | Quote tweet | Internal use |
| `twitterdive` | Thread exploration | Internal use |

---

## Development Conventions

### Code Style
- Follow Rust idioms and best practices
- Use `anyhow::Result` for error handling
- Add doc comments for public APIs
- Keep tasks thin: compose capabilities, don't duplicate logic

### Testing Practices
- Unit tests in the same file as implementation (using `#[cfg(test)]`)
- Integration tests in `tests/` directory
- Test names: `test_<feature>_<scenario>`
- Run `cargo test` before committing

### Task Authoring Guidelines
- Accept `api: &TaskContext` and `payload: Value`
- Use task-api verbs exclusively (don't call chromiumoxide directly)
- Handle errors with `?` operator
- Add verification step at end of task
- Keep task names canonical (lowercase, no spaces)

### Logging
```rust
use log::{info, warn, error, debug};

info!("Task started");
debug!("Detailed debug info");
warn!("Warning condition");
error!("Error occurred");
```

### Error Handling
- Use `anyhow::anyhow!()` for custom errors
- Classify errors using `TaskErrorKind::classify()`
- Retryable errors: Timeout, Navigation, Session, Browser, Unknown
- Non-retryable: Validation errors

---

## Architecture Highlights

### Session Management
- Sessions connect to pre-existing browser instances
- Each session has a managed page registry
- Health scoring (0-100) based on consecutive failures
- Graceful shutdown with cleanup

### Retry Strategy
- Exponential backoff with jitter
- Default: 3 retries, 200ms initial delay, 2x factor, 30% jitter
- Max delay capped at 10 seconds
- Circuit breaker protection (5 failures → open, 30s timeout)

### Task Execution Flow
```
1. Parse CLI args into task groups
2. Discover browsers from config
3. For each group:
   a. Acquire workers (browser sessions)
   b. Execute tasks in parallel
   c. Collect results with retry logic
   d. Update metrics and health scores
4. Export run-summary.json
5. Graceful shutdown on Ctrl+C
```

### Mouse Movement
- 6 path styles: Bezier, Arc, Zigzag, Overshoot, Stopped, Muscle
- Gaussian distribution for natural movement
- Element-aware hover phases
- 80ms press duration for clicks
- Clustered pauses with micro-movements

---

## Debugging

### Log Levels
```bash
# Production (default)
RUST_LOG=info

# Debug task execution
RUST_LOG=info,orchestrator=debug

# Verbose with browser debugging
RUST_LOG=debug,chromiumoxide=warn

# JSON structured logging
RUST_LOG=json
```

### Common Issues

**No browsers discovered:**
```bash
# Ensure browser is running with remote debugging
brave.exe --remote-debugging-port=9001
chrome.exe --remote-debugging-port=9222
```

**Task validation errors:**
```bash
# Check task name spelling
cargo run --help

# See available tasks in task/mod.rs
```

**Circuit breaker open:**
```bash
# Check API endpoint availability
# Verify RoxyBrowser is running
# Review logs for failure details
```

**Memory warnings:**
```bash
# Reduce concurrency
export MAX_GLOBAL_CONCURRENCY=10
```

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `src/lib.rs` | Library exports, prelude module |
| `src/main.rs` | Entry point, shutdown handling |
| `src/cli.rs` | CLI parsing, task group logic |
| `src/result.rs` | Task result types, error classification |
| `src/orchestrator.rs` | Task group execution coordinator |
| `task/mod.rs` | Task registry, dispatcher |
| `src/runtime/task_context.rs` | TaskContext API implementation |
| `src/api/client.rs` | HTTP client with circuit breaker |
| `config.rs` | Configuration loading & validation |
| `browser.rs` | Browser discovery & connection |

---

## Related Documentation

- **README.md**: User-facing documentation with full feature list
- **AGENTS.md**: MCP tool usage guidelines for this project
- **V2_ROADMAP.md**: Twitter Activity V2 (LLM-powered) implementation plan
- **LLM_SETUP.md**: Ollama/OpenRouter setup guide
- **whitepaper.md**: Migration plan from Node.js to Rust

---

## Quick Reference

```bash
# Most common commands
cargo run cookiebot                           # Run cookiebot task
cargo run pageview=url=https://example.com    # Run pageview with URL
cargo run task1 then task2                    # Sequential groups
cargo test                                    # Run tests
cargo clippy --all-targets --all-features     # Lint
cargo fmt                                     # Format

# Environment setup
cp .env.example .env                          # Create env file
cp config.toml.example config.toml            # Create config
```
