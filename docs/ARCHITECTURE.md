# Auto-Rust Architecture

## Overview

Auto-Rust is a Rust-based browser automation orchestrator for social media engagement (primarily Twitter/X). It uses Chromium via `chromiumoxide` for browser control and provides a task-based DSL for defining automation workflows.

## Module Hierarchy

```
src/
├── lib.rs                 # Crate root, re-exports
├── main.rs               # CLI binary entry point
├── cli/                   # Command-line interface
│   ├── mod.rs             # CLI module, command dispatch
│   └── parser.rs          # Task group parsing, argument handling
├── task/                  # Task runner and DSL
│   ├── mod.rs             # Task module root
│   ├── dsl/               # DSL executor (modularized from 2,362 lines)
│   │   ├── mod.rs           # Re-exports and module declarations
│   │   ├── types.rs         # DurationMs, Action, Condition types
│   │   ├── cache.rs        # SelectorCache for DOM element caching
│   │   ├── debug.rs        # DebugEvent, Breakpoint, DebugEventType
│   │   ├── profiling.rs    # ActionProfiler, ActionMetrics, ExecutionReport
│   │   ├── evaluator.rs    # Variable substitution, condition evaluation
│   │   ├── control_flow.rs # If/Loop/Foreach/While/Retry/Parallel
│   │   ├── executor.rs     # DslExecutor main struct and execute()
│   │   ├── parser.rs        # DSL parsing (YAML/TOML), validation
│   │   └── dsl_executor.rs # Backward-compat shim for old path
│   ├── validation.rs       # Pre-flight task validation
│   └── demo_mouse.rs       # Demo mouse task implementation
├── utils/
│   ├── mouse.rs            # Mouse simulation (2,877 lines, has submodules)
│   │   ├── native.rs      # Native input, calibration (680 lines)
│   │   ├── trajectory.rs  # Bezier/Arc/Zigzag curves (500 lines)
│   │   └── types.rs       # Point, PathStyle, etc. (74 lines)
│   ├── twitter/            # Twitter automation (27 files)
│   │   ├── twitteractivity_engagement.rs  # Core engagement logic
│   │   ├── twitteractivity_*.rs          # 26 other twitter modules
│   │   └── sentiment/                   # Sentiment analysis sub-modules
│   ├── math.rs             # Gaussian, random_in_range utilities
│   ├── scroll.rs           # Page scrolling utilities
│   ├── timing.rs           # Human-pause, delay utilities
│   └── ...                 # Other utility modules
├── llm/                   # LLM integration
│   ├── unified_processor.rs  # LLM response processing
│   └── ...
└── config/                # Configuration structures
```

## Data Flow

```
User Input (CLI)
    │
    ▼
cli/parser.rs (parse task groups)
    │
    ▼
TaskContext (orchestrator)
    │
    ├──► load task YAML/TOML
    │
    ▼
validation.rs (pre-flight checks)
    │
    ▼ (if valid)
DslExecutor::execute()
    │
    ├──► evaluator.rs (substitute variables, evaluate conditions)
    ├──► control_flow.rs (if/loop/foreach/while/retry/parallel)
    └──► executor.rs (dispatch actions)
            │
            ▼
        twitteractivity_engagement.rs (for Twitter tasks)
            │
            ▼
        twitteractivity_interact.rs (click, type, hover, etc.)
            │
            ▼
        TaskContext::api.click() / .type() / .pause()
            │
            ▼
        chromiumoxide (CDP protocol)
            │
            ▼
        Browser (Chrome/Brave/Roxybrowser)
```

## Key Design Decisions

### 1. Task DSL (Domain-Specific Language)
- **Why**: Non-technical users need to define automation workflows
- **Format**: YAML (primary) with TOML fallback
- **Structure**: Actions, conditions, parameters, includes
- **Implementation**: `dsl/` modular structure (9 files after spec 0017)

### 2. Twitter Module Organization
- **Why**: Twitter automation has 27+ specialized modules
- **Pattern**: Each concern has its own file (sentiment, retry, selectors, etc.)
- **Benefit**: Isolated testing, clear responsibilities
- **Note**: Refactoring within files (spec 0018, 0021) avoids over-modularization

### 3. Mouse Simulation Architecture
- **Why**: Human-like mouse movement requires complex math (Bezier curves, Fitts's Law)
- **Structure**: Root file + 3 submodules (native, trajectory, types)
- **Path styles**: Bezier, Arc, Zigzag, Overshoot, Stopped, Muscle
- **Refactoring**: Extract helpers within file (check_point_collision, dispatch_single_mouse_event)

### 4. Browser Abstraction
- **Why**: Support multiple Chromium browsers (Chrome, Brave, Roxybrowser)
- **Implementation**: `TaskContext` provides unified API
- **Actions**: `api.click()`, `api.type()`, `api.pause()`, `api.hover()`, etc.
- **Async**: All browser interactions are async via tokio

### 5. Browser Support
- **Supported**: Brave, Chrome, and Roxybrowser
- **Future connectors**: Other Chromium browsers are planned later
- **Scope**: Browser-specific behavior stays behind the unified `TaskContext` API

### 6. TaskContext Contract
- **Entry point**: `TaskContext` stays thin and composes shared capabilities.
- **Pause behavior**: `api.pause(base_ms)` uses a uniform ±20% random delay; `api.pause_with_variance(base_ms, pct)` uses the same uniform model with a custom spread; `api.pause_human(base_ms, pct)` uses a Gaussian delay.
- **Cancellation**: `TaskContext::new` and `new_with_metrics` take a final `Option<CancellationToken>` so pauses can wake early on group cancel.
- **Settle timing**: High-level task-api verbs already add a post-action settle pause.
- **Default interaction**: `api.click(selector)` runs the selector pipeline with scroll + move + click.
- **Execution model**: Task groups broadcast to every active browser session; parallel fan-out is the default.
- **Validation contract**: Validation and task execution share one payload resolver for alias handling and normalization.
- **Parsing boundary**: Task-specific parsing stays out of orchestrator code when shared validation can own it.
- **Run summaries**: Include active, healthy, and unhealthy session counts plus per-task/per-session breakdowns.
- **Health warning**: Emit a warning when healthy sessions drop below the operational threshold.
- **API surface**: Prefer task-api verbs that stay on the API surface, not ad hoc helpers.
- **Text helpers**: Keep shared UTF-8-safe text helpers in the internal/text utility layer.
- **Verification**: Prefer deterministic verification of the same target element that was clicked or inspected.
- **Task names**: Keep task names canonical and consistent across `task/mod.rs`, `src/cli.rs`, validation, and README.

### 7. LLM Integration
- **Why**: Generate contextual replies, quotes, sentiment analysis
- **Pattern**: Unified processor handles multiple LLM providers
- **Usage**: `twitteractivity_llm.rs` for reply/quote generation

## Extension Points

### Adding a New Task Type
1. Define action in `dsl/types.rs` (add to `Action` enum)
2. Implement execution in `dsl/executor.rs` or separate module
3. Add validation in `dsl/parser.rs` (validate_action)
4. Create demo in `task/demo_*.rs` (optional)

### Adding a New Twitter Feature
1. Create module in `utils/twitter/` (e.g., `twitteractivity_new.rs`)
2. Implement feature (e.g., new engagement action)
3. Integrate into `twitteractivity_engagement.rs` (add to action dispatch)
4. Add tests in module's `#[cfg(test)]` block

### Adding a New Browser
1. Add browser detection in `TaskContext` or config
2. Implement browser-specific logic if needed
3. Update documentation in `README.md`

## Testing Strategy

- **Unit tests**: Inline in each module (`#[cfg(test)]`)
- **Integration tests**: `tests/` directory for cross-module tests
- **DSL tests**: `dsl/` modules have comprehensive test coverage
- **Twitter tests**: Inline in twitter modules (sentiment, engagement, etc.)
- **CI**: `cargo nextest` runs all tests via `check.ps1`

## Performance Considerations

- **Selector caching**: `SelectorCache` with TTL (5 seconds) for DOM lookups
- **Human-like timing**: `human_pause()`, `clustered_engagement_pause()` avoid detection
- **Retry with backoff**: `retry_with_backoff()` for resilient actions
- **Parallel execution**: `Parallel` action for concurrent task execution

## Security Notes

- **No credential storage**: API keys via environment variables
- **Limited scope**: Only automates specified actions (no arbitrary code execution)
- **CDP protocol**: Direct browser control without external dependencies
