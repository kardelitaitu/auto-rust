# Rust Migration: Master Index

**Project:** Node.js Browser Automation Orchestrator → Rust  
**Reference Codebase:** `nodejs reference codebase/`  
**Target:** `rust-orchestrator/`  
**Timeline:** 1-3 weeks (depending on number of tasks)

---

## Overview

This migration converts a Node.js CLI browser automation orchestrator (Playwright) to a pure Rust version using `chromiumoxide`. The goal is to eliminate all race conditions and memory leaks while keeping **exactly the same CLI behavior** for the smoke test (`cargo run cookiebot pageview=www.reddit.com then cookiebot`).

### Current Problems (Node.js)
- Race conditions between parallel tabs/tasks
- Memory leaks (forgotten pages, listeners, contexts)
- Unstable long-running behavior (needs frequent restarts)
- High memory usage (~500MB after 1 hour)

### Rust Benefits
- Zero race conditions (borrow checker guarantees safety)
- Automatic cleanup via RAII → no leaks even after weeks of continuous use
- 5-10x lower memory usage (~100MB after 1 hour)
- Single static binary (no Node.js runtime)
- Can run 24/7 without babysitting

---

## Migration Phases

| Phase | Document | Duration | Status |
|-------|----------|----------|--------|
| **1** | [Project Setup & Skeleton](phase-1-project-setup.md) | 1 day | ✅ Documented |
| **2** | [Core Orchestrator Engine](phase-2-core-orchestrator.md) | 2-3 days | ✅ Documented |
| **3** | [Utils Library (Human-Like Behaviors)](phase-3-utils-library.md) | 2-4 days | ✅ Documented |
| **4** | [Task Migration Guide](phase-4-task-migration.md) | 1 day/task | ✅ Documented |
| **5** | [Testing & Validation](phase-5-testing-validation.md) | 3-5 days | ✅ Documented |
| **6** | [Polish & Deployment](phase-6-polish-deployment.md) | 1-2 days | ✅ Documented |

---

## Quick Navigation

### Phase 1: Project Setup & Skeleton
- [Create Rust project structure](phase-1-project-setup.md#11-create-rust-project)
- [Cargo.toml dependencies](phase-1-project-setup.md#12-cargotoml-dependencies)
- [config.toml.example template](phase-1-project-setup.md#14-configtomlexample)
- [Skeleton source files](phase-1-project-setup.md#16-create-skeleton-source-files)

### Phase 2: Core Orchestrator Engine
- [CLI argument parsing (then separator)](phase-2-core-orchestrator.md#21-cli-argument-parsing-srclirs)
- [Configuration loader](phase-2-core-orchestrator.md#22-configuration-loader-srcconfigrs)
- [Browser connection (chromiumoxide)](phase-2-core-orchestrator.md#23-browser-connection-srcbrowserrs)
- [Session management](phase-2-core-orchestrator.md#24-session-management-srcsessionrs)
- [Orchestrator task queue](phase-2-core-orchestrator.md#25-orchestrator-srcorchestratorrs)
- [Task module dispatcher](phase-2-core-orchestrator.md#26-task-module-dispatcher-taskmodrs)

### Phase 3: Utils Library
- [Math utilities (gaussian, random)](phase-3-utils-library.md#31-math-utilities-srcutilsmathrs)
- [Timing utilities (human delays)](phase-3-utils-library.md#32-timing-utilities-srcutilstimingrs)
- [Navigation utilities (goto, back)](phase-3-utils-library.md#33-navigation-utilities-srcutilsnavigationrs)
- [Scroll utilities (read, focus)](phase-3-utils-library.md#34-scroll-utilities-srcutilsscrollrs)
- [Mouse utilities (Bezier, Fitts)](phase-3-utils-library.md#35-mouse-utilities-srcutilsmousers)
- [Keyboard utilities (typing)](phase-3-utils-library.md#36-keyboard-utilities-srcutilskeyboardrs)

### Phase 4: Task Migration
- [Task migration overview](phase-4-task-migration.md#41-task-migration-overview)
- [Side-by-side examples (pageview, cookiebot)](phase-4-task-migration.md#43-side-by-side-migration-examples)
- [Common migration patterns](phase-4-task-migration.md#44-common-migration-patterns)
- [Adding new tasks (template)](phase-4-task-migration.md#48-adding-a-new-task-template)

### Phase 5: Testing & Validation
- [Unit tests for utils](phase-5-testing-validation.md#52-unit-tests-for-utils-srctestsutils_testrs)
- [Integration tests](phase-5-testing-validation.md#53-integration-tests-srctestsintegration_testrs)
- [Long-running stability test](phase-5-testing-validation.md#56-long-running-test-script)
- [Memory/CPU monitoring](phase-5-testing-validation.md#57-memory--cpu-monitoring)
- [Edge case tests](phase-5-testing-validation.md#55-edge-case-tests-srctestsedge_case_testrs)

### Phase 6: Polish & Deployment
- [Release binary build](phase-6-polish-deployment.md#61-build-release-binary)
- [Shell wrappers](phase-6-polish-deployment.md#62-shell-wrapper-optional)
- [Enhanced error handling](phase-6-polish-deployment.md#63-enhanced-error-handling--retry-logic)
- [Structured logging](phase-6-polish-deployment.md#64-structured-logging)
- [GitHub Actions CI](phase-6-polish-deployment.md#66-github-actions-ci-optional)

---

## Architecture Comparison

### Node.js Architecture

```
main.js
  ├── task-parser.js          (CLI arg parsing, "then" separator)
  ├── orchestrator.js         (Task queue, batch execution)
  │     ├── sessionManager.js (Worker pools, page acquisition)
  │     └── automator.js      (Playwright CDP connections)
  ├── discovery.js            (Browser auto-scan)
  └── tasks/*.js              (Task modules)
        └── api/index.js      (Unified API: goto, scroll, type, etc.)
```

### Rust Architecture

```
main.rs
  ├── cli.rs                  (CLI arg parsing, "then" separator)
  ├── orchestrator.rs         (Task queue, batch execution)
  │     ├── session.rs        (Worker pools, page acquisition)
  │     └── browser.rs        (chromiumoxide CDP connections)
  ├── config.rs               (config.toml loader)
  └── task/*.rs               (Task modules)
        └── utils/*.rs        (Navigation, scroll, type, timing, etc.)
```

---

## Technology Mapping

| Node.js | Rust | Notes |
|---------|------|-------|
| Playwright `chromium.connectOverCDP()` | `chromiumoxide::Browser::connect()` | CDP protocol |
| `page.goto(url, options)` | `navigation::goto(page, url, options)` | Equivalent API |
| `page.mouse.move(x, y)` | `mouse::mouse_move_to(page, x, y)` | Bezier path |
| `page.keyboard.type(text)` | `keyboard::human_type(page, selector, text)` | Typo injection |
| `setTimeout(fn, ms)` | `tokio::time::sleep(Duration::from_millis(ms))` | Async sleep |
| `Promise.race()` | `tokio::time::timeout()` | Timeout wrapping |
| `Promise.allSettled()` | `futures::future::join_all()` | Parallel execution |
| `fs.readFile()` | `std::fs::read_to_string()` | File I/O |
| `Math.random()` | `rand::thread_rng().gen_range()` | Random numbers |
| `logger.info()` | `info!()` (tracing crate) | Structured logging |
| `try/catch` | `match result { Ok, Err }` | Error handling |
| `createSuccessResult()` | `Ok(())` | Rust Result type |

---

## Key Files Reference

### Node.js Reference Codebase

| File | Purpose |
|------|---------|
| `main.js` | Primary entry point, orchestrator initialization |
| `api/utils/task-parser.js` | CLI argument parsing, "then" separator logic |
| `api/core/orchestrator.js` | Task queue, batch execution, worker loops |
| `api/core/sessionManager.js` | Worker pools, page acquisition/release |
| `api/core/automator.js` | Playwright CDP connections, circuit breaker |
| `api/core/discovery.js` | Browser auto-scan, connector loading |
| `api/interactions/navigation.js` | goto, reload, back, forward |
| `api/interactions/scroll.js` | read, focus, scroll, toTop, toBottom |
| `api/interactions/actions.js` | click, type, hover |
| `api/interactions/cursor.js` | Mouse movement, fidgeting |
| `api/utils/ghostCursor.js` | Bezier paths, Fitts's Law, overshoot |
| `api/behaviors/timing.js` | think, delay, gaussian, randomInRange |
| `api/utils/math.js` | Box-Muller, PID controller |
| `tasks/cookiebot.js` | Loop navigation task example |
| `tasks/pageview.js` | Single URL visit task example |

### Rust Target Structure

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry point, orchestrator initialization |
| `src/cli.rs` | CLI argument parsing, "then" separator logic |
| `src/orchestrator.rs` | Task queue, batch execution, worker loops |
| `src/session.rs` | Worker pools, page acquisition/release |
| `src/browser.rs` | chromiumoxide CDP connections, circuit breaker |
| `src/config.rs` | config.toml loading, defaults |
| `src/utils/navigation.rs` | goto, reload, back, forward |
| `src/utils/scroll.rs` | read, focus, scroll, toTop, toBottom |
| `src/utils/mouse.rs` | Mouse movement, Bezier paths, Fitts's Law |
| `src/utils/keyboard.rs` | Typing with typo injection |
| `src/utils/timing.rs` | thinking_pause, human_delay, gaussian |
| `src/utils/math.rs` | Box-Muller, PID controller, random |
| `task/cookiebot.rs` | Loop navigation task (migrated) |
| `task/pageview.rs` | Single URL visit task (migrated) |
| `task/mod.rs` | Task registry, perform_task dispatcher |

---

## Success Criteria

- [ ] **CLI smoke test passes**: `cargo run cookiebot pageview=www.reddit.com then cookiebot`
- [ ] **Memory usage stays flat** after 24-48 hours of continuous runs
- [ ] **No more race conditions or leaks** (Rust borrow checker guarantees)
- [ ] **All tasks migrated** to `task/` folder
- [ ] **Helpers easily accessible** via `use crate::utils::*;`
- [ ] **New tasks can be added** in < 30 minutes

---

## Getting Started

### Prerequisites

1. Install Rust: https://rustup.rs/
2. Install Chrome/Chromium browser (for CDP)
3. Clone repository

### Step 1: Create Project

Follow [Phase 1](phase-1-project-setup.md) to create the project structure.

### Step 2: Implement Core Engine

Follow [Phase 2](phase-2-core-orchestrator.md) to implement CLI parsing, browser discovery, and task execution.

### Step 3: Build Utils

Follow [Phase 3](phase-3-utils-library.md) to implement human-like behaviors.

### Step 4: Migrate Tasks

Follow [Phase 4](phase-4-task-migration.md) to migrate each Node.js task to Rust.

### Step 5: Test

Follow [Phase 5](phase-5-testing-validation.md) to validate stability and performance.

### Step 6: Deploy

Follow [Phase 6](phase-6-polish-deployment.md) to build release binary and deploy.

---

## Risks & Mitigations

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Borrow checker learning curve | Medium | Use ready-made code from these documents |
| Missing CDP feature | Low | chromiumoxide is very complete in 2026 |
| Many similar tasks | Medium | Generic helpers in utils/ reduce duplication |
| Long-term maintenance | Low | Modular structure (one task = one file) |

---

## Timeline (Realistic)

```
Week 1: Phase 1 + 2 (working core)
  ├── Day 1: Project setup, skeleton, dependencies
  └── Day 2-3: Core orchestrator, CLI, browser connection

Week 2: Phase 3 + 4 (utils + tasks)
  ├── Day 1-2: Utils library (math, timing, navigation)
  ├── Day 3-4: Utils library (scroll, mouse, keyboard)
  └── Day 5-7: Task migration (1 day per complex task)

Week 3: Phase 5 + 6 (testing + deployment)
  ├── Day 1-2: Unit + integration tests
  ├── Day 3-4: Long-running tests, edge cases
  └── Day 5-7: Release build, CI, deployment
```

---

## Documents Index

| Document | Path | Purpose |
|----------|------|---------|
| **Whitepaper** | `whitepaper.md` | Original migration plan, high-level overview |
| **Phase 1** | `migration-steps/phase-1-project-setup.md` | Project structure, dependencies, skeleton code |
| **Phase 2** | `migration-steps/phase-2-core-orchestrator.md` | CLI parsing, orchestrator, browser connection |
| **Phase 3** | `migration-steps/phase-3-utils-library.md` | Human-like behaviors (scroll, mouse, keyboard, timing) |
| **Phase 4** | `migration-steps/phase-4-task-migration.md` | Task migration guide with side-by-side examples |
| **Phase 5** | `migration-steps/phase-5-testing-validation.md` | Testing strategy, long-running tests, monitoring |
| **Phase 6** | `migration-steps/phase-6-polish-deployment.md` | Release build, shell wrappers, CI, deployment |

---

## Notes

- All code examples in these documents are **ready to use** - copy/paste and adapt as needed
- chromiumoxide API may differ slightly from Playwright - test and adjust
- Rust's ownership system prevents many bugs that exist in the Node.js version
- RAII guarantees no memory leaks even after weeks of continuous use
- Each phase builds on the previous one - follow in order

---

**Last Updated:** April 15, 2026  
**Document Version:** 1.0
