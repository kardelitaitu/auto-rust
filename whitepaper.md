# Migration Plan: Node.js Browser Automation Orchestrator → Rust

**Document Version:** 3.0
**Date:** April 15, 2026
**Author:** Migration Planning Document
**Project Goal:** Completely replace the current Node.js CLI orchestrator (Playwright) with a pure Rust version using `chromiumoxide`.
Eliminate all race conditions and memory leaks while keeping **exactly the same CLI behavior** (`cargo run cookiebot pageview=www.reddit.com then cookiebot`).

---

## Executive Summary

Your Node.js orchestrator currently suffers from:
- Race conditions between parallel tabs/tasks
- Memory leaks (forgotten pages, listeners, contexts)
- Unstable long-running behavior (needs frequent restarts)

**Rust migration benefits**:
- Zero race conditions (borrow checker guarantees safety)
- Automatic cleanup via RAII → no leaks even after weeks of continuous use
- Identical CLI and behavior
- 5–10× lower memory usage
- Single static binary (no Node.js runtime)
- Can run 24/7 without babysitting

**Effort estimate**: 1–3 weeks (depending on number of tasks).
**Risk**: Very low — we already have a fully working prototype.

---

## Current State (Node.js)

- CLI command: `node main.js task1 task2=url then task3`
- Connects to already-opened browser(s) via Playwright `connect`
- Tasks run in batches:
  - Inside one batch → parallel on separate tabs
  - Between batches → sequential
- Tasks (`.js` files) read URLs from `.txt` files, do random navigation, scrolling, timings, etc.
- Pain points: races, leaks, high memory growth over time

---

## Target State (Rust)

- CLI command: `cargo run cookiebot pageview=www.reddit.com then cookiebot` (or `./orchestrator ...` after build)
- **Tasks can be specified with or without `.js` extension**: `cookiebot` or `cookiebot.js` both work
- **Payload syntax**: `taskname=value`, `taskname=url=value`, or `taskname=key=value`
- Internal payloads should preserve type where possible, so numeric inputs stay numeric and URLs stay strings
- Auto-scans browsers from `config.toml`
- Same batching logic (`then` separator)
- All tasks implemented as pure Rust modules
- Uses `chromiumoxide` (native CDP, no Node.js dependency)
- Production-ready single binary

## Verification Contract

The authoritative smoke test for the migration is:

`cargo run cookiebot pageview=www.reddit.com then cookiebot`

Everything else in the parser and task system should be judged by whether it preserves that exact behavior and the related normalization rules.

---

## CLI Behavior Specification (Exact Match)

### Supported CLI Patterns

```bash
# Single task (no payload)
cargo run cookiebot

# Single task (with .js extension)
cargo run cookiebot.js

# Multiple tasks in one group (parallel execution)
cargo run cookiebot pageview

# Task with URL payload (auto-prepends https://)
cargo run pageview=www.reddit.com

# Task with explicit URL key
cargo run pageview=url=https://example.com

# Task with numeric payload
cargo run taskname=42

# Sequential groups (using 'then' separator)
cargo run cookiebot then pageview

# Complex example: multiple groups with mixed payloads
cargo run cookiebot pageview=www.reddit.com then cookiebot twitterFollow=x.com/user

# With browser filter option
cargo run -- --browsers=localChrome cookiebot pageview=reddit.com
```

### Parsing Rules

1. **`then` keyword**: Splits tasks into sequential groups (case-insensitive)
2. **Tasks without `=`**: Simple task name, empty payload
3. **Tasks with `=`**: First `=` separates task name from value
   - If value is numeric → payload: `{ "value": 42 }`
   - If value looks like URL (contains `.` or `localhost`) → payload: `{ "url": "https://value" }`
   - Otherwise → payload: `{ "url": "value" }`
4. **Multiple `=`**: `task=url=https://x.com` → task: `task`, payload: `{ "url": "https://x.com" }`
5. **`.js` extension**: Automatically stripped if present (`cookiebot.js` → `cookiebot`)
6. **Quoted values**: `task="value with spaces"` → quotes removed

---

## Final Folder Structure (Exactly as Requested)

```bash
rust-orchestrator/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── .gitignore
├── LICENSE (optional)
│
├── config.toml                 # Live browser URLs (add to .gitignore)
├── config.toml.example         # Safe template for git
│
├── task/                       # ← TASK FOLDER AT ROOT (your request)
│   ├── mod.rs                  # Task registry + perform_task dispatcher
│   ├── cookiebot.rs            # cookiebot task logic
│   ├── task2.rs
│   ├── scraper.rs
│   └── ...                     # One .rs file per task
│
├── data/                       # All .txt files (URL lists, etc.)
│   ├── cookiebot.txt
│   ├── task2.txt
│   └── ...
│
├── src/                        # All Rust source code
│   ├── main.rs                 # Thin entry point
│   ├── cli.rs                  # Clap + batch parsing ("then")
│   ├── config.rs               # config.toml loader + auto-scan
│   ├── browser.rs              # chromiumoxide connection logic
│   │
│   ├── utils/                  # ← UTILS FOLDER INSIDE SRC (your request)
│   │   ├── mod.rs              # ← Barrel file for easy imports
│   │   ├── navigation.rs       # goto, wait_for, etc.
│   │   ├── scroll.rs           # random_scroll, human_scroll
│   │   ├── mouse.rs            # human_mouse_move, click
│   │   ├── keyboard.rs         # natural_typing, press_key
│   │   ├── timing.rs           # random_delay, human_pause
│   │   └── ...                 # Add more as needed
│   │
│   └── tests/                  # ← TESTS FOLDER INSIDE SRC (your request)
│       ├── mod.rs
│       ├── integration_test.rs
│       └── task_test.rs
│
└── target/                     # Auto-generated (ignored)
```

Why this structure is ideal:
- `task/` at root → super easy to add/edit tasks
- `utils/` with `mod.rs` → clean imports like `use crate::utils::*;`
- `tests/` inside `src/` → exactly as requested
- Clean separation: engine in `src/`, tasks in root `task/`, data in `data/`

---

## Migration Phases

### Phase 1: Project Setup & Skeleton (1 day)
- Create new Rust project with the exact folder structure above
- Add all dependencies in Cargo.toml
- Create `config.toml.example` and `config.toml`
- Wire up `#[path = "../task/mod.rs"]` and utils barrel
- Copy working prototype code (already provided in our chat)

**Deliverable:** Compiles and runs `cargo run cookiebot`

### Phase 2: Core Orchestrator Engine (2–3 days)
- `src/cli.rs` – argument parsing + batch logic
- `src/config.rs` – load config + browser auto-scan
- `src/browser.rs` – connect + handler
- `src/main.rs` – glue everything together
- `task/mod.rs` – perform_task dispatcher

**Deliverable:** Full batching (`then`) and parallel task execution working

### Phase 3: Utils Library (2–4 days)
- Build reusable helpers in `src/utils/`:
  - `navigation.rs`, `scroll.rs`, `mouse.rs`, `keyboard.rs`, `timing.rs`
- Export everything from `src/utils/mod.rs`
- Implement human-like behaviors (random delays, natural mouse movement, smooth scrolling)

**Deliverable:** Rich set of helpers ready for all tasks

### Phase 4: Task Migration (1 day per complex task)
- Migrate each Node.js task to its own file in `task/`
- Example: `task/cookiebot.rs` (already done)
- Reuse utils heavily
- Add new tasks without touching other files

**Deliverable:** All existing tasks work identically in Rust

### Phase 5: Testing & Validation (3–5 days)
- Long-running tests in `src/tests/`
- Side-by-side comparison with Node.js version
- Memory/CPU monitoring
- Edge cases (browser offline, empty `.txt`, large batches)

**Deliverable:** 100% confidence in stability

### Phase 6: Polish & Deployment (1–2 days)
- Build release binary
- Add optional shell wrapper
- Logging, error handling, retry logic
- (Optional) GitHub Actions CI

**Deliverable:** Production-ready binary

---

## Risks & Mitigations

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Borrow checker learning curve | Medium | Use the ready prototype I provided |
| Missing CDP feature | Low | chromiumoxide is very complete in 2026 |
| Many similar tasks | Medium | Generic helpers in utils/ reduce duplication |
| Long-term maintenance | Low | Modular structure (one task = one file) |

---

## Timeline (Realistic)

- **Week 1:** Phase 1 + 2 (working core)
- **Week 2:** Phase 3 + 4 (utils + tasks)
- **Week 3:** Phase 5 + 6 (testing + deployment)

**Total:** 1–3 weeks depending on number of tasks.

---

## Success Criteria

- ✅ **CLI works exactly as before:** `cargo run cookiebot pageview=www.reddit.com then cookiebot`
- ✅ **Tasks accept with or without `.js`:** `cookiebot` and `cookiebot.js` both work
- ✅ **URL payloads auto-format:** `pageview=reddit.com` → `https://reddit.com`
- ✅ **Memory usage stays flat** after 24–48 hours of continuous runs
- ✅ **No more race conditions or leaks**
- ✅ **All tasks migrated** to `task/` folder
- ✅ **Helpers easily accessible** via `use crate::utils::*;`
- ✅ **You can add new tasks** in < 30 minutes
