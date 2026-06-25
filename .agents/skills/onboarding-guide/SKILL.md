# Onboarding Guide

Meta-skill for new contributors — route to the right skill based on task type, quickstart, and map of skills → directories → common tasks.

---

## Quickstart (First 30 Minutes)

```powershell
# 1. Build the project
cargo build --release

# 2. Run all lib tests
cargo test --lib

# 3. Run the fast check
.\check-fast.ps1

# 4. Run the full CI check
.\check.ps1

# 5. List available tasks
cargo run -- --list-tasks
```

You've successfully built, tested, and run the project. Now read the relevant skill for your task.

---

## Skill Router

What do you want to do?

| If you want to... | Read this skill first |
|---|---|
| **Add a new task** (Rust file) | `create-new-task-rs` |
| **Add a new task** (YAML DSL) | `create-new-task-dsl` |
| **Modify an existing task** (Rust) | `modify-task-rs` |
| **Modify an existing task** (DSL) | `modify-task-dsl` |
| **Write or fix tests** | `testing-rust` |
| **Work with Twitter tasks** | `twitter-module` |
| **Understand browser automation** | `cdp-browser-automation` |
| **Handle errors, retries, circuit breakers** | `error-handling-reliability` |
| **Add or change config options** | `configuration` |
| **Add logging or debug issues** | `logging-debugging` |
| **Work with DSL executor internals** | `dsl-task-development` |
| **Integrate or change LLM providers** | `llm-integration` |
| **Optimize Rust performance** | `codebase-optimization` |
| **Learn session/pool/factory lifecycle** | `session-lifecycle` |
| **Work with orchestrator pipeline** | `orchestrator-pipeline` |
| **Understand error types and TaskResult** | `result-system` |
| **Validate task definitions** | `validation` |
| **Run security audit or fix advisories** | `security-auditing` |
| **Learn spec workflow and commit format** | `git-workflow` |
| **Work with learning engine/predictive scorer** | `adaptive-learning` |

---

## Project Overview

### Directory Map

| Directory | Purpose | Key Skills |
|---|---|---|
| `src/` | All Rust source code | All skills |
| `src/task/` | Task definitions (16+ built-in tasks) | `create-new-task-rs`, `modify-task-rs` |
| `src/task/dsl/` | YAML DSL executor (actions, control flow, caching) | `dsl-task-development`, `create-new-task-dsl` |
| `src/session/` | Browser session lifecycle, pool, factory, permits | `session-lifecycle` |
| `src/orchestrator/` | Task execution pipeline, guards, retry, health | `orchestrator-pipeline` |
| `src/adaptive/` | Learning engine, predictive scorer, self-healing | `adaptive-learning` |
| `src/utils/twitter/` | Twitter automation (27+ modules) | `twitter-module` |
| `src/llm/` | LLM integration (providers, processors, strategies) | `llm-integration` |
| `src/runtime/task_context/` | `TaskContext` API, click learning, mouse, keyboard | `cdp-browser-automation` |
| `src/config/` | Configuration types, defaults, env var overrides | `configuration` |
| `src/validation/` | Task validation, registry validation | `validation` |
| `src/result/` | Error types, TaskResult, error classification | `result-system` |
| `src/error.rs` | Top-level error types | `error-handling-reliability` |
| `src/health_logger.rs` | Background health monitoring | `logging-debugging`, `error-handling-reliability` |
| `src/metrics.rs` | Metrics collection and export | `logging-debugging` |
| `tests/` | Integration tests | `testing-rust` |
| `config/` | TOML configuration files (`default.toml`, `llm.toml`) | `configuration` |
| `docs/specs/` | Spec packages (`_template/`, `_active/`, `_done/`) | `git-workflow` |
| `docs/` | All documentation | — |
| `scripts/` | CI scripts, git hooks | `git-workflow` |
| `.agents/skills/` | These skill files | — |

### Common Task Patterns

**Adding a new Twitter task:**
1. Read `twitter-module` (understand architecture)
2. Read `modify-task-rs` (learn safety patterns)
3. Read `testing-rust` (write proper tests)
4. Read `git-workflow` (commit and push)

**Fixing a bug in browser automation:**
1. Read `cdp-browser-automation` (understand click pipeline)
2. Read `error-handling-reliability` (retry and circuit breaker)
3. Read `logging-debugging` (add debug logging)
4. Read `testing-rust` (write regression test)

**Adding a config option:**
1. Read `configuration` (8-step recipe)
2. Read `testing-rust` (write config tests)
3. Read `git-workflow` (commit format)

---

## Who to Ask for Help

| Issue | Where to ask |
|---|---|
| Code changes | Buffy (the AI agent) — chat in the CLI |
| Spec system | Buffy will create or modify specs |
| GitHub issues | Open an issue in the repository |
| Skill questions | Buffy can explain any skill in detail |

---

## First Change Workflow

```powershell
# 1. Read the relevant skill
# 2. Read the relevant source files
# 3. Make your change
# 4. Run fast check
.\check-fast.ps1
# 5. Run relevant tests
cargo test --lib <module>::tests
# 6. Run full CI
.\check.ps1
# 7. Commit
git add -A
git commit -m "type: summary (reason)"
git push origin v0.2.32
```

**Commit format reminder:** `type: short summary (reason/impact)`
- Good: `feat: add twitterquote task (reuse LLM reply flow)`
- Bad: `update`, `fix`, `changes`
