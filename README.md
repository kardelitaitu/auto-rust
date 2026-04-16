# Rust Orchestrator

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Tokio](https://img.shields.io/badge/Tokio-000000?style=for-the-badge&logo=rust&logoColor=white)

A high-performance, multi-browser automation orchestrator built in Rust. Execute automated tasks across multiple browser sessions with advanced concurrency control, session management, and failure recovery.

## ✨ Features

- 🚀 **High Performance**: Built with Rust and Tokio for maximum throughput
- 🌐 **Multi-Browser Support**: Connect to multiple browser instances simultaneously
- 📊 **Advanced Orchestration**: Task grouping, timeouts, retries, and circuit breakers
- 🔄 **Session Management**: Automatic browser session lifecycle management
- 📈 **Metrics & Monitoring**: Built-in performance tracking and health checks
- 🛡️ **Production Ready**: Comprehensive error handling and graceful shutdown
- ⚙️ **Flexible Configuration**: TOML-based config with environment variable overrides

## 📦 Installation

### Prerequisites

- Rust 1.70+ ([Install Rust](https://rustup.rs/))
- A compatible browser (Brave, Chrome, etc.) or [RoxyBrowser](https://roxybrowser.com/)

### Build from Source

```bash
# Clone the repository
git clone <repository-url>
cd rust-orchestrator

# Build the project
cargo build --release

# Run tests (optional)
cargo test
```

The binary will be available at `target/release/rust-orchestrator`.

## 🚀 Quick Start

### Basic Usage

```bash
# Run a simple task
cargo run cookiebot

# Run multiple tasks sequentially
cargo run cookiebot then pageview

# Run tasks with parameters
cargo run cookiebot pageview=url=https://example.com

# Use a custom config file
cargo run -- --config path/to/config.toml cookiebot
```

### Configuration

Create a `config/default.toml` file:

```toml
[browser]
max_discovery_retries = 3
discovery_retry_delay_ms = 5000

[[browser.profiles]]
name = "brave-local"
type = "brave"
ws_endpoint = ""

[browser.roxybrowser]
enabled = true
api_url = "http://127.0.0.1:50000/"
api_key = "your-api-key-here"

[orchestrator]
max_global_concurrency = 20
task_timeout_ms = 600000
group_timeout_ms = 600000
worker_wait_timeout_ms = 10000
stuck_worker_threshold_ms = 120000
task_stagger_delay_ms = 2000
max_retries = 2
retry_delay_ms = 500
```

### Environment Variables

Override configuration with environment variables:

```bash
export ROXYBROWSER_API_URL="https://api.roxybrowser.com/"
export ROXYBROWSER_API_KEY="your-api-key"
export RUST_LOG="info,orchestrator=debug"
```

## 📋 Available Tasks

### CookieBot (`cookiebot`)
Manages browser cookies and consent dialogs.

```bash
cargo run cookiebot
```

### PageView (`pageview`)
Navigates to web pages and simulates user interaction.

```bash
# Navigate to a specific URL
cargo run pageview url=https://example.com

# Multiple pageviews
cargo run pageview=url=https://site1.com pageview=url=https://site2.com
```

## ⚙️ Configuration Reference

### Browser Configuration

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `max_discovery_retries` | u32 | 3 | Max retries for browser discovery |
| `discovery_retry_delay_ms` | u64 | 5000 | Delay between discovery retries |

### Browser Profiles

```toml
[[browser.profiles]]
name = "brave-local"      # Profile name
type = "brave"           # Browser type
ws_endpoint = ""         # WebSocket endpoint (leave empty for auto-discovery)
```

### RoxyBrowser Integration

```toml
[browser.roxybrowser]
enabled = true                          # Enable RoxyBrowser API
api_url = "http://127.0.0.1:50000/"    # API endpoint
api_key = "your-api-key"                # API authentication key
```

### Orchestrator Settings

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `max_global_concurrency` | usize | 20 | Maximum concurrent tasks |
| `task_timeout_ms` | u64 | 600_000 | Individual task timeout |
| `group_timeout_ms` | u64 | 600_000 | Task group timeout |
| `worker_wait_timeout_ms` | u64 | 10_000 | Worker acquisition timeout |
| `stuck_worker_threshold_ms` | u64 | 120_000 | Stuck worker detection |
| `task_stagger_delay_ms` | u64 | 2_000 | Delay between task starts |
| `max_retries` | u32 | 2 | Maximum retry attempts |
| `retry_delay_ms` | u64 | 500 | Delay between retries |

## 🔧 Development

### Project Structure

```
src/
├── main.rs          # Application entry point
├── cli.rs           # Command-line argument parsing
├── config.rs        # Configuration management
├── browser.rs       # Browser connection and management
├── orchestrator.rs  # Task orchestration logic
├── session.rs       # Session lifecycle management
├── result.rs        # Result types and error handling
├── api/             # HTTP client utilities
├── metrics.rs       # Performance monitoring
├── utils/           # Utility functions
└── task/            # Task implementations
    ├── mod.rs
    ├── cookiebot.rs
    └── pageview.rs
```

### Building for Development

```bash
# Debug build
cargo build

# Release build with optimizations
cargo build --release

# Run with debug logging
RUST_LOG=debug cargo run cookiebot
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_task_error_kind_classify

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Run linter
cargo clippy

# Format code
cargo fmt

# Generate documentation
cargo doc --open
```

## 📚 API Reference

### Core Types

#### `TaskResult`
Represents the outcome of a task execution.

```rust
pub enum TaskStatus {
    Success,
    Failed(String),
    Timeout,
}

pub struct TaskResult {
    pub status: TaskStatus,
    pub attempt: u32,
    pub max_retries: u32,
    pub last_error: Option<String>,
    pub duration_ms: u64,
}
```

#### `TaskErrorKind`
Categorizes different types of task failures.

```rust
pub enum TaskErrorKind {
    Timeout,      // Task exceeded timeout
    Validation,   // Input validation failed
    Navigation,   // Page navigation failed
    Session,      // Session management error
    Browser,      // Browser connection error
    Unknown,      // Other errors
}
```

### Key Functions

#### `parse_task_groups()`
Parse command-line arguments into executable task groups.

```rust
pub fn parse_task_groups(task_args: &[String]) -> Vec<Vec<TaskDefinition>>
```

#### `execute_task_with_retry()`
Execute a task with automatic retry logic.

```rust
pub async fn execute_task_with_retry(
    task_def: &TaskDefinition,
    session: &Session,
    config: &Config,
) -> TaskResult
```

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

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with [Chromium Oxide](https://crates.io/crates/chromiumoxide) for browser automation
- Inspired by the Node.js reference implementation
- Thanks to the Rust community for excellent crates and tooling

---

**Note**: This is a Rust port of an existing Node.js browser automation system, providing better performance and reliability for production workloads.</content>
<parameter name="filePath">C:\My Script\auto-rust\README.md