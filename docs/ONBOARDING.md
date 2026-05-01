# Onboarding Guide

> **Welcome to rust-orchestrator!**  
> This guide will help you get started with development in ~30 minutes.

## Table of Contents

1. [Development Setup](#development-setup)
2. [Project Structure](#project-structure)
3. [Testing Guidelines](#testing-guidelines)
4. [Code Review Checklist](#code-review-checklist)
5. [Common Tasks](#common-tasks)
6. [Troubleshooting](#troubleshooting)

---

## Development Setup

### Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.75+ | Language compiler |
| Git | 2.30+ | Version control |
| Chrome/Chromium | 90+ | Browser for testing |
| PowerShell/Bash | Any | Running scripts |

### 1. Install Rust

```bash
# Via rustup (recommended)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version  # Should show 1.75 or higher
cargo --version
```

### 2. Clone Repository

```bash
git clone https://github.com/your-org/rust-orchestrator.git
cd rust-orchestrator
```

### 3. Install cargo-nextest (Required)

```bash
cargo install --locked cargo-nextest
```

### 4. Verify Setup

```bash
# Run CI checks (should all pass)
./check.ps1        # Windows PowerShell
./check.sh         # Unix (if available)
```

Expected output:
```
1. Build check (cargo check)
PASS
2. Format check (cargo fmt --all -- --check)
PASS
3. Clippy check (cargo clippy --all-targets --all-features -- -D warnings)
PASS
4. Nextest check (cargo nextest run --all-features --lib)
PASS
CI CHECKER REPORT: All checks passed!
```

---

## Project Structure

### Directory Layout

```
rust-orchestrator/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library exports
│   ├── cli.rs               # Argument parsing with clap
│   ├── config.rs            # Configuration management
│   ├── orchestrator.rs      # Core orchestration logic
│   │
│   ├── internal/            # Internal utilities
│   │   ├── circuit_breaker.rs
│   │   └── metrics.rs
│   │
│   ├── session/             # Browser session management
│   │   ├── mod.rs
│   │   └── worker.rs
│   │
│   ├── task/                # Task implementations
│   │   ├── mod.rs           # Task trait, TaskContext
│   │   ├── twitter*.rs      # Twitter automation
│   │   └── demo*.rs         # Demo tasks
│   │
│   ├── runtime/             # Runtime utilities
│   │   └── task_context.rs  # Browser automation API
│   │
│   └── utils/               # Shared utilities
│       ├── browser.rs
│       ├── mouse.rs
│       ├── keyboard.rs
│       └── twitter/         # Twitter helpers
│
├── tests/                   # Integration tests
├── examples/                # Usage examples
├── docs/                    # Documentation
│   ├── ARCHITECTURE.md
│   ├── API_REFERENCE.md
│   └── ONBOARDING.md        # This file
│
├── Cargo.toml              # Dependencies
├── Cargo.lock              # Locked versions
├── check.ps1               # CI script (Windows)
└── .config/
    └── nextest.toml        # Test configuration
```

### Key Files to Know

| File | Purpose |
|------|---------|
| `src/task/mod.rs` | Task trait definition, TaskContext API |
| `src/session/mod.rs` | Session lifecycle, health tracking |
| `src/orchestrator.rs` | Multi-session coordination |
| `src/utils/mouse.rs` | Human-like mouse movement |
| `check.ps1` | Run all CI checks locally |

---

## Testing Guidelines

### Test Philosophy

> **"Test behavior, not implementation"**

Focus on:
- Public API contracts
- Edge cases and error conditions
- Integration workflows
- Performance regressions (benchmarks)

Avoid:
- Testing private functions directly
- Brittle tests tied to implementation details
- Tests that require real browsers (use mocks)

### Test Types

#### 1. Unit Tests

Location: Inline in `#[cfg(test)]` modules

```rust
// src/utils/mouse/trajectory.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bezier_point_at_start() {
        let p0 = Point::new(0.0, 0.0);
        let result = bezier_point(p0, p1, p2, p3, 0.0);
        assert!((result.x - p0.x).abs() < 0.001);
    }
}
```

Run unit tests:
```bash
cargo nextest run --all-features --lib
```

#### 2. Integration Tests

Location: `tests/` directory

```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_full_workflow() {
    // Setup
    let orchestrator = Orchestrator::new(config);
    
    // Execute
    let result = orchestrator.run(task).await;
    
    // Assert
    assert!(result.is_ok());
}
```

Run integration tests:
```bash
cargo test --test '*'
```

#### 3. Documentation Tests

```rust
/// # Example
/// ```
/// let point = Point::new(10.0, 20.0);
/// assert_eq!(point.x, 10.0);
/// ```
pub struct Point { ... }
```

Run doc tests:
```bash
cargo test --doc
```

### Adding Tests

**New Feature Checklist:**
- [ ] Unit tests for core logic
- [ ] Edge case tests (empty input, max values, errors)
- [ ] Integration test if applicable
- [ ] Doc tests for public API examples

**Example: Adding Tests for a New Task**

```rust
// In your task file
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_execution_success() {
        let ctx = TaskContext::mock();
        let task = MyTask::new();
        
        let result = task.execute(&ctx).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, TaskStatus::Success);
    }

    #[tokio::test]
    async fn test_task_handles_missing_element() {
        let ctx = TaskContext::mock_with_missing_element();
        let task = MyTask::new();
        
        let result = task.execute(&ctx).await;
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
```

---

## Code Review Checklist

Before submitting a PR, ensure:

### Code Quality
- [ ] **No `unwrap()` in production code** (tests OK)
- [ ] All `clippy` warnings resolved
- [ ] Code formatted with `cargo fmt`
- [ ] No compiler warnings

### Testing
- [ ] New functionality has tests
- [ ] All tests pass (`./check.ps1`)
- [ ] Edge cases covered
- [ ] No flaky tests (run 3x to verify)

### Documentation
- [ ] Public APIs have rustdoc
- [ ] Complex logic has inline comments
- [ ] README updated if user-facing changes
- [ ] Architecture doc updated if design changes

### Performance
- [ ] No unnecessary allocations
- [ ] Async functions don't block
- [ ] Consider benchmark if hot path

### Security
- [ ] No hardcoded secrets
- [ ] Input validation for external data
- [ ] Path traversal protection

### Git
- [ ] Descriptive commit message (see format below)
- [ ] Commits are atomic and focused
- [ ] Branch is up-to-date with main

### Commit Message Format

```
type: description (reason/impact)

Examples:
feat: add twitterquote task with LLM integration
fix: handle rate limit in twitterfollow retry logic
docs: rewrite README with TOC (843 -> 350 lines)
test: add edge case tests for mouse trajectory
refactor: consolidate error handling in session module

Types: feat, fix, docs, test, refactor, perf, chore
```

**DON'T:**
- "update"
- "fix"
- "changes"
- "wip"

---

## Common Tasks

### Adding a New Task

1. Create file in `src/task/yourtask.rs`
2. Implement `Task` trait
3. Register in task registry
4. Add tests
5. Update documentation

Example:
```rust
// src/task/my_task.rs
use crate::task::{Task, TaskContext, TaskResult};

pub struct MyTask {
    url: String,
}

#[async_trait]
impl Task for MyTask {
    async fn execute(&self, ctx: &TaskContext) -> Result<TaskResult> {
        ctx.goto(&self.url).await?;
        ctx.click("#button").await?;
        Ok(TaskResult::success())
    }

    fn name(&self) -> &str {
        "my_task"
    }
}
```

### Debugging a Failing Test

```bash
# Run specific test with output
cargo nextest run --all-features --lib test_name -- --nocapture

# Run with logging
RUST_LOG=debug cargo nextest run --all-features --lib test_name

# Run with backtrace on panic
RUST_BACKTRACE=1 cargo nextest run --all-features --lib test_name
```

### Profiling Performance

```bash
# Build release binary
cargo build --release

# Run with timing
time ./target/release/auto --task my_task

# For detailed profiling, use flamegraph
cargo install flamegraph
cargo flamegraph --bin auto -- --task my_task
```

---

## Troubleshooting

### Common Issues

#### "cargo-nextest not found"

```bash
cargo install --locked cargo-nextest
```

#### "Chrome not found" in tests

Ensure Chrome/Chromium is installed and in PATH:
```bash
# Windows
where chrome

# macOS/Linux
which google-chrome
which chromium
```

#### Clippy warnings about deprecated functions

Update dependencies:
```bash
cargo update
```

If warnings persist, check if we need to update our usage.

#### Build fails with linking errors

On Windows, ensure Visual Studio Build Tools are installed.
On Linux, install `build-essential`.

```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# macOS
xcode-select --install
```

#### Tests timeout

Check if browser processes are stuck:
```bash
# Find Chrome processes
ps aux | grep chrome

# Kill stuck processes (use with caution)
pkill -9 chrome
```

### Getting Help

1. Check `docs/ARCHITECTURE.md` for design context
2. Review `docs/API_REFERENCE.md` for API details
3. Search existing tests for examples
4. Ask in Slack/Discord #dev channel

---

## Quick Reference

### Commands

| Command | Purpose |
|---------|---------|
| `./check.ps1` | Run all CI checks |
| `cargo nextest run --all-features --lib` | Run unit tests |
| `cargo fmt --all` | Format all code |
| `cargo clippy --all-targets --all-features -- -D warnings` | Linting |
| `cargo doc --all-features --no-deps` | Generate docs |
| `cargo build --release` | Release build |

### File Patterns

| Pattern | Meaning |
|---------|---------|
| `src/task/*.rs` | Task implementations |
| `src/utils/*.rs` | Shared utilities |
| `tests/*_test.rs` | Integration tests |
| `examples/*.rs` | Usage examples |

---

## Next Steps

After completing this guide:

1. **Read** `docs/ARCHITECTURE.md` for system design
2. **Explore** `examples/` for usage patterns
3. **Try** implementing a simple task
4. **Review** `docs/API_REFERENCE.md` for detailed API docs

Welcome to the team! 🚀
