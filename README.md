# Rust Orchestrator

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Tokio](https://img.shields.io/badge/Tokio-000000?style=for-the-badge&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

A high-performance, multi-browser automation framework built in Rust. Execute automated tasks across multiple browser sessions with advanced concurrency control, session management, and failure recovery.

## ✨ Features

### Core Capabilities
- 🚀 **High Performance**: Built with Rust and Tokio for maximum throughput
- 🌐 **Multi-Browser Support**: Connect to multiple browser instances simultaneously
- 📊 **Advanced Orchestration**: Task grouping, timeouts, retries, and circuit breakers
- 🔄 **Session Management**: Automatic browser session lifecycle management with health scoring
- 📈 **Metrics & Monitoring**: Built-in performance tracking, health checks, and JSON export
- 🛡️ **Production Ready**: Comprehensive error handling, graceful shutdown, and memory monitoring
- ⚙️ **Flexible Configuration**: TOML-based config with environment variable overrides
- ✅ **Node.js Parity**: Parser exactly mirrors `.nodejs-reference` task syntax and semantics

### Framework Surface
- `runtime`: owns browser/session/page lifecycle
- `capabilities`: stable task-facing actions like mouse, keyboard, clipboard, navigation, scroll
- `state`: session-scoped handles like `ClipboardState`
- `internal`: framework-only bridge layer for shared helpers
- `utils`: low-level implementation details and helper modules

### Human-Like Mouse Simulation
- **Cursor Movement**: 6 path styles (Bezier, Arc, Zigzag, Overshoot, Stopped, Muscle) with Gaussian distribution
- **Click Mechanics**: Pointer events (pointerenter, pointerleave, pointermove), element-aware hover phases, 80ms press duration
- **Timing**: Clustered pauses with micro-movements for reduced detection patterns
- **Element Detection**: Automatic element type detection (button, link, input, checkbox, dropdown) for behavior customization

### Future Browser Connectors
- Current browser discovery covers configured Chromium profiles, Roxybrowser, and local Brave instances.
- Planned next step is to add more connector backends behind the same runtime/session API.
- Goal: keep task code unchanged while expanding support for additional Chromium-compatible browsers and connector styles.
- Other Chromium-compatible browsers are planned as future connectors, not current default support.

### Production Hardening
- **Graceful Shutdown**: Ctrl+C signal handling with clean task cancellation
- **Exponential Backoff**: Smart retry with jitter to prevent thundering herd
- **Circuit Breaker**: Fault tolerance for API calls with half-open recovery
- **Health Scoring**: Real-time session health monitoring (0-100 score)
- **Memory Monitoring**: Threshold-based warnings for resource usage
- **Task Validation**: Startup validation with detailed error messages
- **Group Timeouts**: Hard-stop for batch execution to prevent hangs
- **Worker Health Checks**: Stale task cleanup and page registry management

## 📦 Installation

### Prerequisites

- Rust 1.70+ ([Install Rust](https://rustup.rs/))
- Current supported browsers: Brave and [RoxyBrowser](https://roxybrowser.com/)

### Build from Source

```bash
# Clone the repository
git clone <repository-url>
cd rust-orchestrator

# Build the project
cargo build --release

# Run tests
cargo test

# Run linter
cargo clippy --all-targets --all-features
```

The binary will be available at `target/release/rust-orchestrator`.

## 🚀 Quick Start

### Basic Usage

```bash
# Run a single task
cargo run cookiebot

# Run multiple tasks in sequence (then = new group)
cargo run cookiebot then pageview

# Run tasks with parameters
cargo run pageview=url=https://example.com

# Run with custom config
cargo run -- --config path/to/config.toml cookiebot
```

### Task Syntax

```
# Single task
cookiebot

# Task with URL parameter
pageview=www.reddit.com

# Task with explicit URL parameter
pageview=url=https://example.com

# Task with legacy value alias
pageview=value=https://example.com

# Multiple tasks in same group (run in parallel)
cookiebot pageview=reddit.com

# Multiple groups (sequential execution)
cookiebot pageview=reddit.com then cookiebot

# Task with .js extension (auto-stripped)
cookiebot.js pageview.js
```

### Parallel Fan-Out

- Task groups are intentionally broadcast to every active browser session.
- This keeps same-assigned tasks running in parallel across browsers.
- Future task files should assume concurrent multi-session execution by default.
- Partial failure is allowed when at least one session succeeds.

### Execution Model

- Validation and task execution share the same payload resolver.
- `pageview` accepts both `url` and the legacy `value` alias.
- Task code should stay thin and avoid re-implementing framework parsing rules.

## 📋 Available Tasks

### CookieBot (`cookiebot`)
Manages browser cookies and consent dialogs. Visits URLs from `data/cookiebot.txt`.

```bash
cargo run cookiebot
```

**Data File:** `data/cookiebot.txt`
```
https://example1.com
https://example2.com
# This is a comment
https://example3.com
```

### PageView (`pageview`)
Navigates to web pages and simulates human-like browsing behavior.

```bash
# Navigate to a URL
cargo run pageview=www.reddit.com

# With custom duration (2 minutes)
cargo run pageview=url=https://example.com,duration_ms=120000

# With custom scroll behavior
cargo run pageview=url=https://example.com,scroll_read_amount=400,scroll_read_pauses=3
```

**Payload Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `url` | string | required | Target URL |
| `value` | string | legacy alias | Alternate target URL input |
| `duration_ms` | u64 | 120000 | Task duration |
| `initial_pause_ms` | u64 | 1000 | Initial delay |
| `selector_wait_ms` | u64 | 6000 | Wait for visible content |
| `cursor_interval_min_ms` | u64 | from profile | Min cursor move interval |
| `cursor_interval_max_ms` | u64 | from profile | Max cursor move interval |
| `scroll_interval_min_ms` | u64 | from profile | Min scroll interval |
| `scroll_interval_max_ms` | u64 | from profile | Max scroll interval |
| `scroll_read_pauses` | u32 | 2 | Pauses during scroll read |
| `scroll_read_amount` | i32 | 650 | Scroll amount per burst |
| `scroll_read_variable_speed` | bool | true | Variable scroll speed |
| `scroll_read_back_scroll` | bool | false | Scroll back to top |

### DemoQA Text Box (`demoqa`)
Demonstrates the task-api on the DemoQA text box page: focus, keyboard input, random cursor movement, submit, and output verification.
It writes a fixed sample record:
- Full Name: `Demo QA`
- Email: `demoqa@example.com`
- Current Address: `123 Demo Street, Demo City`
- Permanent Address: `456 Demo Avenue, Demo Town`

```bash
cargo run demoqa
```

### Twitter Activity (`twitteractivity`)
Simulates human-like Twitter/X engagement with persona-based behavior.

```bash
# Run with default persona
cargo run twitteractivity

# Run with custom duration and engagement limits
cargo run twitteractivity,duration_ms=120000,scroll_count=12

# Run with custom persona weights
cargo run 'twitteractivity,weights={"like_prob":0.4,"retweet_prob":0.15,"follow_prob":0.05}'
```

**Features:**
- 🎭 **Persona-Based Behavior**: 21 preset personas (teen, senior, professional, etc.)
- ❤️ **Like Tweets**: Human-like cursor movement and timing
- 🔁 **Retweet**: Native retweets with modal confirmation
- 👤 **Follow Users**: From tweet context or profile pages
- 💬 **Reply**: Context-aware reply composition (V2: LLM-powered)
- 🧵 **Thread Dives**: Read full conversation threads
- 🔖 **Bookmark**: Save tweets (disabled in V1, config-driven)

**Engagement Limits (Default):**
| Action | Limit | Configurable |
|--------|-------|--------------|
| Likes | 5 | `TWITTER_MAX_LIKES` |
| Retweets | 3 | `TWITTER_MAX_RETWEETS` |
| Follows | 2 | `TWITTER_MAX_FOLLOWS` |
| Replies | 1 | `TWITTER_MAX_REPLIES` |
| Thread Dives | 3 | `TWITTER_MAX_THREAD_DIVES` |
| **Total** | **10** | `TWITTER_MAX_TOTAL_ACTIONS` |

**Configuration:**
```toml
[twitter_activity]
feed_scan_duration_ms = 120000    # 2 minutes
feed_scroll_count = 12             # Scroll actions
engagement_candidate_count = 5     # Tweets to consider

[twitter_activity.engagement_limits]
max_likes = 5
max_retweets = 3
max_follows = 2
max_replies = 1
max_thread_dives = 3
max_bookmarks = 0                  # Disabled in V1
max_total_actions = 10

# LLM Configuration (V2 - for smart replies & quote tweets)
[twitter_activity.llm]
enabled = false                    # Set true for V2 features
provider = "ollama"
model = "llama3.2:latest"
```

**Payload Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `duration_ms` | u64 | 120000 | Session duration |
| `scroll_count` | u32 | 12 | Scroll actions |
| `candidate_count` | u32 | 5 | Engagement candidates |
| `weights` | object | persona | Engagement probabilities |
| `profile` | string | Average | Persona preset |

**Persona Presets:**
`Average`, `Teen`, `Senior`, `Enthusiast`, `PowerUser`, `Cautious`, `Impatient`, `Erratic`, `Researcher`, `Casual`, `Professional`, `Novice`, `Expert`, `Distracted`, `Focused`, `Analytical`, `QuickScanner`, `Thorough`, `Adaptive`, `Stressed`, `Leisure`

---

### Twitter Follow (`twitterfollow`)
Navigate to Twitter/X profiles and follow users with human-like behavior.

```bash
# Follow from profile URL
cargo run twitterfollow=url=https://x.com/username

# Follow from tweet URL (navigates to profile)
cargo run twitterfollow=url=https://x.com/user/status/123

# Follow with username directly
cargo run 'twitterfollow={"username":"username"}'
```

**Features:**
- 🎯 **Smart URL Detection**: Handles profile URLs, tweet URLs, usernames
- ✅ **Already Following Check**: Prevents duplicate follows
- ⏳ **Pending State Handling**: Waits for follow confirmation
- 🔄 **Retry Logic**: Page reload on failure (up to 7 attempts)
- 🛡️ **Rate Limit Detection**: Skips actions when rate-limited
- 👻 **Popup Dismissal**: Handles login/signup modals

**Rate Limit Signals Detected:**
- "Rate limit"
- "Too many attempts"
- "Try again later"
- "You have been rate limited"
- "Temporary restriction"
- "Something went wrong"
- "Unable to follow"

---

### Twitter Reply (`twitterreply`)
Extract tweet context and compose human-like replies.

```bash
# Reply to specific tweet
cargo run twitterreply=url=https://x.com/user/status/123
```

**Features:**
- 📝 **Context Extraction**: Reads tweet + top replies
- 🎯 **Sentiment Analysis**: Matches reply tone to context
- ⚡ **Quick Generation**: Fast, template-based replies
- 🔜 **V2: LLM-Powered**: Contextual AI-generated replies

**V2 (Planned):**
- LLM-powered contextual replies
- Multi-turn conversation tracking
- Language matching (reply in tweet's language)
- Smart engagement decisions (skip low-quality tweets)

See `V2_ROADMAP.md` for implementation timeline.

## ⚙️ Configuration

### Configuration File

Create `config/default.toml`:

```toml
# Browser Configuration
[browser]
max_discovery_retries = 3
discovery_retry_delay_ms = 5000
user_agent = "Mozilla/5.0 ..."  # Optional global user agent

# Extra HTTP headers (optional)
[browser.extra_http_headers]
Accept-Language = "en-US,en;q=0.9"
DNT = "1"

# Browser Profiles
[[browser.profiles]]
name = "brave-local"
type = "brave"
ws_endpoint = ""  # Empty for auto-discovery

[[browser.profiles]]
name = "chromium-remote"
type = "chromium"
ws_endpoint = "ws://192.168.1.100:9222"

# RoxyBrowser Integration
[browser.roxybrowser]
enabled = true
api_url = "http://127.0.0.1:50000/"
api_key = "your-api-key-here"

# Circuit Breaker Configuration
[browser.circuit_breaker]
enabled = true
failure_threshold = 5
success_threshold = 3
half_open_time_ms = 30000

# Orchestrator Settings
[orchestrator]
max_global_concurrency = 20      # Max concurrent tasks (1-100)
task_timeout_ms = 600000         # Individual task timeout (10min)
group_timeout_ms = 600000        # Group timeout (10min)
worker_wait_timeout_ms = 10000   # Worker acquisition timeout
stuck_worker_threshold_ms = 120000
task_stagger_delay_ms = 2000     # Delay between task starts
max_retries = 2                  # Retry attempts (0-10 recommended)
retry_delay_ms = 500             # Initial retry delay

# Twitter Activity Configuration
[twitter_activity]
feed_scan_duration_ms = 60000
feed_scroll_count = 10
engagement_candidate_count = 5
persona_file_path = "data/persona.json"  # Optional
```

### Environment Variables

Override configuration at runtime:

```bash
# RoxyBrowser API
export ROXYBROWSER_API_URL="https://api.roxybrowser.com/"
export ROXYBROWSER_API_KEY="your-api-key"

# Orchestrator Settings
export MAX_GLOBAL_CONCURRENCY="10"
export TASK_TIMEOUT_MS="300000"
export MAX_RETRIES="3"

# Browser Settings
export BROWSER_USER_AGENT="Mozilla/5.0 ..."
export BROWSER_EXTRA_HTTP_HEADERS="Accept-Language=en-US,DNT=1"

# Logging
export RUST_LOG="info,orchestrator=debug"
```

### Configuration Validation

The orchestrator validates configuration at startup:

| Setting | Valid Range | Default | Validation |
|---------|-------------|---------|------------|
| `max_global_concurrency` | 1-100 | 20 | Error if 0 or >100 |
| `task_timeout_ms` | >5000 | 600000 | Warn if <5s or >1h |
| `group_timeout_ms` | >0 | 600000 | Error if < task_timeout |
| `max_retries` | 0-10 | 2 | Warn if >10 |
| `retry_delay_ms` | >0 | 500 | Warn if 0 or >30s |

### Task Parser Parity

The Rust `parse_task_groups()` function is **behaviorally identical** to the Node.js reference implementation (`.nodejs-reference/api/utils/task-parser.js`). All edge cases are covered:

- **Parameter merging**: `["cookiebot", "pageview=reddit.com"]` → 1 task with `pageview` param
- **Shorthand repeat**: `["follow=x.com", "follow=y.com"]` → 2 separate tasks
- **Numeric values**: Stored as strings (compatible with task payload consumption)
- **URL auto-format**: `pageview=www.reddit.com` → `https://www.reddit.com`
- **Empty arguments**: Silently skipped
- **Explicit `url=` override**: `pageview=url=https://example.com` → uses explicit value
- **Equal signs in values**: `task=value=with=equals` → entire value treated as URL
- **Query params**: `pageview=example.com?q=test&page=1` → preserved intact

## 🔧 Advanced Features

### Retry with Exponential Backoff

Tasks automatically retry with intelligent backoff:

```
Attempt 1: Immediate
Attempt 2: 500ms + jitter (0-30%)
Attempt 3: 1000ms + jitter
Attempt 4: 2000ms + jitter
...
Max delay capped at 30 seconds
```

**Retryable Errors:**
- Timeouts
- Network failures
- Connection errors
- Temporary unavailability

**Non-Retryable Errors:**
- Validation failures
- Authentication errors
- Permanent failures

### Circuit Breaker Pattern

Protects against cascading failures:

```
State: CLOSED (normal operation)
  ↓ (5 consecutive failures)
State: OPEN (blocking requests)
  ↓ (30 second timeout)
State: HALF_OPEN (testing)
  ↓ (3 consecutive successes)
State: CLOSED (recovered)
```

### Health Monitoring

Real-time session health scoring (0-100):

```
Health Score Calculation:
- Start: 100 points
- Consecutive failures: -10 each (max -50)
- Total failures: -1 per 10 (max -30)
- Successes: +5 each (max +20)

Health States:
- Healthy: 0-2 consecutive failures
- Degraded: 3-4 consecutive failures
- Unhealthy: 5+ consecutive failures
```

### Graceful Shutdown

Press `Ctrl+C` to gracefully shutdown:

```
1. Shutdown signal received
2. Stop starting new task groups
3. Wait for in-flight tasks to complete (with timeout)
4. Export metrics summary
5. Log final health status
6. Clean browser session cleanup
```

## 📊 Observability

### Metrics Export

Automatic export to `run-summary.json`:

```json
{
  "timestamp": "2025-01-15T10:30:00Z",
  "total_tasks": 42,
  "succeeded": 38,
  "failed": 3,
  "timed_out": 1,
  "success_rate": 90.48,
  "total_duration_ms": 1234567
}
```

### Periodic Health Monitoring

Background task logs system health every 60 seconds:

```
[INFO] [health] active_tasks=5 total_tasks=42 success_rate=90.5% memory=256.3 MiB/16384.0 MiB (1.6%)
[WARN] [health] Memory usage 1024.5 MiB exceeds threshold 1024.0 MiB
[INFO] [health-detail] failures=3 timeouts=1 avg_duration_ms=28540
```

Thresholds trigger warnings:
- Active tasks > 50 → WARNING, > 100 → CRITICAL
- Memory usage exceeds configurable threshold (default 1 GiB) → WARNING
- Health logger runs independently, does not block shutdown

## 🎯 Current Status

### Completed (Fast Track — Production Ready ✅)

All core orchestration features are implemented and tested:

| Phase | Feature | Status |
|-------|---------|--------|
| 0 | Unified result types, error typing | ✅ |
| 1 | Per-task timeout, group timeout, retry metadata, health checks | ✅ |
| 2 | Full `connect_to_browser`, session state tracking, page registry, graceful shutdown | ✅ |
| 3 | Config loader (TOML + env), task validator, **parser parity with Node.js**, startup validation | ✅ |
| 4 | HTTP client with retry (exponential backoff + jitter), circuit breaker (feature-flagged) | ✅ |
| 5 | Metrics collector, task history ring buffer, `run-summary.json` export, periodic health logging | ✅ |
| 6 | JS fallback path, deterministic utility tests, integration tests | ✅ |

Last verified on April 21, 2026.

**Test coverage:** 68 tests passing (unit + integration + doc tests)  
**Lint status:** `cargo clippy` clean (all targets, all features)  
**Node.js parity:** Parser behavior verified across all edge cases

### Optional (Not Required for Core Functionality)

| Feature | Description | Status |
|---------|-------------|--------|
| Provider fallback strategy | OpenRouter multi-model + API key rotation + proxy fallback (FreeApiRouter parity) | ⏸️ Not implemented |
| Additional circuit-breaker stats | Extended metrics for circuit breaker states | ⏸️ Deferred |

> The system is production-stable without the optional AI provider routing layer. Core browser automation, task orchestration, session management, and observability are fully implemented.

### Log Levels

```bash
# Production (default)
RUST_LOG="info"

# Debug task execution
RUST_LOG="info,orchestrator=debug"

# Verbose with browser debugging
RUST_LOG="debug,chromiumoxide=warn"

# JSON structured logging
# (configured in code)
```

## 🧪 Testing

### Run All Tests

```bash
cargo test
```

### Run Specific Tests

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test graceful_shutdown_integration

# Specific test
cargo test test_shutdown_channel_signal

# With output
cargo test -- --nocapture
```

### Linting and Formatting

```bash
# Run clippy linter
cargo clippy --all-targets --all-features

# Auto-fix clippy warnings
cargo clippy --all-targets --all-features --fix

# Format code with rustfmt
cargo fmt

# Check without building
cargo check
```

### Build

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Build all features enabled
cargo build --all-features
```

## 🐛 Troubleshooting

### Common Issues

**No browsers discovered:**
```
Solution: Ensure browser is running with remote debugging enabled
Brave: brave.exe --remote-debugging-port=9001
Chrome: chrome.exe --remote-debugging-port=9222
```

**Task validation errors:**
```
Solution: Check task name spelling and payload format
cargo run --help  # See available tasks
```

**Circuit breaker open:**
```
Solution: Check API endpoint availability
- Verify RoxyBrowser is running
- Check network connectivity
- Review API logs
```

**Memory warnings:**
```
Solution: Reduce concurrency or add memory
export MAX_GLOBAL_CONCURRENCY=10
```

### Getting Help

1. Check logs with `RUST_LOG=debug`
2. Review `run-summary.json` for failure details
3. Inspect health logs for session issues
4. Verify configuration with `config::validate_config()`

## 📚 API Reference

### Task Authoring

Create new tasks in `task/` directory:

How to author a new task:
- Use `TaskContext` only.
- Read and normalize payload fields first.
- Keep task logic small and deterministic.
- Reuse `api.*` verbs instead of low-level helpers.
- Add the task to `task/mod.rs`.

```rust
use rust_orchestrator::prelude::*;
use serde_json::Value;

pub async fn run(api: &TaskContext, payload: Value) -> anyhow::Result<()> {
    // Navigate to URL
    let url = payload.get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing url"))?;
    
    api.navigate_to(url, 30000).await?;
    
    // Human-like behavior
    api.focus("input, textarea").await?;
    api.keyboard("input, textarea", "Hello").await?;
    api.pause(1000).await;
    api.random_scroll().await?;
    
    Ok(())
}
```

### Register New Task

Add to `task/mod.rs`:

```rust
pub mod my_new_task;

// In perform_task():
"my_new_task" => my_new_task::run(api, payload.clone()).await,
```

Note: `TaskContext` is the task-api. `keyboard(...)` is the preferred typing verb. `api.r#type(...)` remains available if you want the Rust-keyword-safe alias.

High-level task-api verbs already include a short settle pause after the action. `api.pause(base_ms)` is the explicit uniform wait helper and uses a 20% deviation band.
`api.click(selector)` is the default click mechanism: selector pipeline with scroll + move + click. Use coordinate clicks only when you already have coordinates or the selector path is not appropriate.

Common task-api verbs now include `api.click(...)`, `api.click_and_wait(...)`, `api.double_click(...)`, `api.middle_click(...)`, `api.right_click(...)`, `api.drag(...)`, `api.focus(...)`, `api.hover(...)`, `api.keyboard(...)`, `api.randomcursor()`, `api.clear(...)`, `api.select_all(...)`, `api.exists(...)`, `api.visible(...)`, `api.text(...)`, `api.html(...)`, `api.attr(...)`, `api.wait_for(...)`, `api.wait_for_visible(...)`, `api.scroll_to(...)`, `api.url()`, and `api.title()`.

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow Rust best practices and idioms
- Add tests for new functionality
- Update documentation for API changes
- Run `cargo clippy` and `cargo fmt` before submitting
- Ensure all tests pass with `cargo test`

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🙏 Acknowledgments

- Built with [Chromium Oxide](https://crates.io/crates/chromiumoxide) for browser automation
- Inspired by the Node.js reference implementation (`.nodejs-reference/`)
- Thanks to the Rust community for excellent crates and tooling:
  - [Tokio](https://tokio.rs/) for async runtime
  - [anyhow](https://crates.io/crates/anyhow) for error handling
  - [serde](https://serde.rs/) for JSON serialization
  - [clap](https://crates.io/crates/clap) for CLI parsing
  - [reqwest](https://crates.io/crates/reqwest) for HTTP client

## 📊 Performance Benchmarks

| Metric | Value | Notes |
|--------|-------|-------|
| Task Throughput | ~50 tasks/sec | With 20 concurrent sessions |
| Memory Usage | ~50-200 MB | Depends on concurrency |
| Startup Time | <2 seconds | Including browser discovery |
| Shutdown Time | <5 seconds | Graceful cleanup |

---

**Note**: This is a Rust port of an existing Node.js browser automation system, providing better performance and reliability for production workloads.

