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
- [DSL Task System](#dsl-task-system-v010)
- [Task Registry](#task-registry)
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

> **Note:** Pass CLI flags after `--`, for example `cargo run -- --list-tasks` or `cargo run -- --dry-run cookiebot`.

## DSL Task System (v0.1.0)

Define automation tasks declaratively in YAML/TOML format. No Rust coding required for simple to moderately complex workflows.

### Quick DSL Example

Create a file `tasks/my_login.task`:

```yaml
name: my_login
description: "Login to example.com"
policy: default

parameters:
  username:
    type: string
    required: true
  password:
    type: string
    required: true

actions:
  - action: navigate
    url: "https://example.com/login"
  
  - action: wait_for
    selector: "#username"
    timeout_ms: 5000
  
  - action: type
    selector: "#username"
    text: "{{username}}"
  
  - action: type
    selector: "#password"
    text: "{{password}}"
  
  - action: click
    selector: "#login-button"
  
  - action: wait
    duration_ms: 2000
  
  - action: screenshot
    path: "/tmp/logged_in.png"
```

Run it:

```bash
cargo run -- my_login username=admin password=secret123
```

### Available DSL Actions

| Action | Purpose | Example |
|--------|---------|---------|
| `navigate` | Go to URL | `url: "https://example.com"` |
| `click` | Click element | `selector: "#button"` |
| `type` | Type text | `selector: "#input", text: "hello"` |
| `wait` | Pause | `duration_ms: 1000` |
| `wait_for` | Wait for element | `selector: "#result", timeout_ms: 5000` |
| `scroll_to` | Scroll to element | `selector: "#footer"` |
| `extract` | Get element text | `selector: "#title", variable: "title_text"` |
| `clear` | Clear input | `selector: "#search"` |
| `hover` | Hover element | `selector: "#menu"` |
| `select` | Select dropdown | `selector: "#country", value: "USA"` |
| `right_click` | Right-click | `selector: "#item"` |
| `double_click` | Double-click | `selector: "#file"` |
| `screenshot` | Capture screen | `path: "/tmp/page.png"` |
| `log` | Log message | `message: "Done!", level: info` |
| `if` | Conditional | `condition: ..., then: [...]` |
| `loop` | Repeat actions | `count: 5, actions: [...]` |
| `call` | Call other task | `task: "login", parameters: {...}` |
| `foreach` | Iterate collection | `variable: "item", collection: ...` |
| `while` | Loop while condition | `condition: ..., actions: [...]` |
| `retry` | Retry with backoff | `actions: [...], max_attempts: 3` |
| `try` | Error handling | `try_actions: [...], catch_actions: [...]` |

### Control Flow Examples

#### If/Then/Else
```yaml
- if:
    condition:
      element_visible: "#premium-content"
    then:
      - log:
          message: "Premium user detected"
          level: info
      - click:
          selector: "#premium-button"
    else:
      - log:
          message: "Standard user"
          level: info
```

#### Foreach (Iterate over collection)
```yaml
# Array collection
- foreach:
    variable: "product"
    collection:
      type: array
      values: ["iPhone", "iPad", "MacBook"]
    actions:
      - log:
          message: "Checking {{product}}..."
          level: info

# Range collection
- foreach:
    variable: "index"
    collection:
      type: range
      start: 1
      end: 6
    actions:
      - log:
          message: "Processing item {{index}}"
          level: info

# DOM elements collection
- foreach:
    variable: "item"
    collection:
      type: elements
      selector: ".product-item"
    max_iterations: 10
    actions:
      - click:
          selector: "{{item}}"
```

#### While (Condition-based loop)
```yaml
# Wait for loading to complete
- while:
    condition:
      element_visible: ".loading-spinner"
    max_iterations: 30
    actions:
      - wait:
          duration_ms: 500
      - log:
          message: "Still loading..."
          level: debug

# Wait for element to appear
- while:
    condition:
      element_not_exists: "#content"
    max_iterations: 20
    actions:
      - wait:
          duration_ms: 250
```

#### Retry (Automatic retry with backoff)
```yaml
- retry:
    actions:
      - click:
          selector: "#flaky-button"
    max_attempts: 5
    initial_delay_ms: 1000
    max_delay_ms: 30000
    backoff_multiplier: 2.0
    jitter: true
    retry_on: ["timeout", "network"]
```

#### Try/Catch/Finally
```yaml
- try:
    try_actions:
      - click:
          selector: "#primary-button"
    catch_actions:
      - log:
          message: "Primary failed, using fallback"
          level: warn
      - click:
          selector: "#fallback-button"
    error_variable: "error_message"
    finally_actions:
      - log:
          message: "Attempt complete"
          level: info
```

### Task Composition (Call Action)

Tasks can call other tasks as subroutines, enabling modular, reusable automation workflows.

#### Call Action Reference

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `task` | string | Yes | Name of the task to call |
| `parameters` | object | No | Key-value pairs passed to the called task |

#### Variable Inheritance (Parent → Child)

When a task calls another, **parent variables are automatically available** to the child:

```yaml
# Parent task: fetch_and_process.task
name: fetch_and_process
actions:
  - action: navigate
    url: "https://api.example.com"
  
  - action: extract
    selector: "#api-key"
    variable: api_key    # Set in parent
  
  - action: call
    task: fetch_data
    parameters:
      endpoint: "/users"  # Explicit parameter
      # api_key is inherited automatically!
```

```yaml
# Child task: fetch_data.task
name: fetch_data
parameters:
  endpoint: { type: string, required: true }
actions:
  - action: log
    message: "Using API key: {{api_key}}"  # Inherited from parent
    level: info
  - action: log
    message: "Fetching from {{endpoint}}"   # Explicit parameter
    level: info
```

#### Return Values (Child → Parent)

Child tasks can **return data** to parents via `extract` actions. Variables set in the child are merged back into the parent:

```yaml
# Child task: get_user_info.task
name: get_user_info
parameters:
  user_id: { type: string, required: true }
actions:
  - action: navigate
    url: "https://app.example.com/users/{{user_id}}"
  - action: extract
    selector: "#user-name"
    variable: user_name    # Will be returned to parent
  - action: extract
    selector: "#user-email"
    variable: user_email   # Will be returned to parent
```

```yaml
# Parent task: notify_users.task
name: notify_users
actions:
  - action: call
    task: get_user_info
    parameters:
      user_id: "12345"
  
  - action: log
    message: "Found user: {{user_name}} ({{user_email}})"  # From child!
    level: info
```

#### Recursion Limits

Task composition supports up to **10 levels of nesting** (configurable). Exceeding this limit causes execution to fail with a clear error:

```
Maximum call depth (10) exceeded when calling task 'recursive_task'
```

#### Complete Example: Login + Dashboard Workflow

Create reusable task `tasks/login.task`:

```yaml
name: login
parameters:
  url: { type: url, required: true }
  username: { type: string, required: true }
  password: { type: string, required: true }
actions:
  - action: navigate
    url: "{{url}}"
  - action: type
    selector: "#user"
    text: "{{username}}"
  - action: type
    selector: "#pass"
    text: "{{password}}"
  - action: click
    selector: "#submit"
  - action: wait_for
    selector: "#dashboard"
    timeout_ms: 5000
  - action: extract
    selector: "#user-id"
    variable: logged_in_user_id
```

Create workflow `tasks/full_workflow.task`:

```yaml
name: full_workflow
include:
  - path: login.task
actions:
  - action: call
    task: login
    parameters:
      url: "https://app.example.com"
      username: "admin"
      password: "{{admin_pass}}"
  
  - action: log
    message: "Logged in as user {{logged_in_user_id}}"  # From login task
    level: info
  
  - action: navigate
    url: "https://app.example.com/dashboard"
  
  - action: screenshot
    path: "/tmp/dashboard.png"
```

### Configuration

Enable task discovery in `config.toml`:

```toml
[task_discovery]
enabled = true
roots = ["./tasks", "~/.config/auto/tasks"]
extensions = ["task", "yaml", "yml"]
```

### CLI Flags

```bash
# List all tasks
cargo run -- --list-tasks

# Validate tasks
cargo run -- --validate-tasks

# Hot reload (auto-reload on file changes)
cargo run -- --watch my_task

# Dry run
cargo run -- --dry-run my_task
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

### Task Registry

List all built-in tasks and their policy/source metadata:

```bash
cargo run -- --list-tasks
```

Example output:

```text
Available Tasks:
================

  cookiebot             BuiltInRust                     policy=cookiebot
  pageview              BuiltInRust                     policy=pageview
  twitteractivity       BuiltInRust                     policy=twitteractivity

Total: 15 tasks
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
