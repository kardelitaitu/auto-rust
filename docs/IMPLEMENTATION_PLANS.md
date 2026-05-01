# Comprehensive Implementation Plans

> Generated: 2026-05-01
> Based on TODO.md Priorities - Sorted by Impact

---

## Priority 1: High Impact (Quick Wins)

### 1. Dependency Audit
**Goal:** Optimize dependency tree, reduce binary size, improve build times

#### Pre-Audit Analysis
Current dependencies: 39 total (see `Cargo.toml`)
- Browser automation: chromiumoxide, tokio
- CLI: clap
- Config: serde, serde_json, toml
- HTTP: reqwest, url
- Logging: tracing, tracing-subscriber
- Observability: opentelemetry ecosystem (5 crates)
- Async: futures, async-trait
- Random: rand, rand_distr
- Image: image, webp
- Concurrency: parking_lot, dashmap, sysinfo
- Utils: regex, rustc-hash, once_cell, lazy_static, log, do-over, urlencoding
- Dev: env_logger, tempfile, wiremock

#### Phase 1: Discovery (30 min)
```bash
# Generate dependency tree
cargo tree > docs/dependency_tree_full.txt

# Find duplicates
cargo tree -d > docs/dependency_duplicates.txt

# Check outdated (requires cargo-outdated)
cargo outdated > docs/dependency_outdated.txt 2>/dev/null || echo "cargo-outdated not installed"
```

#### Phase 2: Analysis (1-2 hours)
**Check each dependency:**

| Category | Crates to Review | Action |
|----------|-----------------|--------|
| **Potentially Unused** | `do-over`, `webp`, `urlencoding`, `do_over` | Check if referenced in code |
| **Duplicate Functionality** | `once_cell` + `lazy_static` | Migrate to `once_cell` only |
| **Heavy/Large** | `opentelemetry-*` (5 crates), `image` | Evaluate feature flags |
| **Outdated** | All 39 deps | Check for security patches |

**Investigation commands:**
```bash
# Check if crate is used
grep -r "do_over" src/ --include="*.rs" | head -5
grep -r "webp" src/ --include="*.rs" | head -5
grep -r "urlencoding" src/ --include="*.rs" | head -5

# Find lazy_static usage for migration
grep -r "lazy_static" src/ --include="*.rs" | wc -l
```

#### Phase 3: Optimization (1-2 hours)

**Action Items:**
1. **Remove unused:** Delete deps with 0 references
2. **Consolidate:** Replace `lazy_static` with `once_cell::sync::Lazy`
3. **Feature flags:** Enable only needed features (e.g., `image/webp`)
4. **Update security patches:** Focus on `reqwest`, `tokio`, `regex`

#### Phase 4: Documentation (30 min)
Create `docs/DEPENDENCY_AUDIT.md`:
```markdown
# Dependency Audit Report
Date: 2026-05-01

## Summary
- Total dependencies: 39 → X
- Binary size impact: X MB
- Build time impact: X seconds

## Removed
- `crate-name`: Reason

## Consolidated
- `lazy_static` → `once_cell::sync::Lazy`

## Updated (Security)
- `reqwest`: 0.12.X → 0.12.Y

## Recommendations
1. Consider removing opentelemetry if not using tracing
2. Evaluate if `enigo` is needed (only for native input)
```

#### Success Criteria
- [ ] At least 2 unused dependencies removed
- [ ] `lazy_static` migrated to `once_cell`
- [ ] Security patches applied to HTTP/net crates
- [ ] Document created with findings

---

### 2. Increase Bus Factor
**Goal:** Document architecture for team scalability

#### Phase 1: Architecture Document (2-3 hours)
Create `docs/ARCHITECTURE.md`:

```markdown
# Architecture Overview

## Core Components

### 1. Session Management
- **File:** `src/session/mod.rs`
- **Responsibility:** Browser session lifecycle, circuit breaker, health tracking
- **Key Types:** `Session`, `SessionState`, `CircuitBreaker`
- **Pattern:** Actor model with atomic state

### 2. Task Execution
- **File:** `src/task/mod.rs`, `src/runtime/`
- **Responsibility:** Task definition, validation, execution
- **Key Types:** `Task`, `TaskContext`, `TaskResult`
- **Pattern:** Trait-based plugin system

### 3. Browser Automation
- **File:** `src/utils/browser.rs`, `src/utils/mouse.rs`, `src/utils/keyboard.rs`
- **Responsibility:** CDP communication, input simulation
- **Key Types:** `Page`, `Viewport`, `Point`
- **Pattern:** Async/await with timeout guards

### 4. Orchestration
- **File:** `src/orchestrator.rs`
- **Responsibility:** Multi-session coordination, result aggregation
- **Key Types:** `Orchestrator`, `RunSummary`
- **Pattern:** Fan-out execution with semaphore control

## Data Flow
```
CLI → Task Registry → Validation → Orchestrator
                                      ↓
                              Session Pool (N browsers)
                                      ↓
                              Task Execution
                                      ↓
                              Result Aggregation
```

## Error Handling Philosophy
1. Use `anyhow` for application errors
2. Use `thiserror` for library errors
3. Always propagate with context
4. Circuit breaker for external failures

## Testing Strategy
- Unit tests in `#[cfg(test)]` modules
- Integration tests in `tests/`
- Examples in `examples/`
```

#### Phase 2: Onboarding Guide (1-2 hours)
Create `docs/ONBOARDING.md`:

```markdown
# Onboarding Guide

## Development Setup
1. Install Rust 1.75+
2. Install cargo-nextest: `cargo install cargo-nextest`
3. Clone repo: `git clone ...`
4. Run checks: `./check.ps1`

## Project Structure
```
src/
  main.rs          # CLI entry point
  lib.rs           # Library exports
  cli.rs           # Argument parsing
  config.rs        # Configuration
  orchestrator.rs  # Core orchestration
  session/         # Session management
  task/            # Task implementations
  utils/           # Utilities (mouse, keyboard, etc.)
```

## Testing Guidelines
- Run: `cargo test --lib`
- Run with nextest: `cargo nextest run --all-features --lib`
- CI check: `./check.ps1`

## Code Review Checklist
- [ ] Tests added for new logic
- [ ] Clippy warnings resolved
- [ ] Documentation updated
- [ ] No unwrap() in production code
```

#### Phase 3: Decision Log (30 min)
Create `docs/DECISION_LOG.md`:

```markdown
# Architecture Decision Log

## 2026-04: Session Circuit Breaker
**Context:** Need to handle browser crashes gracefully
**Decision:** Implement circuit breaker pattern with failure counting
**Consequences:** Sessions auto-mark unhealthy after threshold

## 2026-04: Nextest Migration
**Context:** Slow test execution with standard test runner
**Decision:** Adopt cargo-nextest for parallel execution
**Consequences:** 2-10x faster test runs

## 2026-03: Mouse Trajectory Module
**Context:** Need human-like cursor movement
**Decision:** Separate trajectory algorithms into `utils/mouse/trajectory.rs`
**Consequences:** Testable, swappable path generation
```

#### Phase 4: Module Documentation (1 hour)
Add rustdoc headers to 5+ core modules:
```rust
//! Session management for browser automation.
//!
//! Provides session lifecycle, health tracking, and circuit breaker
//! pattern for resilient browser automation.
//!
//! # Example
//! ```rust
//! let session = Session::new(config).await?;
//! session.execute_task(task).await?;
//! ```
```

#### Success Criteria
- [ ] ARCHITECTURE.md covers all major components
- [ ] ONBOARDING.md helps new devs get started in <30 min
- [ ] DECISION_LOG.md has 5+ entries
- [ ] 5+ modules have comprehensive rustdoc

---

## Priority 2: Medium Impact (Defined Effort)

### 3. Add Benchmark Suite
**Goal:** Measure and track performance of hot paths

#### Phase 1: Setup (30 min)
```bash
# Add criterion
cargo add --dev criterion

# Create benches directory
mkdir -p benches
touch benches/README.md
```

#### Phase 2: Trajectory Benchmarks (2-3 hours)
Create `benches/trajectory.rs`:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use auto::utils::mouse::trajectory::*;

fn bench_bezier_curve(c: &mut Criterion) {
    let start = Point::new(0.0, 0.0);
    let end = Point::new(1000.0, 1000.0);
    
    c.bench_function("bezier_curve_100_steps", |b| {
        b.iter(|| {
            generate_bezier_curve_with_config(
                black_box(&start),
                black_box(&end),
                black_box(50.0),
                black_box(Some(100))
            )
        })
    });
}

fn bench_all_curve_types(c: &mut Criterion) {
    let start = Point::new(0.0, 0.0);
    let end = Point::new(500.0, 500.0);
    
    let mut group = c.benchmark_group("curve_generation");
    
    group.bench_function("arc", |b| {
        b.iter(|| generate_arc_curve(black_box(&start), black_box(&end)))
    });
    
    group.bench_function("zigzag", |b| {
        b.iter(|| generate_zigzag_curve(black_box(&start), black_box(&end)))
    });
    
    group.bench_function("muscle", |b| {
        b.iter(|| generate_muscle_path(black_box(&start), black_box(&end)))
    });
    
    group.finish();
}

criterion_group!(trajectory, bench_bezier_curve, bench_all_curve_types);
criterion_main!(trajectory);
```

#### Phase 3: DOM Selector Benchmarks (2 hours)
Create `benches/selectors.rs`:
```rust
// Benchmark CSS selector parsing and resolution
// Requires mock DOM structure
```

#### Phase 4: CI Integration (1 hour)
Update `.github/workflows/ci.yml`:
```yaml
- name: Benchmark
  run: cargo bench -- --baseline main
  continue-on-error: true  # Don't fail on perf regression yet
```

#### Phase 5: Documentation (30 min)
Create `benches/README.md` with baseline metrics

#### Success Criteria
- [ ] 3+ benchmark files created
- [ ] Hot paths (trajectory, selectors) covered
- [ ] CI runs benchmarks (optional gate)
- [ ] Baseline metrics documented

---

### 4. Config Loading Normalization
**Goal:** Consolidate and improve configuration handling

#### Phase 1: Audit (1 hour)
Find all config usage:
```bash
grep -r "config::Config" src/ --include="*.rs"
grep -r "Config::new" src/ --include="*.rs"
grep -r "config\." src/main.rs src/cli.rs
```

#### Phase 2: Validation Module (3 hours)
Create `src/config/validation.rs`:
```rust
//! Configuration validation and error handling.

use anyhow::{Context, Result};

pub fn validate_config(config: &Config) -> Result<()> {
    validate_browser_paths(config)?;
    validate_timeouts(config)?;
    validate_concurrency(config)?;
    Ok(())
}

fn validate_browser_paths(config: &Config) -> Result<()> {
    if let Some(ref path) = config.browser.executable_path {
        if !std::path::Path::new(path).exists() {
            anyhow::bail!("Browser executable not found: {}", path);
        }
    }
    Ok(())
}

fn validate_timeouts(config: &Config) -> Result<()> {
    if config.navigation.timeout_ms == 0 {
        anyhow::bail!("Navigation timeout cannot be zero");
    }
    Ok(())
}
```

#### Phase 3: Error Messages (1 hour)
Add detailed error messages to all config failures

#### Phase 4: Example Configs (1 hour)
Create `examples/config/`:
```
examples/config/
  full.toml          # All options with comments
  minimal.toml       # Required options only
  production.toml    # Optimized for production
```

#### Phase 5: Unit Tests (2 hours)
Add tests for validation edge cases:
```rust
#[test]
fn test_validate_zero_timeout_fails() {
    let config = Config { timeout_ms: 0, .. };
    assert!(validate_config(&config).is_err());
}
```

#### Success Criteria
- [ ] All config loading consolidated
- [ ] Validation module with clear errors
- [ ] Example configs for common scenarios
- [ ] Unit tests for edge cases

---

### 5. Click-Learning Persistence
**Goal:** Persist learned click corrections for smarter automation

#### Phase 1: Design (2 hours)
Create `docs/CLICK_LEARNING_DESIGN.md`:
```markdown
## Persistence Format

### Option A: JSON File
`~/.auto-rust/click-corrections.json`
```json
{
  "version": 1,
  "corrections": [
    {
      "selector": "button[data-testid='follow']",
      "original": {"x": 100, "y": 200},
      "corrected": {"x": 105, "y": 195},
      "confidence": 0.95,
      "last_used": "2026-05-01T10:00:00Z",
      "ttl_days": 30
    }
  ]
}
```

### Option B: SQLite
Better for large datasets, queries

## Privacy
- Store locally only
- TTL for auto-cleanup
- User can disable/disable
```

#### Phase 2: Module Structure (4 hours)
Create `src/learning/`:
```rust
// src/learning/mod.rs
pub mod correction;
pub mod persistence;
pub mod privacy;

use crate::utils::mouse::Point;

pub struct ClickCorrection {
    pub selector: String,
    pub original: Point,
    pub corrected: Point,
    pub confidence: f64,
    pub last_used: DateTime<Utc>,
}

pub trait CorrectionStore {
    fn load(&self) -> Vec<ClickCorrection>;
    fn save(&mut self, corrections: &[ClickCorrection]) -> Result<()>;
    fn cleanup_expired(&mut self, ttl_days: u32) -> Result<usize>;
}
```

#### Phase 3: Integration (4 hours)
Integrate into `TaskContext` click methods:
```rust
pub async fn click_with_learning(&self, selector: &str) -> Result<()> {
    // Try learned correction first
    if let Some(correction) = self.learning.get_correction(selector) {
        if self.click_at(correction.x, correction.y).await.is_ok() {
            return Ok(());
        }
    }
    
    // Fall back to normal click + record result
    let result = self.click(selector).await;
    if result.is_err() {
        // Try correction and record if successful
    }
    result
}
```

#### Phase 4: TTL & Privacy (2 hours)
- Add cleanup on load
- Add opt-out setting
- Add clear data function

#### Success Criteria
- [ ] Click corrections persisted across runs
- [ ] TTL cleanup working
- [ ] Privacy controls (opt-out)
- [ ] Measurable improvement in click success rate

---

## Priority 3: Lower Impact (Larger Refactorings)

### 6. Runtime Shutdown Coordination
**Goal:** Clean, coordinated shutdown without zombie processes

#### Phase 1: Audit (2 hours)
Map all shutdown paths:
- Ctrl+C signal handler
- Task completion shutdown
- Error-triggered shutdown
- Timeout shutdown

#### Phase 2: Token Architecture (4 hours)
Implement shutdown token propagation:
```rust
// src/runtime/shutdown.rs
use tokio::sync::broadcast;

pub struct ShutdownCoordinator {
    tx: broadcast::Sender<ShutdownReason>,
}

pub enum ShutdownReason {
    Signal,      // Ctrl+C
    Complete,    // All tasks done
    Error,       // Fatal error
    Timeout,     // Global timeout
}
```

#### Phase 3: Browser Pool Coordination (4 hours)
Ensure all browser sessions close cleanly:
```rust
impl SessionPool {
    pub async fn shutdown(&mut self, timeout: Duration) {
        // Close all sessions with timeout
        // Kill orphaned processes
        // Cleanup temp files
    }
}
```

#### Phase 4: Integration Tests (2 hours)
Test shutdown scenarios:
```rust
#[tokio::test]
async fn test_graceful_shutdown_closes_browsers() {
    // Spawn orchestrator
    // Send shutdown signal
    // Verify no zombie processes
}
```

#### Success Criteria
- [ ] No zombie browser processes after shutdown
- [ ] All temp files cleaned up
- [ ] Timeout-enforced shutdown works
- [ ] Integration tests pass

---

### 7. TaskContext Click Pipeline
**Goal:** Unified, reliable click abstraction

#### Phase 1: Audit (2 hours)
Map current click paths:
- CDP mouse dispatch
- DOM event dispatch (fallback)
- Native input (enigo)

#### Phase 2: Design Pipeline (4 hours)
```rust
// src/runtime/click_pipeline.rs
pub struct ClickPipeline {
    strategies: Vec<Box<dyn ClickStrategy>>,
    verifier: Box<dyn ClickVerifier>,
}

pub trait ClickStrategy {
    async fn execute(&self, page: &Page, target: &Element) -> Result<()>;
}

pub struct CdpClick;
pub struct DomClick;
pub struct NativeClick; // For non-browser contexts
```

#### Phase 3: Retry Logic (2 hours)
```rust
pub async fn click_with_retry(
    &self,
    selector: &str,
    max_attempts: u32,
) -> Result<()> {
    for attempt in 0..max_attempts {
        match self.try_click(selector).await {
            Ok(_) => return Ok(()),
            Err(e) if attempt < max_attempts - 1 => {
                let delay = Duration::from_millis(100 * 2_u64.pow(attempt));
                tokio::time::sleep(delay).await;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

#### Phase 4: Verification (2 hours)
```rust
pub trait ClickVerifier {
    async fn verify(&self, page: &Page, before: ElementState) -> Result<()>;
}

pub struct StateChangeVerifier; // Check element changed
pub struct EventFiredVerifier;  // Check event was dispatched
```

#### Phase 5: Refactor TaskContext (4 hours)
Replace existing click methods with pipeline

#### Success Criteria
- [ ] Unified pipeline for all click types
- [ ] Retry with exponential backoff
- [ ] Verification before/after click
- [ ] Better error messages

---

### 8. CLI Parsing + Registry Self-Containment
**Goal:** Clean CLI architecture, self-contained task registry

#### Phase 1: Audit (1 hour)
Review `src/main.rs` and `src/cli.rs` structure

#### Phase 2: Create cli/ Module (4 hours)
```
src/cli/
  mod.rs       # CLI exports
  args.rs      # Clap argument definitions
  commands.rs  # Command handlers
  registry.rs  # Task registry (moved from main)
```

#### Phase 3: Self-Contained Registry (4 hours)
```rust
// src/cli/registry.rs
pub struct TaskRegistry {
    tasks: HashMap<String, TaskDef>,
}

impl TaskRegistry {
    pub fn discover() -> Self {
        // Auto-discover tasks from src/task/
        // No hardcoded list
    }
    
    pub fn get(&self, name: &str) -> Option<&TaskDef> {
        self.tasks.get(name)
    }
}
```

#### Phase 4: Main.rs Cleanup (2 hours)
```rust
// src/main.rs - should be <50 lines
fn main() {
    let args = cli::Args::parse();
    cli::run(args).await?;
}
```

#### Phase 5: Tests + Docs (2 hours)
- CLI integration tests
- Help documentation
- Examples

#### Success Criteria
- [ ] `main.rs` under 50 lines
- [ ] CLI module self-contained
- [ ] Task registry auto-discovers
- [ ] Integration tests for CLI

---

### 9. Browser Discovery Predictability
**Goal:** Reliable browser detection with fallbacks

#### Phase 1: Audit (2 hours)
Review current `chromiumoxide` usage and browser detection

#### Phase 2: Capability Detection (4 hours)
```rust
// src/utils/browser/discovery.rs
pub struct BrowserCapabilities {
    pub cdpp_enabled: bool,
    pub headless_supported: bool,
    pub version: semver::Version,
}

pub async fn detect_capabilities(executable: &Path) -> Result<BrowserCapabilities> {
    // Launch browser, test CDP connection
    // Check version via /json/version
    // Return capabilities
}
```

#### Phase 3: Fallback Chain (4 hours)
```rust
pub const BROWSER_FALLBACKS: &[&str] = &[
    "google-chrome",
    "chromium",
    "chromium-browser",
    "microsoft-edge",
    "brave",
];

pub async fn find_browser() -> Result<PathBuf> {
    for browser in BROWSER_FALLBACKS {
        if let Ok(path) = which::which(browser) {
            if detect_capabilities(&path).await.is_ok() {
                return Ok(path);
            }
        }
    }
    anyhow::bail!("No supported browser found")
}
```

#### Phase 4: Version Compatibility (2 hours)
```rust
pub const MIN_CHROME_VERSION: semver::Version = semver::Version::new(90, 0, 0);

pub fn check_version(capabilities: &BrowserCapabilities) -> Result<()> {
    if capabilities.version < MIN_CHROME_VERSION {
        anyhow::bail!("Chrome {} is too old, need >= {}", 
            capabilities.version, MIN_CHROME_VERSION);
    }
    Ok(())
}
```

#### Phase 5: Documentation + Tests (2 hours)
- Document supported browsers
- Unit tests for discovery
- Integration tests with mock browsers

#### Success Criteria
- [ ] Chrome → Chromium → Edge fallback works
- [ ] Version compatibility checks
- [ ] Clear error messages for unsupported browsers
- [ ] Documentation of requirements

---

## Summary Table

| # | Item | Est. Time | Priority | Blockers |
|---|------|-----------|----------|----------|
| 1 | Dependency Audit | 2-4 hrs | P1 | None |
| 2 | Bus Factor | 4-6 hrs | P1 | None |
| 3 | Benchmarks | 1-2 days | P2 | None |
| 4 | Config Normalization | 1 day | P2 | None |
| 5 | Click-Learning | 2-3 days | P2 | Design decision |
| 6 | Shutdown Coordination | 3-4 days | P3 | Architecture design |
| 7 | Click Pipeline | 3-5 days | P3 | Refactor scope |
| 8 | CLI Self-Containment | 2-3 days | P3 | Registry design |
| 9 | Browser Discovery | 4-7 days | P3 | Testing complexity |

---

## Recommended Start Order

1. **Dependency Audit** (Quick win, immediate benefit)
2. **Bus Factor** (Long-term team benefit)
3. **Config Normalization** (Foundation for other work)
4. **Benchmarks** (Measure before optimizing)
5. Remaining items based on product priorities
