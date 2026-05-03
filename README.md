# Auto

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Tokio](https://img.shields.io/badge/Tokio-000000?style=for-the-badge&logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

A high-performance, multi-browser automation framework built in Rust. Execute automated tasks across multiple browser sessions with advanced concurrency control, session management, and failure recovery.

## 📑 Table of Contents --

- [Why Auto?](#why-auto)
- [Quick Start](#quick-start)
- [Key Features](#key-features)
- [Installation](#installation)
- [Basic Usage](#basic-usage)
- [Architecture](#architecture)
- [Current Status](#current-status)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)
- [Documentation](#documentation)
- [Contributing](#contributing)

## Why Auto?

**From Node.js to Rust:** A port of an existing Node.js browser automation system, delivering:

- **10x throughput** with Tokio async runtime (~50 tasks/sec with 20 sessions)
- **<2 second startup** including browser discovery
- **~50-200 MB memory** footprint (vs Node.js baseline)
- **Production-grade reliability** with graceful shutdown, circuit breakers, and health monitoring

## Quick Start

```bash
# 1. Start Brave with remote debugging
brave.exe --remote-debugging-port=9001

# 2. Run a single task
cargo run cookiebot

# 3. Run with parameters
cargo run pageview=url=https://reddit.com

# 4. Chain tasks
cargo run cookiebot then pageview=reddit.com then twitteractivity
```

**Task syntax:** `taskname=value` or `taskname=url=https://...` | Use `then` for sequential groups.

## Key Features

| Feature | Description |
|---------|-------------|
| 🚀 **High Performance** | Rust + Tokio for maximum throughput |
| 🌐 **Multi-Browser** | Connect multiple Brave/RoxyBrowser instances |
| 📊 **Parallel Fan-Out** | Task groups broadcast to all sessions |
| 🎭 **Human-Like Behavior** | 6 cursor path styles, 21 personas, Gaussian timing |
| 🛡️ **Production Ready** | Graceful shutdown, circuit breakers, health scoring |
| 📈 **Observability** | `run-summary.json` export + periodic health logging |
| ⚙️ **Flexible Config** | TOML + environment variable overrides |

**Human-Like Details:**
- **Cursor:** Bezier, Arc, Zigzag, Overshoot, Stopped, Muscle paths
- **Timing:** Clustered pauses with micro-movements, 80ms press duration
- **Personas:** Average, Teen, Senior, Professional, Enthusiast, PowerUser, Cautious, Impatient, Erratic, Researcher, Casual, Novice, Expert, Distracted, Focused, Analytical, QuickScanner, Thorough, Adaptive, Stressed, Leisure

## Installation

### Prerequisites

- Rust 1.70+ ([rustup.rs](https://rustup.rs/))
- Brave browser (or RoxyBrowser API access)

### Build

```bash
git clone <repository-url>
cd auto
cargo build --release
```

Binary: `target/release/auto`

### Enable Brave

```bash
# Windows
brave.exe --remote-debugging-port=9001

# macOS
/Applications/Brave\ Browser.app/Contents/MacOS/Brave\ Browser --remote-debugging-port=9001

# Linux
brave --remote-debugging-port=9001
```

## Basic Usage

### Task Syntax Examples

```bash
# Single task
cargo run cookiebot

# With URL parameter
cargo run pageview=www.reddit.com
cargo run pageview=url=https://example.com

# Multiple tasks (parallel within group)
cargo run cookiebot pageview=reddit.com

# Sequential groups (then = new group)
cargo run cookiebot pageview=reddit.com then cookiebot

# With parameters
cargo run 'twitteractivity,duration_ms=120000,scroll_count=12'

# .js extension auto-stripped
cargo run cookiebot.js pageview.js
```

### API Quick Examples (v0.0.3)

```rust
// Cookie Management - Check if logged in
let has_session = ctx.has_cookie("session_id", Some("example.com")).await?;

// Data File - Read configuration
let config: serde_json::Value = ctx.read_json_data("config/app.json")?;

// Network - Call REST API
let response = ctx.http_get("https://api.example.com/data").await?;

// DOM Inspection - Check element visibility
let rect = ctx.get_element_rect("#submit-btn").await?;
let visible = ctx.is_in_viewport("#hero-image").await?;

// Browser Management - Export complete state
let browser_data = ctx.export_browser("https://example.com").await?;
```

See [API Usage Guide](docs/API_USAGE_GUIDE.md) for complete examples.

### Common Patterns

```bash
# Pre-warm with cookiebot, then browse, then engage
cargo run cookiebot then pageview=reddit.com then twitteractivity

# Debug mode
RUST_LOG=debug cargo run pageview=example.com

# Custom config
cargo run -- --config path/to/config.toml cookiebot
```

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      CLI / CLI.rs                       │
│              (Task parsing, config loading)             │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│                   Orchestrator.rs                       │
│     (Task groups, parallel fan-out, timeouts, retry)  │
└─────────────────────────────────────────────────────────┘
                           │
┌─────────────────────────────────────────────────────────┐
│                   Session Manager                       │
│  (Browser lifecycle, health scoring 0-100, registry)   │
└─────────────────────────────────────────────────────────┘
                           │
┌──────────────┬──────────────┬───────────────────────────┐
│  Session 1   │  Session 2   │       Session N         │
│  (Brave)     │  (RoxyAPI)   │     (Chromium)          │
└──────────────┴──────────────┴───────────────────────────┘
```

**Framework Layers:**

| Layer | Purpose |
|-------|---------|
| `runtime` | Browser/session/page lifecycle |
| `capabilities` | Task-facing actions (mouse, keyboard, scroll, navigation) |
| `state` | Session-scoped handles (ClipboardState) |
| `internal` | Framework bridge layer |
| `utils` | Low-level implementation (mouse paths, timing, profiles) |

**Execution Model:**
- Task groups broadcast to **every active browser session**
- Parallel fan-out is the **default** execution model
- Partial failure allowed when **at least one session succeeds**

## Current Status

### Production Ready ✅

This project is actively maintained and used in production environments.

### v0.0.3 Features (Latest)

**26 New TaskContext APIs** with permission-based security:

| Category | APIs | Use Cases |
|----------|------|-----------|
| **Cookie Management** | 3 APIs | Export by domain, session cookies, existence check |
| **Session Management** | 3 APIs | localStorage export/import, validation |
| **Clipboard Management** | 3 APIs | Clear, check content, append text |
| **Data File Management** | 7 APIs | Read/write JSON, list, delete, metadata |
| **Network/HTTP** | 3 APIs | GET, POST JSON, download files |
| **DOM Inspection** | 5 APIs | Computed styles, element rect, scroll position, count, viewport check |
| **Browser Management** | 2 APIs | Complete state export/import |

**Accessibility Locator (Feature-Gated):**
- Internal accessibility-locator parser/resolver behind `--features accessibility-locator`
- Deterministic locator error taxonomy (`locator_parse_error`, `locator_not_found`, `locator_ambiguous`, `locator_scope_invalid`, `locator_unsupported`)
- Action-path integration for locator syntax (`click`, `hover`, `focus`, `double_click`, `right_click`, `middle_click`, `drag`)
- Pilot migration complete for `twitterfollow` with locator-first follow/following detection and safe CSS/JS fallback

**Security:** All APIs require explicit permissions via `TaskPolicy` - secure by default.

**Documentation:**
- [API Usage Guide](docs/API_USAGE_GUIDE.md) - Practical recipes and patterns
- [API Reference](docs/API_REFERENCE.md) - Complete API documentation
- Rustdoc examples in IDE - See examples when using auto-complete

| Feature | Status |
|---------|--------|
| Unified result types, error typing | ✅ |
| Per-task timeout, group timeout, cancellation | ✅ |
| Retry with exponential backoff + jitter | ✅ |
| Session lifecycle, health scoring | ✅ |
| Config loader (TOML + env vars) | ✅ |
| Task parser (Node.js parity verified) | ✅ |
| HTTP client with circuit breaker | ✅ |
| Metrics export (`run-summary.json`) | ✅ |
| Health monitoring (60s interval) | ✅ |
| Integration tests | ✅ |

**Quality Metrics:**
- **Extensive test coverage** across unit, integration, and doc test suites
- **Feature-on validation**: `cargo check --features accessibility-locator` passing
- **Pilot suite validation**: `cargo test --features accessibility-locator twitterfollow` passing
- **Parser parity:** All edge cases verified vs Node.js reference

### Browser Support

| Browser | Status | Notes |
|---------|--------|-------|
| Brave | ✅ Supported | `--remote-debugging-port=9001` |
| RoxyBrowser | ✅ Supported | API integration |
| Other Chromium | ⏸️ Future | Planned connectors |

## Configuration

### Quick Config

Create `config/default.toml`:

```toml
[browser]
max_discovery_retries = 3
discovery_retry_delay_ms = 5000

[[browser.profiles]]
name = "brave-local"
type = "brave"
ws_endpoint = ""  # Auto-discovery

[orchestrator]
max_global_concurrency = 20      # Max concurrent tasks (1-100)
task_timeout_ms = 600000         # 10 minutes
group_timeout_ms = 600000        # 10 minutes
max_retries = 2                  # Retry attempts
retry_delay_ms = 500             # Initial retry delay

# Twitter Activity (optional)
[twitter_activity]
feed_scan_duration_ms = 120000
feed_scroll_count = 12
```

### Environment Variables

```bash
# RoxyBrowser
export ROXYBROWSER_API_URL="https://api.roxybrowser.com/"
export ROXYBROWSER_API_KEY="your-api-key"

# Orchestrator
export MAX_GLOBAL_CONCURRENCY="10"
export TASK_TIMEOUT_MS="300000"

# Logging
export RUST_LOG="info,orchestrator=debug"
```

### Validation

| Setting | Valid Range | Default |
|---------|-------------|---------|
| `max_global_concurrency` | 1-100 | 20 |
| `task_timeout_ms` | >5000 | 600000 |
| `max_retries` | 0-10 | 2 |

## Troubleshooting

### No browsers discovered

```bash
# Solution: Enable remote debugging
brave.exe --remote-debugging-port=9001
```

### Custom browser port ranges

By default, the orchestrator scans these ports for browsers:
- **Brave**: ports 9001-9050
- **Chrome**: ports 9222-9230

You can customize these ranges via environment variables:

```bash
# Custom Brave port range
export BRAVE_PORT_START=9100
export BRAVE_PORT_END=9150

# Custom Chrome port range
export CHROME_PORT_START=9300
export CHROME_PORT_END=9350

# Run with custom ports
cargo run cookiebot
```

**Validation rules:**
- If `START > END`, values are automatically swapped
- Ports below 1024 are clamped to 1024 (reserved ports)
- Invalid values fall back to defaults

### Task validation errors

```bash
# Solution: Check task name and format
cargo run --help
# See: docs/TASKS/overview.md
```

### Circuit breaker open

```bash
# Solution: Check API endpoint
# - Verify RoxyBrowser is running
# - Check network connectivity
```

### Memory warnings

```bash
# Solution: Reduce concurrency
export MAX_GLOBAL_CONCURRENCY=10
```

### Rate limited (Twitter tasks)

```bash
# Solution: Reduce engagement limits in config
# Or wait before retrying
```

### Getting Help

1. `RUST_LOG=debug cargo run ...` - detailed logs
2. Check `run-summary.json` - failure breakdown
3. Review health logs - session issues
4. Open an issue with logs and config

## Documentation

| Document | Purpose |
|----------|---------|
| [docs/TASKS/overview.md](docs/TASKS/overview.md) | All tasks and how to run them |
| [docs/TASKS/cookiebot.md](docs/TASKS/cookiebot.md) | Cookie management task |
| [docs/TASKS/pageview.md](docs/TASKS/pageview.md) | Page browsing task |
| [docs/TASKS/twitteractivity.md](docs/TASKS/twitteractivity.md) | Twitter engagement task |
| [docs/API_REFERENCE.md](docs/API_REFERENCE.md) | Complete TaskContext API |
| [docs/TUTORIAL_BUILDING_FIRST_TASK.md](docs/TUTORIAL_BUILDING_FIRST_TASK.md) | Build your own tasks |
| [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) | PR guidelines |
| [Cargo docs](https://docs.rs) | `cargo doc --open` |

### Available Tasks

**Demo Tasks:**
- `cookiebot` - Cookie/consent management
- `demoqa` - Demo form automation
- `demo-keyboard` - Keyboard interaction demo
- `demo-mouse` - Mouse movement demo
- `pageview` - Human-like page browsing
- `task-example` - Example task template

**Twitter/X Tasks:**
- `twitteractivity` - Full feed engagement with smart decisions
- `twitterdive` - Thread diving and reading
- `twitterfollow` - Profile following
- `twitterintent` - Intent-based actions (like, follow)
- `twitterlike` - Like specific tweets
- `twitterquote` - Quote tweets with LLM
- `twitterreply` - Reply to tweets with LLM
- `twitterretweet` - Retweet specific tweets
- `twittertest` - Twitter automation smoke tests

## Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for:
- Development setup
- PR template
- Code style guidelines
- Task authoring guide

Quick checks before submitting:

```bash
cargo test
cargo clippy --all-targets --all-features
cargo fmt
```

## License

MIT License - see LICENSE file for details.

---

**Built with:** [Chromium Oxide](https://crates.io/crates/chromiumoxide) | [Tokio](https://tokio.rs/) | [Serde](https://serde.rs/)
