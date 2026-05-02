# Task Runner Proposal: Hybrid Task System

**Status:** Draft  
**Date:** 2026-05-02  
**Author:** AI Assistant  
**Target Version:** v0.0.5

---

## 1. Executive Summary

Extend the existing task system to support **both** Rust-based tasks (`.rs`) and human-readable DSL tasks (`.task`) with the **same CLI interface**. Users can run either type seamlessly:

```bash
# Runs src/task/oldtask.rs (existing behavior preserved)
cargo run oldtask

# Runs ~/.auto/tasks/newtask.task (new capability)
cargo run newtask
```

DSL tasks use a minimal line-based syntax for rapid task authoring without Rust code. Existing Rust tasks continue to work unchanged while new simple tasks can be written in minutes.

### Goals
- **Backward compatibility**: All existing Rust tasks work without modification
- **Lower barrier**: Non-Rust users can write DSL tasks
- **Unified interface**: Single `cargo run taskname` command for both types
- **Seamless coexistence**: Mix .rs and .task files
- **Rapid prototyping**: Test ideas in DSL before implementing in Rust

---

## 2. Proposed Design

### 2.1 File Format Specification

**Locations:**
- Rust tasks: `src/task/{taskname}.rs` (existing, unchanged)
- DSL tasks: `~/.auto/tasks/{taskname}.task` (new)

### 2.1.1 DSL Format Specification

The DSL is a **line-based, action-oriented** format optimized for readability and rapid authoring.

#### File Structure

```
# Line 1: Task name
name: strawberry_search

# Line 2: Duration range (min-max in seconds, or with 's', 'm', 'h' suffix)
duration: 10-120s

# Subsequent lines: Actions (one per line)
navigate https://google.com
click "textarea[name='q']"
type "textarea[name='q']" "strawberry" --humanize
keypress Enter
wait_navigation 5000
screenshot  # Auto-saved to data/screenshot/
end
```

#### Syntax Rules

1. **Comments**: Lines starting with `#` are ignored
2. **Empty lines**: Ignored
3. **Header** (first 2 non-comment lines):
   - `name: <identifier>` - Task identifier
   - `duration: <min>-<max><unit>` - Random duration between min and max
4. **Actions** (remaining lines): `action_name [param1] [param2] ... [flags]`
5. **Quotes**: Double quotes for parameters containing spaces or special characters
6. **Flags**: Start with `--` (e.g., `--humanize`, `--retry=3`)

#### Automatic Delay Between Actions

By default, the executor adds a random delay between each action to simulate human behavior:

```
Default: 1000-2000ms (random uniform distribution)
```

The delay can be customized per task:

```
name: cookiebot_simple
duration: 30-60s
delay: 500-1500ms  # Custom delay range

navigate https://example.com
click "#accept"   # 500-1500ms delay after this
screenshot  # Auto-saved to data/screenshot/
end
```

Use `delay: none` to disable automatic delays:

```
name: fast_task
duration: 10s
delay: none

navigate https://example.com
click "#submit"   # No delay, immediate next action
end
```

### 2.2 Supported DSL Action Types

| Action | Description | Syntax | Example |
|--------|-------------|--------|---------|
| `navigate` | Navigate to URL | `navigate <url>` | `navigate https://google.com` |
| `click` | Click element | `click <selector>` | `click "#submit"` |
| `type` | Type text | `type <selector> <text> [flags]` | `type "#search" "hello" --humanize` |
| `keypress` | Press key | `keypress <key>` | `keypress Enter` |
| `scroll` | Scroll element | `scroll <selector> <direction> <amount>` | `scroll window down 3` |
| `pause` | Wait duration | `pause <ms>` | `pause 2000` |
| `wait_visible` | Wait for element | `wait_visible <selector> [timeout]` | `wait_visible "#result" 5000` |
| `wait_navigation` | Wait for page load | `wait_navigation [timeout]` | `wait_navigation 5000` |
| `screenshot` | Capture screenshot | `screenshot` | `screenshot` |
| `end` | End task | `end` | `end` |

#### Flags Reference

| Flag | Description | Applicable Actions |
|------|-------------|-------------------|
| `--humanize` | Human-like typing speed | `type` |
| `--clear` | Clear field before typing | `type` |
| `--retry=<n>` | Retry on failure up to n times | `click`, `type` |
| `--optional` | Skip action if element not found | `click`, `type`, `wait_visible` |
| `--screenshot` | Screenshot if action fails | `click`, `type` |

### 2.3 DSL Examples

#### Simple Cookiebot

```
name: cookiebot_simple
duration: 30-60s

navigate https://example.com
pause 3000
click "#onetrust-accept-btn-handler" --optional
screenshot "./result.png"
end
```

#### Google Search with Screenshot

```
name: strawberry_search_ss
duration: 10-120s

navigate https://google.com
click "textarea[name='q']"
type "textarea[name='q']" "strawberry" --humanize
keypress Enter
wait_navigation 5000
screenshot "./screenshots/strawberry_result.png"
end
```

#### Twitter Engagement (Simple)

```
name: twitter_like_feed
duration: 300-600s

navigate https://twitter.com
wait_visible "[data-testid='primaryColumn']" 10000

# Scroll and like loop (simplified)
scroll "[data-testid='primaryColumn']" down 3
pause 2000
click "button[aria-label*='Like']" --optional --retry=3
pause 1000

end
```

### 2.4 Selector Syntax in DSL

```toml
# Simple CSS selector
selector = "button[aria-label='Like']"

# Accessibility locator (existing syntax)
selector = "role=button[name='Like'][scope='main']"

# Variable reference
selector = { type = "variable", name = "current_tweet" }

# Within parent
selector = { type = "within", parent = "tweet", selector = "button[aria-label*='Like']" }
```

---

## 3. Implementation Plan

### 3.1 New Modules

```
src/
├── task/
│   ├── mod.rs              # Existing task module
│   ├── dsl/                # NEW: DSL task system
│   │   ├── mod.rs          # Public API
│   │   ├── lexer.rs        # Tokenize DSL lines
│   │   ├── parser.rs       # Build AST from tokens
│   │   ├── ast.rs          # AST definitions
│   │   ├── executor.rs     # Bridge to TaskContext
│   │   └── error.rs        # Error types & messages
│   └── runner/             # Existing runner (unchanged)
```

User task files location:
```
~/.auto/tasks/
├── strawberry_search_ss.task    # Example DSL task
├── cookiebot_simple.task        # Another example
└── linkedin_connect.task        # User-created tasks
```

### 3.2 Task Discovery Mechanism

```rust
// src/task/dsl/mod.rs - Public API
use std::path::PathBuf;

pub struct DslTaskRunner;

impl DslTaskRunner {
    /// Execute a DSL task file
    pub async fn execute_file(
        path: &Path,
        ctx: &TaskContext,
    ) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let task = parser::parse(&content)?;
        executor::run(&task, ctx).await
    }
    
    /// Check if a DSL task exists for given name
    pub fn find_task(name: &str) -> Option<PathBuf> {
        let path = dirs::home_dir()?
            .join(".auto/tasks")
            .join(format!("{}.task", name));
        if path.exists() { Some(path) } else { None }
    }
}
```

### 3.3 CLI Integration

**No CLI changes required.** The existing task dispatch system is extended to support both formats:

```rust
// Enhanced task resolution in src/task/mod.rs
pub async fn execute_single_attempt(
    api: &TaskContext,
    name: &str,
    payload: &Value,
    config: &Config,
) -> Result<()> {
    // Check for DSL task first (user-defined)
    if let Some(dsl_path) = dsl::DslTaskRunner::find_task(name) {
        return dsl::DslTaskRunner::execute_file(&dsl_path, api).await;
    }
    
    // Fall back to existing Rust tasks
    match name {
        "cookiebot" => cookiebot::run(api, payload.clone()).await,
        "pageview" => pageview::run(api, payload.clone()).await,
        // ... existing tasks
        _ => Err(anyhow!("Unknown task: {}", name)),
    }
}
```

**CLI behavior remains identical:**
```bash
cargo run taskname [OPTIONS]
```

The system automatically checks `~/.auto/tasks/{name}.task` before falling back to built-in Rust tasks.

### 3.4 Execution Flow

```
cargo run strawberry_search_ss
           ↓
    CLI receives "strawberry_search_ss"
           ↓
    Check for ~/.auto/tasks/strawberry_search_ss.task
           ↓  ├─ Yes → DslTaskRunner::execute_file()
           ↓  └─ No  → Check for src/task/strawberry_search_ss.rs
                          ↓
                   ├─ Yes → run_rust_task()
                   └─ No  → Error: Task not found
```

**Resolution Priority:**
1. If `~/.auto/tasks/{name}.task` exists → run as DSL task
2. If `src/task/{name}.rs` exists → run as Rust task
3. If neither → error

---

## 4. DSL Implementation Details

### 4.1 Architecture Overview

The DSL system consists of 5 components (~660 lines total):

| Component | Lines | Purpose |
|-----------|-------|---------|
| **Lexer** | ~80 | Tokenize lines, handle quoted strings |
| **Parser** | ~120 | Build AST from tokens |
| **AST** | ~60 | Data structures for Task/Action |
| **Executor** | ~180 | Bridge to TaskContext |
| **Error Handling** | ~60 | Line numbers, helpful messages |
| **Tests** | ~120 | Unit tests |
| **Integration** | ~40 | File loading, CLI hook |
| **Total** | **~660** | Production + tests |

### 4.2 AST Definition (ast.rs - ~60 lines)

```rust
#[derive(Debug)]
pub struct Task {
    pub name: String,
    pub duration_min: u64,  // seconds
    pub duration_max: u64,  // seconds
    pub delay_ms: Option<(u64, u64)>,  // Optional (min, max) delay between actions
    pub actions: Vec<Action>,
}

#[derive(Debug)]
pub enum Action {
    Navigate { url: String },
    Click { selector: String, optional: bool, retry: u32 },
    Type { selector: String, text: String, humanize: bool },
    Keypress { key: String },
    Pause { duration_ms: u64 },
    Scroll { selector: String, direction: Direction, amount: u32 },
    WaitVisible { selector: String, timeout_ms: u64 },
    WaitNavigation { timeout_ms: u64 },
    Screenshot,
}

#[derive(Debug)]
pub enum Direction { Up, Down, Left, Right }
```

### 4.3 Parser Implementation (parser.rs - ~120 lines)

```rust
pub fn parse(input: &str) -> Result<Task, ParseError> {
    let mut lines = input.lines().enumerate();
    
    // Parse header: name | duration
    let (_, first) = lines.next().ok_or(ParseError::Empty)?;
    let name = parse_name_line(first)?;
    
    let (_, second) = lines.next().ok_or(ParseError::MissingDuration)?;
    let (min, max) = parse_duration_line(second)?;
    
    // Parse actions
    let mut actions = Vec::new();
    for (line_no, line) in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        actions.push(parse_action_line(line, line_no)?);
    }
    
    Ok(Task { name, duration_min: min, duration_max: max, actions })
}

fn parse_action_line(line: &str, line_no: usize) -> Result<Action, ParseError> {
    let tokens = tokenize(line)?;
    match tokens.first().map(|s| s.as_str()) {
        Some("navigate") => parse_navigate(&tokens, line_no),
        Some("click") => parse_click(&tokens, line_no),
        Some("type") => parse_type(&tokens, line_no),
        // ... etc
        Some(unknown) => Err(ParseError::UnknownAction {
            action: unknown.to_string(),
            line: line_no,
        }),
        None => Err(ParseError::EmptyLine { line: line_no }),
    }
}
```

### 4.4 Executor Implementation (executor.rs - ~180 lines)

```rust
pub async fn run(task: &Task, api: &TaskContext) -> Result<()> {
    // Compute random duration
    let duration_secs = random_in_range(task.duration_min, task.duration_max);
    let deadline = Instant::now() + Duration::from_secs(duration_secs);
    
    // Parse delay config (default: 1000-2000ms)
    let (delay_min, delay_max) = task.delay_ms.unwrap_or((1000, 2000));
    
    info!(
        "Running task '{}' for {} seconds ({} actions, {}-{}ms delays)",
        task.name, duration_secs, task.actions.len(), delay_min, delay_max
    );
    
    for (idx, action) in task.actions.iter().enumerate() {
        // Check deadline before executing
        if Instant::now() > deadline {
            info!("Task duration reached ({}s), exiting gracefully", duration_secs);
            return Ok(());
        }
        
        // Execute action
        match execute_action(action, api).await {
            Ok(_) => {}
            Err(e) if is_recoverable(&e) => {
                warn!("Action {} failed (recoverable): {}", idx, e);
            }
            Err(e) => {
                error!("Action {} failed: {}", idx, e);
                return Err(e);
            }
        }
        
        // Add random delay between actions (except after the last one)
        if idx < task.actions.len() - 1 && delay_max > 0 {
            let delay_ms = random_in_range(delay_min, delay_max);
            debug!("Pausing {}ms before next action", delay_ms);
            sleep(Duration::from_millis(delay_ms)).await;
        }
    }
    
    info!("Task '{}' completed all actions", task.name);
    Ok(())
}

async fn execute_action(action: &Action, api: &TaskContext) -> Result<()> {
    match action {
        Action::Navigate { url } => api.goto(url).await,
        Action::Click { selector, optional, retry } => {
            for attempt in 0..=*retry {
                match api.click(selector).await {
                    Ok(_) => return Ok(()),
                    Err(e) if *optional && attempt == *retry => {
                        warn!("Optional click failed, skipping: {}", e);
                        return Ok(());
                    }
                    Err(e) if attempt < *retry => {
                        warn!("Click failed, retrying: {}", e);
                        api.pause(1000).await;
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(())
        }
        Action::Type { selector, text, humanize } => {
            if *humanize {
                api.r#type_with_humanize(selector, text).await
            } else {
                api.r#type(selector, text).await
            }
        }
        Action::Keypress { key } => api.keyboard(key).await,
        Action::Pause { duration_ms } => api.pause(*duration_ms).await,
        Action::Screenshot => api.screenshot().await,
        // ... etc
    }
}
```

### 4.5 Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Task file is empty")]
    Empty,
    #[error("Missing duration line")]
    MissingDuration,
    #[error("Invalid name format at line {line}: {text}")]
    InvalidName { line: usize, text: String },
    #[error("Invalid duration format at line {line}: {text}")]
    InvalidDuration { line: usize, text: String },
    #[error("Unknown action '{action}' at line {line}")]
    UnknownAction { action: String, line: usize },
    #[error("Missing required parameter for {action} at line {line}")]
    MissingParameter { action: String, line: usize },
}
```

### 4.6 Implementation Timeline

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| 1. AST + Lexer | 1 day | Can tokenize DSL files |
| 2. Parser | 1 day | Can parse to AST |
| 3. Executor | 1 day | Can execute against TaskContext |
| 4. Error Messages | 0.5 day | Nice errors with line numbers |
| 5. Integration | 0.5 day | CLI `cargo run taskname` works |
| 6. Tests | 1 day | 90%+ coverage |
| **Total** | **4-5 days** | **~660 lines of code** |

### 4.7 Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_simple_task() {
        let input = r#"
name: test_task
duration: 10-30s

navigate https://example.com
click "#button"
end
"#;
        let task = parse(input).unwrap();
        assert_eq!(task.name, "test_task");
        assert_eq!(task.duration_min, 10);
        assert_eq!(task.duration_max, 30);
        assert_eq!(task.actions.len(), 3);
    }
    
    #[test]
    fn test_parse_with_flags() {
        let input = r#"
name: flagged_task
duration: 5-10s

type "#input" "hello" --humanize
click "#submit" --optional --retry=3
end
"#;
        let task = parse(input).unwrap();
        // Verify flags parsed correctly
    }
}
```

---

## 5. DSL Validation & Error Handling

### 5.1 Validation During Parsing

DSL validation is integrated into the parser with line-number-accurate error messages:

```
Error: Invalid task definition in 'strawberry_search_ss.task'

  Line 4: Unknown action type: 'clik'
   ↳ Did you mean 'click'?

  Line 7: Missing required parameter 'text' for action 'type'
   ↳ Expected: type <selector> <text> [flags]

  Line 2: Invalid duration format: '10-120x'
   ↳ Expected format: <min>-<max>s (e.g., 10-120s)
```

### 5.2 Dry-Run Mode

```bash
# Validate DSL syntax without executing
cargo run strawberry_search_ss --dry-run

# Output:
# ✓ Task name: strawberry_search_ss
# ✓ Duration: 10-120s (random)
# ✓ 7 actions found
# ✓ All actions valid
# ✓ All selectors parseable
# ✓ DSL validation passed (would run for ~65 seconds)
```

### 5.3 Runtime Validation

During execution:
- Selector syntax validated before each click/type
- URLs validated as well-formed
- Screenshots auto-saved with timestamped filenames
- Duration deadline enforced between actions

---

## 6. Advanced Features (Phase 2)

### 6.1 Variables and State (Future)

Store and manipulate values during task execution:

```
name: advanced_task
duration: 60-120s

# Store element count
find_all "article" -> tweet_count

# Conditional based on count
if tweet_count > 5
    click "button[aria-label='Load more']"
    pause 2000
endif

# Increment counter
set liked = 0
click "button[aria-label*='Like']" -> liked + 1

screenshot  # Saved with auto-generated filename
end
```

### 6.2 Task Composition (Future)

Include common action sequences:

```
name: twitter_engagement
duration: 300-600s

# Include predefined sequence
include login_twitter

navigate https://twitter.com/home
scroll window down 5
click "button[aria-label*='Like']" --optional

include cleanup_logout
end
```

### 6.3 Custom Actions via Plugins (Future)

Allow extending the DSL with custom actions:

```
name: ml_filtered_engagement
duration: 600-900s

navigate https://twitter.com

# Custom ML action (provided by plugin)
ml_score_tweets threshold=0.7 -> high_quality_tweets

for_each high_quality_tweets
    click "button[aria-label*='Like']"
    pause 1500
endfor

end
```

---

## 7. Coexistence Strategy

### 7.1 Both Formats Work Together

**Existing Rust tasks remain fully supported.** No migration required:

```bash
# Rust task (src/task/twitter_like.rs)
cargo run twitter_like

# DSL task (~/.auto/tasks/cookiebot_simple.task)
cargo run cookiebot_simple
```

Both tasks coexist and can be listed with:
```bash
cargo run --list-tasks
# Output:
#  ✓ twitter_like       (Rust)   src/task/twitter_like.rs
#  ✓ cookiebot_simple     (DSL)    ~/.auto/tasks/cookiebot_simple.task
```

### 7.2 When to Use Which Format

| Use DSL When... | Use Rust When... |
|------------------|------------------|
| Simple sequential actions | Complex conditional logic |
| No custom data processing | Heavy data transformation |
| Standard browser interactions | Need external API calls |
| Quick prototyping | Performance-critical paths |
| Sharing with non-Rust users | Need custom ML models |
| Cookie acceptance, simple logins | Complex Twitter engagement |

### 7.3 Gradual Migration (Optional)

If you want to convert a Rust task to DSL:

```bash
# 1. Create DSL version alongside Rust version
# ~/.auto/tasks/cookiebot.task (new)
# src/task/cookiebot.rs (existing)

# 2. DSL takes precedence (test the new version)
cargo run cookiebot

# 3. Once verified, remove the .rs file
```

**Note:** Migration is entirely optional. Rust tasks continue to work indefinitely.

---

## 7. Benefits

| Stakeholder | Benefit |
|-------------|---------|
| **End Users** | Write automation without learning Rust |
| **Developers** | Rapid prototyping before implementation |
| **DevOps** | Version-controllable task definitions |
| **QA** | Easy to read and review test scenarios |
| **Community** | Share task recipes via GitHub |

---

## 8. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| TOML becomes complex | Strict schema validation + linting |
| Performance overhead | Compiled cache of validated tasks |
| Maintenance burden | Auto-migration tools for schema updates |
| Security (arbitrary code) | Sandboxed action whitelist |

---

## 9. Implementation Phases

### Phase 1: Core DSL Runner (Week 1)
- [ ] Lexer + Parser (~200 lines)
- [ ] AST definitions (~60 lines)
- [ ] Executor bridge to TaskContext (~180 lines)
- [ ] CLI integration (no changes needed)
- [ ] 5 core actions: navigate, click, type, pause, screenshot
- [ ] Basic error reporting with line numbers

### Phase 2: Advanced Features (Week 3-4)
- [ ] Condition system
- [ ] Variables and state
- [ ] For-each loops
- [ ] Error handling (on_error)
- [ ] Dry-run mode

### Phase 3: Polish (Week 5)
- [ ] `list-tasks` command
- [ ] Task documentation generation
- [ ] Validation linting
- [ ] Example task library
- [ ] Documentation

---

## 10. Open Questions

1. Should we support additional formats (TOML, YAML) alongside DSL?
2. How to handle task file hot-reload during development?
3. Should we add a REPL mode for interactive task testing?
4. Do we need a task marketplace/registry system?
5. Should we support "include" for task composition in Phase 1?

---

## 11. Appendix: Example Tasks

### Simple Cookiebot

```
# ~/.auto/tasks/cookiebot_simple.task
name: cookiebot_simple
duration: 30-60s

navigate https://example.com
pause 3000
click "#onetrust-accept-btn-handler" --optional
screenshot  # Auto-saved to data/screenshot/
end
```

### Google Search with Screenshot

```
# ~/.auto/tasks/strawberry_search_ss.task
name: strawberry_search_ss
duration: 10-120s

navigate https://google.com
click "textarea[name='q']"
type "textarea[name='q']" "strawberry" --humanize
keypress Enter
wait_navigation 5000
screenshot  # Auto-saved to data/screenshot/
end
```

### Twitter Engagement (Simple)

```
# ~/.auto/tasks/twitter_like_feed.task
name: twitter_like_feed
duration: 300-600s

navigate https://twitter.com
wait_visible "[data-testid='primaryColumn']" 10000

# Scroll and like loop (simplified)
scroll "[data-testid='primaryColumn']" down 3
pause 2000
click "button[aria-label*='Like']" --optional --retry=3
pause 1000

end
```

### Complex Tasks Stay in Rust

Complex tasks like `twitteractivity` (ML filtering, conditional logic, state management) remain Rust tasks:

```rust
// src/task/twitteractivity.rs - Too complex for DSL
pub async fn run(api: &TaskContext, payload: Value) -> Result<()> {
    // ML-based engagement scoring
    // Complex retry logic
    // State machines
    // Custom ML models
    // etc.
}
```

**Guideline:** If a task needs conditionals, loops, or custom logic → use Rust. If it's a linear sequence of actions → use DSL.

---

**Next Steps:**
1. Review and approve proposal
2. Create Phase 1 implementation plan
3. Set up task runner module structure
4. Implement basic parser and executor
