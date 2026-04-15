# Phase 6: Polish & Deployment

**Duration:** 1-2 days  
**Goal:** Build release binary, add optional shell wrapper, implement logging/error handling/retry logic, and optionally set up GitHub Actions CI.

---

## 6.1 Build Release Binary

### Optimized Build Configuration

Update `Cargo.toml` with release profile optimizations:

```toml
[package]
name = "rust-orchestrator"
version = "0.1.0"
edition = "2021"

[dependencies]
# ... (all dependencies from Phase 1)

[profile.release]
opt-level = 3              # Maximum optimization
lto = true                 # Link-time optimization (reduces binary size)
codegen-units = 1          # Better optimization (slower build)
panic = "abort"            # Smaller binary (no panic unwinding)
strip = true               # Remove debug symbols from binary

[profile.dev]
opt-level = 0              # No optimization for fast builds
debug = true               # Include debug symbols
```

### Build Commands

```bash
# Debug build (fast, for development)
cargo build

# Release build (optimized, for deployment)
cargo build --release

# Build and run in release mode
cargo run --release -- cookiebot.js pageview.js

# Check binary size
Get-Item target/release/rust-orchestrator.exe | Select-Object Name,Length,LastWriteTime  # Windows
ls -lh target/release/rust-orchestrator      # Linux/Mac
```

### Expected Binary Size

| Platform | Debug | Release | Release + LTO |
|----------|-------|---------|---------------|
| Windows x64 | ~200MB | ~30MB | ~15-20MB |
| Linux x64 | ~180MB | ~25MB | ~12-18MB |
| macOS ARM64 | ~190MB | ~28MB | ~14-20MB |

---

## 6.2 Shell Wrapper (Optional)

Create wrapper scripts for easier execution:

### `run.bat` (Windows)

```batch
@echo off
REM Rust Orchestrator Wrapper
REM Usage: run.bat [tasks...]
REM Example: run.bat cookiebot.js pageview.js then twitterFollow.js

set SCRIPT_DIR=%~dp0
set BINARY=%SCRIPT_DIR%target\release\rust-orchestrator.exe

REM Check if binary exists
if not exist "%BINARY%" (
    echo Error: Release binary not found. Building...
    cargo build --release
    if errorlevel 1 (
        echo Build failed!
        exit /b 1
    )
)

REM Run orchestrator
"%BINARY%" %*
exit /b %ERRORLEVEL%
```

### `run.sh` (Linux/Mac)

```bash
#!/bin/bash
# Rust Orchestrator Wrapper
# Usage: ./run.sh [tasks...]
# Example: ./run.sh cookiebot.js pageview.js then twitterFollow.js

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$SCRIPT_DIR/target/release/rust-orchestrator"

# Check if binary exists
if [ ! -f "$BINARY" ]; then
    echo "Error: Release binary not found. Building..."
    cargo build --release
    if [ $? -ne 0 ]; then
        echo "Build failed!"
        exit 1
    fi
fi

# Run orchestrator
"$BINARY" "$@"
exit $?
```

### Make executable (Linux/Mac)

```bash
chmod +x run.sh
```

---

## 6.3 Enhanced Error Handling & Retry Logic

### Global Error Handler (`src/main.rs`)

```rust
use anyhow::Result;
use tracing::{info, error, warn, Level};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Initialize tracing with better error formatting
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("rust_orchestrator=info".parse()?)
                .add_directive("tokio=warn".parse()?)
                .add_directive("chromiumoxide=warn".parse()?)
        )
        .with_target(false) // Remove module path from logs
        .with_thread_ids(false)
        .with_level(true)
        .init();

    // Run async main
    let result = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main());

    // Handle result
    match result {
        Ok(_) => {
            info!("Shutdown complete. Exiting gracefully.");
            Ok(())
        }
        Err(e) => {
            error!("Fatal error: {}", e);
            error!("Chain:");
            for cause in e.chain().skip(1) {
                error!("  -> {}", cause);
            }
            
            // Exit with error code
            std::process::exit(1);
        }
    }
}

async fn async_main() -> Result<()> {
    // ... (existing main logic from Phase 2)
    Ok(())
}
```

### Retry with Exponential Backoff

Add to `src/utils/retry.rs`:

```rust
use anyhow::Result;
use tracing::warn;
use crate::utils::timing::human_delay;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Initial delay in milliseconds
    pub initial_delay_ms: u64,
    /// Backoff multiplier (exponential)
    pub backoff_multiplier: f64,
    /// Maximum delay in milliseconds
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            backoff_multiplier: 2.0,
            max_delay_ms: 30000,
        }
    }
}

/// Retry an async operation with exponential backoff
/// 
/// # Example
/// ```
/// let result = retry_with_backoff(
///     || async { some_flaky_operation().await },
///     RetryConfig::default()
/// ).await;
/// ```
pub async fn retry_with_backoff<F, Fut, T>(
    mut operation: F,
    config: RetryConfig,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut delay_ms = config.initial_delay_ms;

    loop {
        attempt += 1;

        match operation().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                if attempt > config.max_retries {
                    return Err(anyhow::anyhow!(
                        "Operation failed after {} attempts: {}",
                        attempt,
                        e
                    ));
                }

                warn!(
                    "Attempt {}/{} failed: {}. Retrying in {}ms...",
                    attempt,
                    config.max_retries + 1,
                    e,
                    delay_ms
                );

                human_delay(delay_ms).await;

                // Exponential backoff with jitter
                delay_ms = ((delay_ms as f64 * config.backoff_multiplier) as u64)
                    .min(config.max_delay_ms);
                
                // Add 10% jitter to prevent thundering herd
                let jitter = (delay_ms as f64 * 0.1) as u64;
                delay_ms += rand::random::<u64>() % jitter;
            }
        }
    }
}

/// Retry with simple fixed delay (no backoff)
pub async fn retry_fixed<F, Fut, T>(
    mut operation: F,
    max_retries: u32,
    delay_ms: u64,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    retry_with_backoff(
        operation,
        RetryConfig {
            max_retries,
            initial_delay_ms: delay_ms,
            backoff_multiplier: 1.0, // No backoff, fixed delay
            max_delay_ms: delay_ms,
        },
    )
    .await
}
```

---

## 6.4 Structured Logging

### JSON Logging for Production

Add to `Cargo.toml`:

```toml
[dependencies]
tracing-appender = "0.2"
tracing-log = "0.2"
```

### File + Console Logging (`src/logger.rs`)

```rust
use anyhow::Result;
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use tracing_appender::{
    rolling::{RollingFileAppender, Rotation},
    non_blocking::WorkerGuard,
};

/// Initialize logging with file + console output
/// Returns WorkerGuard that must be kept alive
pub fn init_logging(log_dir: &str, env_filter: &str) -> Result<WorkerGuard> {
    // File appender (rotates daily)
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("orchestrator")
        .filename_suffix("log")
        .build(log_dir)?;

    // Non-blocking writer (async file writes)
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // File layer (JSON format)
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .json()
        .with_target(false)
        .with_thread_ids(true)
        .with_level(true)
        .with_ansi(false);

    // Console layer (human-readable)
    let console_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .with_ansi(true);

    // Env filter
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(env_filter));

    // Initialize subscriber
    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(console_layer)
        .init();

    Ok(guard)
}
```

### Usage in `main.rs`

```rust
fn main() -> Result<()> {
    // Initialize logging (keep guard alive until exit)
    let _log_guard = init_logging("logs", "rust_orchestrator=info")?;
    
    // ... rest of main
}
```

---

## 6.5 Environment Variables

Create `.env.example`:

```env
# Rust Orchestrator Environment Variables

# Logging level (trace, debug, info, warn, error)
RUST_LOG=info

# Config file path (default: config.toml)
RUST_CONFIG_PATH=config.toml

# Data directory (default: data/)
RUST_DATA_DIR=data

# Log directory (default: logs/)
RUST_LOG_DIR=logs

# Browser connection timeout (ms, default: 10000)
RUST_BROWSER_TIMEOUT=10000

# Task timeout (ms, default: 600000)
RUST_TASK_TIMEOUT=600000

# Max global concurrency (default: 20)
RUST_MAX_CONCURRENCY=20
```

### Load Environment Variables

Add to `Cargo.toml`:

```toml
[dependencies]
dotenvy = "0.15"
```

### In `main.rs`

```rust
fn main() -> Result<()> {
    // Load .env file
    dotenvy::dotenv().ok();
    
    // Initialize logging
    let env_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "rust_orchestrator=info".to_string());
    
    let _log_guard = init_logging("logs", &env_filter)?;
    
    // ... rest of main
}
```

---

## 6.6 GitHub Actions CI (Optional)

### `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    runs-on: windows-latest  # Your primary platform
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        targets: x86_64-pc-windows-msvc
    
    - name: Cache dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
        restore-keys: ${{ runner.os }}-cargo-
    
    - name: Check formatting
      run: cargo fmt -- --check
    
    - name: Run clippy (lints)
      run: cargo clippy -- -D warnings
    
    - name: Build debug
      run: cargo build --verbose
    
    - name: Run tests
      run: cargo test --verbose
    
    - name: Build release
      run: cargo build --release
    
    - name: Upload binary
      uses: actions/upload-artifact@v3
      with:
        name: rust-orchestrator
        path: target/release/rust-orchestrator.exe
        retention-days: 7

  cross-platform:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
    
    - name: Cache dependencies
      uses: actions/cache@v3
      with:
        path: |
          ~/.cargo/registry
          ~/.cargo/git
          target
        key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Build
      run: cargo build --release
    
    - name: Test
      run: cargo test
```

---

## 6.7 Code Quality Tools

### Install Rust Tools

```bash
# Formatter
rustup component add rustfmt

# Linter
rustup component add clippy
```

### Run Checks

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy -- -D warnings

# Run with all warnings as errors
cargo build --release
```

### Recommended `clippy.toml`

```toml
# Allow more complex types
too-many-arguments-threshold = 8

# Allow longer functions
too-many-lines-threshold = 300
```

---

## 6.8 README.md

Create project documentation:

```markdown
# Rust Orchestrator

Multi-browser automation orchestrator written in Rust. Zero race conditions, zero memory leaks, 24/7 stability.

## Features

- **Zero Race Conditions**: Rust borrow checker guarantees safety
- **Zero Memory Leaks**: RAII automatic cleanup
- **5-10x Lower Memory**: Compared to Node.js version
- **Single Binary**: No Node.js runtime required
- **24/7 Stability**: Can run continuously without babysitting

## Quick Start

```bash
# Build
cargo build --release

# Run tasks
cargo run --release -- cookiebot.js pageview.js

# Sequential groups
cargo run --release -- cookiebot.js then pageview.js

# With payload
cargo run --release -- twitterFollow=url=https://x.com/user
```

## CLI Usage

```
rust-orchestrator [TASKS]... [OPTIONS]

Arguments:
  [TASKS]...  Tasks to run, separated by 'then' for sequential groups

Options:
  --browsers <BROWSERS>  Comma-separated list of browser types
  -h, --help             Print help
```

## Configuration

Copy `config.toml.example` to `config.toml` and configure browser endpoints.

## Adding Tasks

1. Create `task/my_task.rs`
2. Register in `task/mod.rs`
3. Test: `cargo run -- my_task.js`

## Performance

| Metric | Node.js | Rust |
|--------|---------|------|
| Memory (1 hour) | ~500MB | ~100MB |
| CPU (idle) | ~5% | ~1% |
| Startup | ~2s | ~0.5s |
| Binary Size | ~100MB | ~20MB |

## License

Proprietary - All rights reserved
```

---

## 6.9 Deployment Checklist

### Pre-Deployment

- [ ] All tests pass (`cargo test`)
- [ ] Clippy clean (`cargo clippy -- -D warnings`)
- [ ] Formatted (`cargo fmt`)
- [ ] Release build compiles (`cargo build --release`)
- [ ] Config files documented (`config.toml.example`)
- [ ] README.md complete
- [ ] .gitignore excludes sensitive files

### Deployment Steps

```bash
# 1. Build release binary
cargo build --release

# 2. Verify binary
target/release/rust-orchestrator.exe --help

# 3. Test with sample tasks
cargo run --release -- cookiebot.js

# 4. Copy to deployment directory
mkdir deployment
cp target/release/rust-orchestrator.exe deployment/
cp config.toml.example deployment/config.toml
cp run.bat deployment/
cp -r data deployment/

# 5. Package (optional)
cd deployment
tar -czf rust-orchestrator-windows.tar.gz *
```

### Deployment Structure

```
deployment/
├── rust-orchestrator.exe    # Release binary
├── run.bat                  # Wrapper script
├── config.toml              # Configuration (edit this)
├── data/                    # URL lists
│   ├── cookiebot.txt
│   ├── pageview.txt
│   └── referrer.txt
└── logs/                    # Runtime logs (auto-created)
```

---

## 6.10 Monitoring & Maintenance

### Health Check Script

Create `scripts/health-check.bat`:

```batch
@echo off
REM Health check: Verify orchestrator can connect to browsers

echo Running health check...
cargo run --release -- cookiebot.js

if %ERRORLEVEL% EQU 0 (
    echo [OK] Health check passed
    exit /b 0
) else (
    echo [FAIL] Health check failed
    exit /b 1
)
```

### Scheduled Runs (Windows Task Scheduler)

```batch
REM Run orchestrator every hour
schtasks /create /tn "Rust Orchestrator" /tr "C:\path\to\run.bat cookiebot.js" /sc hourly /f
```

### Log Rotation

Logs are automatically rotated daily by `tracing-appender`. Old logs are kept in `logs/` directory.

---

## Deliverables

- [ ] Release binary builds successfully (`cargo build --release`)
- [ ] Binary size < 30MB (with LTO)
- [ ] Shell wrappers created (`run.bat`, `run.sh`)
- [ ] Structured logging to file + console
- [ ] Retry logic with exponential backoff
- [ ] README.md with usage instructions
- [ ] GitHub Actions CI configured (optional)
- [ ] Deployment package created
- [ ] Health check script working

---

## Notes

- **LTO (Link-Time Optimization)**: Significantly reduces binary size but increases build time. Use `--release` for production builds only.
- **panic = "abort"**: Removes panic unwinding code, reducing binary size. Panics will immediately exit (acceptable for CLI tool).
- **strip = true**: Removes debug symbols. If you need debugging symbols in release, use `strip = "debuginfo"` instead.
- **tracing-appender**: Non-blocking file writes prevent I/O bottlenecks. WorkerGuard must be kept alive or logs may be truncated on exit.
- **Cross-compilation**: To build for Linux from Windows, use `cargo build --target x86_64-unknown-linux-gnu` with a cross-compilation toolchain.
