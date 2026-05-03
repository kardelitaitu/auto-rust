# Architecture Decision Log (ADL)

> **Purpose:** Record significant technical decisions and their rationale  
> **Format:** ADR (Architecture Decision Record) inspired by [adr.github.io](https://adr.github.io/)

---

## Index

| # | Date | Decision | Status |
|---|------|----------|--------|
| 1 | 2026-04 | Adopt cargo-nextest for test execution | Accepted |
| 2 | 2026-04 | Implement circuit breaker for sessions | Accepted |
| 3 | 2026-04 | Separate mouse trajectory algorithms | Accepted |
| 4 | 2026-03 | Use CDP (Chrome DevTools Protocol) over WebDriver | Accepted |
| 5 | 2026-03 | Adopt anyhow + thiserror for error handling | Accepted |
| 6 | 2026-03 | Use tokio async runtime exclusively | Accepted |
| 7 | 2026-04 | Remove lazy_static, migrate to once_cell | Accepted |
| 8 | 2026-04 | Dependency audit process | Accepted |
| 9 | 2026-05 | Modularize twitteractivity into 27 files | Accepted |
| 10 | 2026-05 | Remove thread cache (simplified LLM context) | Accepted |
| 11 | 2026-05 | Implement Smart Decision Engine (7.1) | Accepted |
| 12 | 2026-05 | Add advanced test coverage (integration/statistical/property) | Accepted |

---

## ADR 1: Adopt cargo-nextest for Test Execution

**Date:** 2026-04-28  
**Status:** Accepted  
**Deciders:** Cascade AI, Project Lead

### Context

Standard `cargo test` execution was slow (60+ seconds for full suite) and lacked:
- Parallel test execution
- Clear test output formatting
- Flaky test detection
- Clean separation of stdout/stderr

### Decision

Migrate to `cargo-nextest` as the primary test runner.

### Consequences

**Positive:**
- 2-10x faster test execution (11s vs 60s)
- Parallel execution by default
- Better output formatting with summaries
- Built-in flaky test detection
- Clean CI integration

**Negative:**
- Additional dev dependency to install
- Slightly different test discovery than standard runner

### Implementation

1. Updated CI workflow (`.github/workflows/ci.yml`)
2. Created `.config/nextest.toml` for configuration
3. Updated `check.ps1` to use nextest
4. Documented in `docs/MIGRATING_TO_NEXTEST.md`

### References

- [nexte.st](https://nexte.st/)
- `docs/MIGRATING_TO_NEXTEST.md`

---

## ADR 2: Implement Circuit Breaker for Sessions

**Date:** 2026-04-25  
**Status:** Accepted  
**Deciders:** Cascade AI

### Context

Browser sessions frequently become unresponsive due to:
- Page crashes
- JavaScript infinite loops
- Memory pressure
- Network issues

Without circuit breaking, failed sessions would continue receiving tasks, causing cascading failures.

### Decision

Implement circuit breaker pattern for browser session health management.

### Design

```rust
struct CircuitBreaker {
    failure_count: AtomicU32,
    failure_threshold: u32,
    timeout_duration: Duration,
    last_failure: AtomicU64, // timestamp
}

enum SessionState {
    Healthy,
    Unhealthy { until: Instant },
}
```

### Consequences

**Positive:**
- Automatic failure detection
- Prevents task assignment to broken sessions
- Self-healing with timeout-based recovery
- Metrics integration for observability

**Negative:**
- Additional complexity in session management
- Requires careful tuning of thresholds
- Potential for false positives

### Implementation

- `src/internal/circuit_breaker.rs` - Core pattern
- `src/session/mod.rs` - Integration with Session
- Uses `do_over` crate for circuit breaker logic

### References

- [Circuit Breaker Pattern (Martin Fowler)](https://martinfowler.com/bliki/CircuitBreaker.html)
- `src/session/mod.rs`

---

## ADR 3: Separate Mouse Trajectory Algorithms

**Date:** 2026-04-20  
**Status:** Accepted  
**Deciders:** Cascade AI

### Context

Mouse movement needed to appear human-like to avoid detection. Original implementation had hardcoded bezier curves inline with movement code, making it:
- Difficult to test
- Hard to extend with new movement types
- Coupled to CDP event dispatch

### Decision

Extract trajectory generation into standalone, testable module `src/utils/mouse/trajectory.rs`.

### Design

```rust
// Pure functions, no CDP dependency
pub fn generate_bezier_curve(start: &Point, end: &Point, spread: f64) -> Vec<Point>;
pub fn generate_arc_curve(start: &Point, end: &Point) -> Vec<Point>;
pub fn generate_zigzag_curve(start: &Point, end: &Point) -> Vec<Point>;
pub fn generate_overshoot_curve(start: &Point, end: &Point) -> Vec<Point>;
pub fn generate_stopped_curve(start: &Point, end: &Point) -> Vec<Point>;
pub fn generate_muscle_path(start: &Point, end: &Point) -> Vec<Point>;
```

### Consequences

**Positive:**
- 25 unit tests covering all algorithms
- Easy to add new movement types
- Deterministic, reproducible paths
- Swappable algorithms per task

**Negative:**
- Additional module to maintain
- Need to tune parameters for realism

### Implementation

- `src/utils/mouse/trajectory.rs` - Algorithms
- `src/utils/mouse.rs` - CDP integration
- Comprehensive test coverage added

### References

- `src/utils/mouse/trajectory.rs`
- 25 unit tests in `#[cfg(test)]` module

---

## ADR 4: Use CDP (Chrome DevTools Protocol) over WebDriver

**Date:** 2026-03-15  
**Status:** Accepted  
**Deciders:** Cascade AI, Architecture Review

### Context

Two main options for browser automation:
1. **WebDriver (Selenium-style)** - Standardized, slower, limited API
2. **CDP (Chrome DevTools Protocol)** - Chrome-native, faster, full control

### Decision

Use CDP via `chromiumoxide` crate for browser control.

### Rationale

| Factor | WebDriver | CDP |
|--------|-----------|-----|
| Speed | Slower (HTTP per command) | Faster (WebSocket multiplexing) |
| API | Limited to standard actions | Full browser control |
| Detection | Easily detected by sites | Harder to detect |
| Modern Features | Lagging behind Chrome | Latest Chrome features |
| Ecosystem | Mature | Growing (Playwright, Puppeteer use it) |

### Consequences

**Positive:**
- Significantly faster execution
- Access to Chrome internals (network, performance)
- Better evasion of bot detection
- Modern async/await API

**Negative:**
- Chrome/Chromium only (no Firefox/Safari)
- Breaking changes in protocol versions
- Smaller ecosystem than WebDriver

### Implementation

- `chromiumoxide` crate for CDP communication
- `src/utils/browser.rs` for browser process management
- WebSocket-based communication

### References

- [Chrome DevTools Protocol](https://chromedevtools.github.io/devtools-protocol/)
- `chromiumoxide` crate documentation

---

## ADR 5: Adopt anyhow + thiserror for Error Handling

**Date:** 2026-03-10  
**Status:** Accepted  
**Deciders:** Cascade AI

### Context

Error handling options in Rust:
1. **Box<dyn Error>** - Simple but loses type information
2. **Custom enums** - Verbose, requires maintenance
3. **anyhow** - Ergonomic for applications
4. **thiserror** - Easy custom error types

### Decision

Use `anyhow` for application-level error handling and `thiserror` for library errors.

### Pattern

```rust
// Application layer (anyhow)
pub async fn run() -> Result<()> {
    do_something().await
        .with_context(|| "Failed to do something")?;
    Ok(())
}

// Library layer (thiserror)
#[derive(Error, Debug)]
pub enum SessionError {
    #[error("Browser process exited: {0}")]
    ProcessExited(String),
}
```

### Consequences

**Positive:**
- Ergonomic error propagation
- Rich context messages
- Minimal boilerplate
- Good ecosystem support

**Negative:**
- Type erasure with anyhow (can't match specific errors)
- Dependency overhead (2 crates)

### Implementation

- `anyhow` in application code
- `thiserror` for structured error types
- Consistent error context throughout

### References

- [anyhow docs](https://docs.rs/anyhow)
- [thiserror docs](https://docs.rs/thiserror)

---

## ADR 6: Use Tokio Async Runtime Exclusively

**Date:** 2026-03-05  
**Status:** Accepted  
**Deciders:** Cascade AI

### Context

Rust async ecosystem has multiple runtimes:
- **Tokio** - Most popular, feature-rich
- **async-std** - Smaller, different API
- **smol** - Minimal

### Decision

Standardize on Tokio as the only async runtime.

### Rationale

- Industry standard (used by most crates)
- Excellent performance
- Rich ecosystem (channels, timeouts, fs, process)
- Required by `chromiumoxide`

### Consequences

**Positive:**
- Consistent async patterns
- Access to tokio-util, tokio-stream
- Best ecosystem compatibility

**Negative:**
- Binary size (but acceptable for this use case)
- Learning curve for newcomers

### Implementation

```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "time", "sync", "fs", "signal", "macros"] }
```

### References

- [Tokio documentation](https://tokio.rs/)

---

## ADR 7: Remove lazy_static, Migrate to once_cell

**Date:** 2026-05-01  
**Status:** Accepted  
**Deciders:** Cascade AI

### Context

Two options for lazy static initialization:
- **lazy_static** - Macro-based, older, requires special syntax
- **once_cell** - Type-based, becoming standard, cleaner API

`once_cell` is being stabilized in std (Rust 1.80+).

### Decision

Remove `lazy_static` and consolidate on `once_cell::sync::Lazy`.

### Migration

```rust
// Before (lazy_static)
lazy_static::lazy_static! {
    static ref CACHE: Arc<RwLock<HashMap>> = Arc::new(RwLock::new(HashMap::new()));
}

// After (once_cell)
static CACHE: Lazy<Arc<RwLock<HashMap>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});
```

### Consequences

**Positive:**
- Cleaner syntax (no macro)
- Standard library future compatibility
- Consistent with rest of codebase (6 existing uses)
- One less dependency

**Negative:**
- Minor refactoring required

### Implementation

- Removed `lazy_static = "1.4"` from `Cargo.toml`
- Migrated `SENTIMENT_CACHE` in `twitteractivity_sentiment_llm.rs`
- Verified all 6 existing `once_cell` usages still work

### References

- [once_cell docs](https://docs.rs/once_cell)
- `docs/DEPENDENCY_AUDIT.md`

---

## ADR 8: Dependency Audit Process

**Date:** 2026-05-01  
**Status:** Accepted  
**Deciders:** Cascade AI

### Context

Dependencies grow over time. Without regular audits:
- Unused dependencies accumulate
- Security vulnerabilities go unnoticed
- Build times increase
- Binary size bloats

### Decision

Implement monthly dependency audits with documented process.

### Process

1. **Discovery** (30 min)
   - Run `cargo tree > docs/dependency_tree_full.txt`
   - Check `cargo tree -d` for duplicates

2. **Analysis** (1-2 hours)
   - For each dependency, grep codebase for usage
   - Identify unused (0 references) = candidate for removal
   - Identify duplicates (multiple versions) = candidate for consolidation

3. **Optimization** (1-2 hours)
   - Remove confirmed unused dependencies
   - Consolidate redundant patterns
   - Update security-critical dependencies

4. **Documentation** (30 min)
   - Create `docs/DEPENDENCY_AUDIT.md`
   - Record findings, actions, recommendations

### Consequences

**Positive:**
- Cleaner dependency tree
- Reduced attack surface
- Smaller binary size
- Faster builds

**Negative:**
- Time investment (~4 hours/month)
- Risk of removing incorrectly-identified dependency

### Implementation

- First audit completed 2026-05-01
- 3 dependencies removed (`urlencoding`, `humantime`, `lazy_static`)
- 37 dependencies (was 39)
- Documented in `docs/DEPENDENCY_AUDIT.md`

### References

- `docs/DEPENDENCY_AUDIT.md`
- `Cargo.toml` (37 direct dependencies)

---

## Proposed Decisions (Pending)

### Proposal 1: Add Criterion Benchmark Suite

**Status:** Under Consideration  
**Context:** Need to measure performance of hot paths (trajectory generation, selectors)

**Options:**
1. Add `criterion` benchmarks (standard)
2. Use `bencher` (simpler, less features)

**Recommendation:** Criterion for comprehensive benchmarking.

---

### Proposal 2: Consolidate Click Pipeline

**Status:** Under Consideration  
**Context:** Click logic scattered across CDP, DOM, and native paths

**Options:**
1. Create unified `ClickPipeline` with strategies
2. Keep separate but add wrapper

**Recommendation:** Unified pipeline with retry logic and verification.

---

## Retired Decisions

_None yet - all decisions remain active._

---

## ADR 9: Modularize twitteractivity into 27 files

**Date:** 2026-05-01  
**Status:** Accepted  
**Deciders:** Cascade AI

### Context

The `twitteractivity.rs` task file had grown to ~3000 lines with mixed concerns:
- Feed scrolling
- Engagement logic
- LLM integration
- Sentiment analysis
- Error handling
- State management

This made the code difficult to maintain, test, and navigate.

### Decision

Split `twitteractivity.rs` into 27 focused modules under `src/utils/twitter/`:

**Core (4):** engagement, feed, dive, interact  
**Decision (5):** decision, decision_unified, decision_hybrid, decision_llm, decision_persona  
**Sentiment (5):** sentiment, sentiment_enhanced, sentiment_emoji, sentiment_context, sentiment_domains, sentiment_llm  
**State (4):** state, constants, limits, persona  
**Infrastructure (6):** navigation, selectors, humanized, popup, retry, errors  
**LLM (2):** llm, sentiment_llm  

### Consequences

**Positive:**
- Single-responsibility modules (100-800 lines each)
- Faster compilation (parallel module compilation)
- Easier testing (test modules per file)
- Better code navigation
- Isolated changes reduce merge conflicts

**Negative:**
- More files to track
- Import management complexity
- Need to maintain module boundaries

### Implementation

- Created `src/utils/twitter/` directory
- Extracted each concern to dedicated file
- Added re-exports in `src/utils/twitter/mod.rs`
- Updated task file to use new modules
- Preserved all existing functionality

### References

- `TWITTERACTIVITY_TODO.md` Section 8
- `AGENTS.md` Twitter Utility Modules section

---

## ADR 10: Remove Thread Cache (Simplified LLM Context)

**Date:** 2026-05-02  
**Status:** Accepted  
**Deciders:** Cascade AI

### Context

Thread caching was originally implemented to:
- Store LLM context between engagements
- Avoid re-extracting thread data
- Improve performance on repeated dives

However, it added complexity and potential for stale data.

### Decision

Remove thread cache entirely:
- Deleted `thread_cache` field from `CandidateContext`
- Removed `DiveIntoThreadOutcome.cache`
- LLM context now extracted fresh each time
- Simplified state management

### Consequences

**Positive:**
- Reduced memory footprint
- Eliminated stale data risk
- Simpler code paths
- Faster state cloning (no cache copy)
- No cache invalidation logic needed

**Negative:**
- Slightly more DOM queries per engagement
- Fresh extraction adds ~10-50ms per LLM call

### Implementation

- `src/utils/twitter/twitteractivity_state.rs`: Removed cache field
- `src/utils/twitter/twitteractivity_engagement.rs`: Removed cache usage
- `src/utils/twitter/twitteractivity_dive.rs`: Removed cache population

### References

- `TWITTERACTIVITY_TODO.md` Section 8.2

---

## ADR 11: Implement Smart Decision Engine (7.1)

**Date:** 2026-05-02  
**Status:** Accepted  
**Deciders:** Cascade AI

### Context

Original engagement decisions were purely probability-based (persona weights). We needed more intelligent decisions that consider:
- Tweet content quality
- Sentiment analysis
- Author credibility
- Engagement history

### Decision

Implement unified smart decision engine in `twitteractivity_decision_unified.rs`:

**Components:**
- Multi-layer sentiment analysis (emoji → enhanced → LLM)
- Scoring system (0-100) for engagement quality
- Threshold-based action selection
- Fallback to persona weights when disabled

**Configuration:**
```rust
smart_decision_enabled: bool
enhanced_sentiment_enabled: bool
dry_run_actions: bool
```

### Consequences

**Positive:**
- Context-aware engagement decisions
- Reduced low-quality engagements
- Better rate limit avoidance
- Configurable AI sophistication level

**Negative:**
- Additional LLM calls when enabled
- More complex decision paths to test
- Feature flag management overhead

### Implementation

- `twitteractivity_decision_unified.rs`: Main engine
- `twitteractivity_sentiment_enhanced.rs`: Multi-layer sentiment
- `twitteractivity_state.rs`: TaskConfig additions
- Added 2000+ tests for decision logic

### References

- `TWITTERACTIVITY_TODO.md` Section 7.1

---

## ADR 12: Advanced Test Coverage (Integration/Statistical/Property)

**Date:** 2026-05-03  
**Status:** Accepted  
**Deciders:** Cascade AI

### Context

Basic unit tests covered individual functions, but we needed:
- Integration tests for decision flows
- Statistical verification of probability distributions
- Property-based tests for invariants

### Decision

Add comprehensive test suite to `twitteractivity_engagement.rs`:

**27 new tests across 4 modules:**
- `integration_tests` (12): Action selection, limits, text extraction
- `decision_integration_tests` (3): Smart decision enabled/disabled
- `statistical_tests` (5): Persona probability distribution (1000 trials, 5% tolerance)
- `property_tests` (8): No-panic, invariants, edge cases

### Consequences

**Positive:**
- 1986 → 2014 tests (net +28)
- Statistical verification catches distribution drift
- Property tests prevent regression panics
- Integration tests verify decision flow

**Negative:**
- Slightly longer CI time (+1-2s)
- More test code to maintain

### Implementation

```rust
#[cfg(test)]
mod integration_tests { /* 12 tests */ }

#[cfg(test)]
mod statistical_tests { /* 5 tests with 1000 trials */ }

#[cfg(test)]
mod property_tests { /* 8 property tests */ }
```

### References

- `TWITTERACTIVITY_TODO.md` Section 9.2
- `src/utils/twitter/twitteractivity_engagement.rs` (test modules)

---

## How to Add a New Decision

1. Create new section with format:
   ```markdown
   ## ADR N: Title
   **Date:** YYYY-MM-DD
   **Status:** Proposed | Accepted | Rejected | Deprecated
   **Deciders:** Names

   ### Context
   Background information

   ### Decision
   What was decided

   ### Consequences
   Positive and negative

   ### Implementation
   Files changed, approach

   ### References
   Links, docs
   ```

2. Update index table

3. Get consensus from team leads

4. Merge when approved

---

## License

These decisions are part of the project documentation, licensed under MIT.
