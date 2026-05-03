# Architecture Overview

> **Project:** rust-orchestrator  
> **Description:** High-performance multi-browser automation framework  
> **Last Updated:** 2026-05-01

## Table of Contents

1. [Core Components](#core-components)
2. [Data Flow](#data-flow)
3. [Architecture Patterns](#architecture-patterns)
4. [Error Handling Philosophy](#error-handling-philosophy)
5. [Testing Strategy](#testing-strategy)
6. [Technology Stack](#technology-stack)

---

## Core Components

### 1. Session Management

| Attribute | Value |
|-----------|-------|
| **Files** | `src/session/mod.rs`, `src/session/*.rs` |
| **Responsibility** | Browser session lifecycle, health tracking, circuit breaker pattern |
| **Key Types** | `Session`, `SessionState`, `CircuitBreaker`, `WorkerPermit` |
| **Pattern** | Actor model with atomic state management |

#### Session Lifecycle

```
┌─────────┐    ┌──────────┐    ┌─────────┐    ┌──────────┐
│ Created │───▶│ Starting │───▶│ Healthy │───▶│ Stopping │
└─────────┘    └──────────┘    └────┬────┘    └──────────┘
                                    │
                                    ▼
                              ┌───────────┐
                              │ Unhealthy │
                              └───────────┘
```

#### Circuit Breaker Pattern

The circuit breaker prevents cascading failures when browser sessions become unresponsive:

- **Failure Threshold:** Configurable count of consecutive failures
- **Timeout Duration:** Cooldown period before attempting recovery
- **Automatic Recovery:** Sessions transition back to `Healthy` after timeout

Circuit breaker is implemented in `src/session/mod.rs` as part of the Session struct.

---

### 2. Task Execution

| Attribute | Value |
|-----------|-------|
| **Files** | `src/task/mod.rs`, `src/task/*.rs`, `src/runtime/` |
| **Responsibility** | Task definition, validation, execution, result aggregation |
| **Key Types** | `Task`, `TaskContext`, `TaskResult`, `TaskDefinition` |
| **Pattern** | Trait-based plugin system with async execution |

#### Task Structure

```rust
pub trait Task: Send + Sync {
    async fn execute(&self, ctx: &TaskContext) -> Result<TaskResult>;
    fn name(&self) -> &str;
}
```

#### TaskContext API

High-level verbs for browser automation:

```rust
// Navigation
ctx.goto("https://example.com").await?;
ctx.reload().await?;
ctx.back().await?;

// Interaction
ctx.click("#button").await?;
ctx.r#type("#input", "text").await?;
ctx.hover("#menu").await?;

// Queries
let text = ctx.text("#element").await?;
let exists = ctx.exists("#element").await?;
```

---

### 3. Browser Automation

| Attribute | Value |
|-----------|-------|
| **Files** | `src/utils/browser.rs`, `src/utils/mouse.rs`, `src/utils/keyboard.rs` |
| **Responsibility** | CDP communication, input simulation, viewport management |
| **Key Types** | `Page`, `Viewport`, `Point`, `CursorMovementConfig` |
| **Pattern** | Async/await with timeout guards, human-like movement |

#### CDP (Chrome DevTools Protocol)

The framework uses `chromiumoxide` for CDP communication:

```
┌─────────────┐     CDP     ┌─────────────┐
│  Orchestrator│◄──────────►│Browser Process│
│   (Rust)     │ WebSocket   │  (Chrome)   │
└─────────────┘             └─────────────┘
```

#### Human-Like Movement

Mouse movement uses multiple trajectory algorithms:

| Path Style | Algorithm | Use Case |
|------------|-----------|----------|
| `Bezier` | Cubic Bezier with Gaussian control points | Natural curves |
| `Arc` | Quadratic arc with random direction | Sweeping motion |
| `Zigzag` | Perpendicular offsets | Searching behavior |
| `Overshoot` | 1.2x overshoot then correct | Human imperfection |
| `Stopped` | 3 equal stops | Hesitant movement |
| `Muscle` | PID-like with jitter | Realistic muscle simulation |

See `src/utils/mouse/trajectory.rs` for implementations.

---

### 4. Orchestration

| Attribute | Value |
|-----------|-------|
| **Files** | `src/orchestrator.rs`, `src/orchestrator/*.rs` |
| **Responsibility** | Multi-session coordination, result aggregation, error recovery |
| **Key Types** | `Orchestrator`, `RunSummary`, `SessionPool` |
| **Pattern** | Fan-out execution with semaphore control |

#### Execution Flow

```
┌────────────────┐
│   Task Queue   │
└───────┬────────┘
        │
        ▼
┌─────────────────────────────────────┐
│          Orchestrator               │
│  ┌───────────────────────────────┐  │
│  │       Session Pool            │  │
│  │  ┌─────┐ ┌─────┐ ┌─────┐    │  │
│  │  │ S1  │ │ S2  │ │ S3  │ ...│  │
│  │  └──┬──┘ └──┬──┘ └──┬──┘    │  │
│  │     │       │       │       │  │
│  │     ▼       ▼       ▼       │  │
│  │  ┌─────┐ ┌─────┐ ┌─────┐    │  │
│  │  │Task1│ │Task2│ │Task3│    │  │
│  │  └─────┘ └─────┘ └─────┘    │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
        │
        ▼
┌────────────────┐
│ Result Aggregator│
│  (Success/Error) │
└────────────────┘
```

---

## Data Flow

### Complete Request Lifecycle

```
┌─────────┐    ┌──────────┐    ┌───────────┐    ┌──────────┐
│   CLI   │───▶│   Args   │───▶│ Task Reg. │───▶│ Validate │
└─────────┘    └──────────┘    └───────────┘    └────┬─────┘
                                                     │
                                                     ▼
┌─────────┐    ┌──────────┐    ┌───────────┐    ┌──────────┐
│  Done   │◀───│ Aggregate│◀───│  Execute  │◀───│Orchestrator│
└─────────┘    └──────────┘    └───────────┘    └──────────┘
```

---

## Architecture Patterns

### 1. Circuit Breaker

Prevents cascading failures by marking unhealthy sessions:

```rust
// Pseudocode
if failure_count > threshold {
    state = Unhealthy;
    cooldown_until = now() + timeout;
}
```

**Files:** `src/session/mod.rs` (embedded in Session), `src/utils/twitter/twitteractivity_retry.rs` (Task-specific)

### 2. Actor Model

Sessions run as independent actors with message-passing:

- Each session has its own browser process
- Communication via CDP (WebSocket)
- State updates via atomic operations

### 3. Trait-Based Plugins

Tasks implement a common trait for extensibility:

```rust
#[async_trait]
pub trait Task: Send + Sync {
    async fn execute(&self, ctx: &TaskContext) -> Result<TaskResult>;
}
```

### 4. Timeout Guards

All async operations have timeouts to prevent hangs:

```rust
timeout(Duration::from_secs(30), operation).await
    .map_err(|_| anyhow!("Operation timed out"))?
```

---

## Error Handling Philosophy

### Principles

1. **Fail Fast:** Detect errors early, propagate immediately
2. **Context Matters:** Always include context in error messages
3. **Graceful Degradation:** Circuit breaker for transient failures
4. **Observability:** Structured logging for debugging

### Error Types

| Layer | Crate | Use Case |
|-------|-------|----------|
| Application | `anyhow` | Top-level error handling |
| Library | `thiserror` | Structured error types |
| Browser | Custom | CDP-specific errors |

### Example

```rust
// Application layer
pub async fn execute(&self) -> Result<TaskResult> {
    self.click("#button").await
        .with_context(|| format!("Failed to click button in task {}", self.name()))?
}

// Library layer
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Browser process exited: {0}")]
    ProcessExited(String),
    #[error("CDP command failed: {0}")]
    CdpFailure(String),
}
```

---

## Testing Strategy

### Test Pyramid

```
       ▲
      /│\
     / │ \      Integration Tests (tests/)
    /  │  \     ──────────────────────────
   /   │   \    Unit Tests (#[cfg(test)])
  /    │    \   ──────────────────────────
 /     │     \  Examples (examples/)
/      │      \ ──────────────────────────
────────────────
```

### Test Categories

| Type | Location | Coverage |
|------|----------|----------|
| **Unit** | `#[cfg(test)]` modules | Core logic, calculations |
| **Integration** | `tests/` directory | End-to-end workflows |
| **Examples** | `examples/` | Usage demonstrations |

### Running Tests

```bash
# Fast parallel execution
cargo nextest run --all-features --lib

# Standard test runner
cargo test --lib

# Integration tests only
cargo test --test '*'
```

---

## Technology Stack

### Core Dependencies

| Category | Crate | Purpose |
|----------|-------|---------|
| **Async Runtime** | `tokio` | Async I/O, task scheduling |
| **Browser Control** | `chromiumoxide` | CDP communication |
| **CLI** | `clap` | Argument parsing |
| **Serialization** | `serde` | Config, data exchange |
| **HTTP** | `reqwest` | API calls |
| **Error Handling** | `anyhow`, `thiserror` | Error management |
| **Observability** | `tracing`, `opentelemetry` | Logging, metrics |
| **Concurrency** | `parking_lot`, `dashmap` | Sync primitives |

### Why These Choices?

- **Tokio:** Industry-standard async runtime, excellent performance
- **chromiumoxide:** Modern CDP client, actively maintained
- **clap v4:** Derive macros for clean CLI definitions
- **anyhow:** Ergonomic error handling for applications
- **tracing:** Structured logging with OpenTelemetry integration

---

## Module Organization

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library exports
├── cli.rs               # Argument parsing
├── config.rs            # Configuration management
├── orchestrator.rs      # Core orchestration logic
│
├── internal/            # Internal utilities
│   ├── circuit_breaker.rs
│   ├── metrics.rs
│   └── mod.rs
│
├── session/             # Session management
│   ├── mod.rs           # Session, SessionState
│   ├── health.rs
│   └── worker.rs
│
├── task/                # Task implementations
│   ├── mod.rs           # Task trait, TaskContext
│   ├── twitter*.rs      # Twitter automation tasks
│   └── demo*.rs       # Demo/example tasks
│
├── runtime/             # Runtime utilities
│   ├── mod.rs
│   └── task_context.rs  # Browser automation API
│
└── utils/               # Shared utilities
    ├── browser.rs       # Browser process management
    ├── mouse.rs         # Mouse movement, clicking
    ├── mouse/
    │   └── trajectory.rs  # Path generation algorithms
    ├── keyboard.rs      # Keyboard input
    ├── scroll.rs        # Scrolling behavior
    ├── zoom.rs          # Zoom control
    └── twitter/         # Twitter automation (27 modularized files)
        ├── twitteractivity_*.rs    # Modular task components
```

---

## Future Architecture Considerations

### Planned Improvements

1. **Shutdown Coordination:** Coordinated shutdown with token propagation
2. **Click Pipeline:** Unified click abstraction with retry logic
3. **Browser Discovery:** Automatic browser detection with fallbacks
4. **Config Normalization:** Consolidated config validation

See `docs/IMPLEMENTATION_PLANS.md` for detailed plans.

---

## Contributing

See `docs/ONBOARDING.md` for:
- Development environment setup
- Testing guidelines
- Code review checklist

---

## License

MIT - See LICENSE file for details.
