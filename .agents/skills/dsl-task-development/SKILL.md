# DSL Task Development — Executor Internals

Teaches agents about the DslExecutor internals — how actions are dispatched, how control flow works, variable substitution lifecycle, the include system, selector caching, and profiling.

## Architecture Overview

```
TaskDefinition (YAML)
  → parser.rs (parse_task_file / parse_task_yaml / parse_task_toml)
  → TaskDefinition.resolve_includes() (merge included files, cycle detection)
  → DslExecutor::new(api, task_def)
  → executor.execute() (main loop)
      → for each action:
          1. check_breakpoints()
          2. execute_action(action) → dispatch by action type
          3. record ActionMetrics
          4. profile via ActionProfiler
```

## File Map

| File | Purpose |
|---|---|
| `src/task/dsl/executor.rs` | `DslExecutor` struct, main `execute()` loop, `execute_action()` dispatch, Call action |
| `src/task/dsl/control_flow.rs` | If/Else, Loop, Foreach, While, Retry, Parallel, Try/Catch/Finally |
| `src/task/dsl/evaluator.rs` | `substitute_variables()`, `evaluate_condition()`, 20 condition types |
| `src/task/dsl/cache.rs` | `SelectorCache` with LRU eviction, TTL, hit rate stats |
| `src/task/dsl/types.rs` | `TaskDefinition`, `Action` (23 variants), `Condition` (20 variants), `IncludeSpec` |
| `src/task/dsl/actions/mod.rs` | Action handlers: browser, wait, inspection, media |
| `src/task/dsl/parser.rs` | YAML/TOML parsing, `get_task_definition()`, `validate_task_definition()` |
| `src/task/dsl/debug.rs` | `DebugEvent`, `Breakpoint`, `watch_variable()` |
| `src/task/dsl/profiling.rs` | `ActionProfiler`, `ActionMetrics`, `ExecutionReport` |
| `src/task/dsl/api/mod.rs` | `DslApi` trait, `MockDslApi` for testing |
| `src/task/dsl/mod.rs` | Module structure, re-exports |

## DslExecutor Struct

```rust
pub struct DslExecutor<'a, T: DslApi> {
    pub api: &'a T,                           // TaskContext API
    pub task_def: TaskDefinition,             // The loaded task definition
    pub variables: HashMap<String, String>,   // Runtime variables
    pub actions_executed: u32,
    pub call_depth: u32,                      // Recursion tracking (max 10)
    pub action_metrics: Vec<ActionMetrics>,
    pub start_time: Instant,
    pub actions_succeeded: u32,
    pub actions_failed: u32,
    pub debug_mode: bool,
    pub breakpoints: Vec<Breakpoint>,
    pub debug_events: Vec<DebugEvent>,
    pub paused: bool,
    pub watched_variables: HashMap<String, String>,
    pub selector_cache: SelectorCache,
    pub action_profilers: HashMap<String, ActionProfiler>,
    pub cache_enabled: bool,                  // default: true
    pub cache_ttl: Duration,                  // default: 5s
}
```

## Main Execute Loop

The `execute()` method iterates through actions sequentially:

```rust
pub async fn execute(&mut self) -> Result<()> {
    for (idx, action) in self.task_def.actions.clone().iter().enumerate() {
        // 1. Create ActionMetrics tracker
        let mut metrics = ActionMetrics::new(idx, &action_type);

        // 2. Check breakpoints (if breakpoint matches, set paused = true)
        if self.check_breakpoints(idx, &action_type) {
            self.paused = true;
        }

        // 3. Wait loop while paused (step-through debugging)
        loop { if !self.paused { break; } tokio::time::sleep(100ms).await; }

        // 4. Record ActionStart debug event
        self.record_debug_event(DebugEventType::ActionStart, ...);

        // 5. Execute the action
        match self.execute_action(action).await {
            Ok(()) => {
                metrics = metrics.complete();
                self.actions_succeeded += 1;
            }
            Err(e) => {
                metrics = metrics.fail(&error_msg);
                self.actions_failed += 1;
                return Err(e);  // First error propagates
            }
        }

        // 6. Record profiler data
        self.action_metrics.push(metrics);
        self.actions_executed += 1;
    }
}
```

**Key behaviors:**
- Actions are executed sequentially (exception: `Parallel` action)
- First error immediately stops execution and propagates
- Breakpoints pause before each matching action
- `ActionMetrics` are collected for every action
- Profiler data is aggregated per action type

## Action Dispatch

`execute_action()` is a single match statement dispatching to 23 action handlers:

```rust
pub(super) async fn execute_action(&mut self, action: &Action) -> Result<()> {
    match action {
        Action::Navigate { url }           => self.execute_navigate(url).await,
        Action::Click { selector }         => self.execute_click(selector).await,
        Action::Type { selector, text }    => self.execute_type(selector, text).await,
        Action::Wait { duration_ms }       => self.execute_wait(*duration_ms).await,
        Action::WaitFor { selector, .. }   => self.execute_wait_for(selector, timeout).await,
        Action::ScrollTo { selector }      => self.execute_scroll_to(selector).await,
        Action::Extract { selector, .. }   => self.execute_extract(selector, variable).await,
        Action::Execute { script }         => self.execute_js(script).await,
        Action::Log { message, level }     => self.execute_log(message, level).await,
        Action::If { condition, then, else } => self.execute_if(condition, then, else).await,
        Action::Loop { count, .. }         => self.execute_loop(count, condition, actions).await,
        Action::Call { task, parameters }  => self.execute_call(task, parameters).await,
        Action::Screenshot { path, .. }    => self.execute_screenshot(path, selector).await,
        Action::Clear { selector }         => self.execute_clear(selector).await,
        Action::Hover { selector }         => self.execute_hover(selector).await,
        Action::Select { selector, .. }    => self.execute_select(selector, value, by_value).await,
        Action::RightClick { selector }    => self.execute_right_click(selector).await,
        Action::DoubleClick { selector }   => self.execute_double_click(selector).await,
        Action::Parallel { actions, .. }   => self.execute_parallel(actions, max_concurrency).await,
        Action::Retry { actions, .. }      => self.execute_retry(actions, &config).await,
        Action::Foreach { variable, .. }   => self.execute_foreach(variable, collection, ...).await,
        Action::While { condition, .. }    => self.execute_while(condition, actions, max).await,
        Action::Try { try_actions, .. }    => self.execute_try(try_actions, catch, ...).await,
    }
}
```

Action handlers are split across modules:
- `actions/browser.rs`: Navigate, Click, Type, Hover, Select, ScrollTo, RightClick, DoubleClick, Clear
- `actions/wait.rs`: Wait, WaitFor
- `actions/inspection.rs`: Extract (text)
- `actions/media.rs`: Screenshot
- `control_flow.rs`: If, Loop, Foreach, While, Retry, Parallel, Try
- `executor.rs`: Call, Execute (JS), Log

## Control Flow Internals

### If/Else

```rust
pub(super) async fn execute_if(&mut self, condition: &Condition,
    then: &[Action], r#else: &Option<Vec<Action>>) -> Result<()>
```

- Evaluates condition via `self.evaluate_condition()`
- If true: executes then actions sequentially via `Box::pin(self.execute_action(action)).await`
- If false: executes else actions (if provided)
- Recursive: each action in then/else can be any Action variant (including nested If)

### Loop (fixed count or conditional)

```rust
pub(super) async fn execute_loop(&mut self, count: &Option<u32>,
    condition: &Option<Condition>, actions: &[Action]) -> Result<()>
```

- If `count` is set: iterates that many times
- If `condition` is set: condition-based loop with safety limit of **100 max iterations**
- If both: count takes priority, condition is ignored
- Logs warning when max iterations reached

### Foreach

```rust
pub(super) async fn execute_foreach(&mut self, variable: &str,
    collection: &ForeachCollection, actions: &[Action],
    max_iterations: &Option<u32>) -> Result<()>
```

Supports 4 collection types:

| Collection | Syntax Example | Behavior |
|---|---|---|
| `Array` | `type: array, values: [a, b, c]` | Iterates over literal YAML array values |
| `Range` | `type: range, start: 0, end: 5` | Iterates over `start..end` range |
| `Elements` | `type: elements, selector: ".item"` | Counts DOM elements, generates `:nth-of-type()` selectors |
| `Variable` | `type: variable, name: my_list` | Splits variable value by commas or uses as single item |

Each iteration binds the current value to the `variable` name in `self.variables`. Max iterations defaults to 100.

**Important:** `Elements` uses `:nth-of-type()` which is fragile with mixed DOM structures. Prefer `Array` or `Variable` collections for robustness.

### While

```rust
pub(super) async fn execute_while(&mut self, condition: &Condition,
    actions: &[Action], max_iterations: &Option<u32>) -> Result<()>
```

- Evaluates condition before each iteration
- Safety limit: **1000 max iterations** by default
- Warning when max iterations is reached

### Retry (exponential backoff)

```rust
pub struct RetryConfig {
    pub max_attempts: u32,           // default: 3
    pub initial_delay_ms: u64,       // default: 1000
    pub max_delay_ms: u64,           // default: 30000
    pub backoff_multiplier: f64,     // default: 2.0
    pub jitter: bool,                // default: true (0-20% random jitter)
    pub retry_on: Option<Vec<String>>, // default: None (retry all errors)
}
```

Flow:
1. Execute all actions in the block
2. If any action fails, check if error matches `retry_on` patterns (if specified)
3. Wait with exponential backoff + optional jitter
4. Repeat up to `max_attempts` times
5. If all attempts exhausted, return error with last failure message

### Parallel

```rust
pub(super) async fn execute_parallel(&mut self, actions: &[Action],
    max_concurrency: &Option<usize>) -> Result<()>
```

- Uses `tokio::sync::Semaphore` for concurrency limiting
- Creates a separate future for each action
- Waits for all to complete via `futures::future::join_all`
- Collects errors and reports all failures at end
- **Note:** Current implementation logs actions but doesn't fully execute them in parallel due to mutable borrow limitations

### Try/Catch/Finally

```rust
pub(super) async fn execute_try(&mut self,
    try_actions: &[Action],
    catch_actions: Option<&Vec<Action>>,
    error_variable: Option<&str>,
    finally_actions: Option<&Vec<Action>>) -> Result<()>
```

- Try block: executes actions, captures error if any
- Catch block: executes on error (can access error via `error_variable`)
- Finally block: always executes (even if try succeeds)
- **Errors in try are suppressed** — the function always returns `Ok` unless catch/finally fails
- Error variable stores the error message string for use in later actions

## Variable Substitution Lifecycle

Variables flow through 5 stages:

1. **Initialization**: CLI payload → `with_parameters()` inserts key-value pairs into `self.variables`
2. **Extraction**: `Extract` actions read DOM text and store it via `self.variables.insert(variable_name, text)`
3. **Substitution**: `substitute_variables(text)` replaces `${var_name}` placeholders before any action call
4. **Call propagation**: Parent variables copied to child executor; only NEW variables copied back (diff-based)
5. **Binding**: `Foreach` binds iteration values to the loop variable name

### Variable substitution

```rust
pub fn substitute_variables(&self, text: &str) -> String {
    let mut result = text.to_string();
    for (key, value) in &self.variables {
        let placeholder = format!("${{{key}}}");
        result = result.replace(&placeholder, value);
    }
    result
}
```

- Uses simple string replacement (not regex)
- Replaces `${var_name}` with variable value
- Applied to selectors in all action types, script in Execute, and parameters in Call
- Missing variables remain as literal `${var_name}` text (no error)

### Call variable semantics (3a + 3b)

```rust
// 3a: Snapshot pre-call variable names
let pre_call_vars: HashSet<String> = self.variables.keys().cloned().collect();

// 3b: Copy parent variables + override with Call parameters
called_executor.variables = self.variables.clone();
for (key, value) in params {
    let resolved = self.substitute_variables(&raw_value);
    called_executor.variables.insert(key, resolved);
}

// Execute the called task...
let result = called_executor.execute().await;

// 3a: Only copy back NEW variables (not in pre-call snapshot)
for (key, value) in called_executor.variables {
    if !pre_call_vars.contains(&key) {
        self.variables.insert(key, value);
    }
}
```

**Key insight**: Called tasks CANNOT modify parent variables. Only newly created variables are copied back. This prevents side effects from called tasks overwriting parent state.

## Include System

### IncludeSpec

```rust
pub struct IncludeSpec {
    pub path: String,                    // File path (relative or absolute)
    pub condition: Option<String>,       // Optional conditional include
}
```

### Resolution Flow

`resolve_includes()` is called on a `TaskDefinition`:

1. Takes all `IncludeSpec`s from the task
2. For each: resolves path (relative to the current task's directory)
3. Tracks visited paths in a `HashSet<PathBuf>` for **cycle detection**
4. Recursively resolves nested includes
5. Merges actions from included files (appended in order)
6. Merges parameters from included files (existing keys NOT overwritten)

```rust
pub fn resolve_includes(self, base_path: Option<&Path>) -> Result<Self, String> {
    let mut visited = HashSet::new();
    self.resolve_includes_inner(base_path, &mut visited)
}
```

### Cycle Detection

If the same file path is encountered twice (circular include), the second pass:
```rust
if !visited.insert(resolved_path.clone()) {
    log::warn!("Circular include detected: '{}' already processed, skipping", path);
    continue;
}
```

The cycle guard prevents infinite recursion but allows legitimate multi-level nesting:
- `A.task includes B.task includes A.task` → cycle detected, stops at A's second appearance
- `A.task includes B.task includes C.task` → no cycle, all 3 levels resolved

### Conditional Includes

Includes with a `condition` field are **skipped with a warning**:
```rust
if include.condition.is_some() {
    log::warn!("Conditional includes not yet supported: skipping '{}'", path);
    continue;
}
```

Conditional includes are not yet implemented — they'll eventually allow runtime-evaluated conditional inclusion.

## Selector Cache

### SelectorCacheEntry

```rust
pub struct SelectorCacheEntry {
    pub exists: bool,           // Does the element exist in DOM?
    pub visible: bool,          // Is the element visible?
    pub text: Option<String>,   // Text content (if extracted)
    pub count: usize,           // Element count (for collection selectors)
    pub cached_at: Instant,     // When cached
    pub ttl: Duration,          // TTL for this entry
}
```

### SelectorCache

```rust
pub struct SelectorCache {
    cache: HashMap<String, (SelectorCacheEntry, Instant)>,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}
```

| Property | Default | Description |
|---|---|---|
| Max size | 100 entries | LRU eviction when exceeded |
| Default TTL | 5 seconds | Created via `SelectorCacheEntry::new()` |
| Custom TTL | Via `with_ttl()` | Configurable per entry |
| Hit rate | Computed | `hits / (hits + misses)` |

### Cached Operations

The executor wraps DOM queries with caching:

```rust
// cached_exists — checks cache, fetches from API on miss
pub(super) async fn cached_exists(&mut self, selector: &str) -> Result<bool>
```

Free functions for cached operations (usable outside executor):
- `cached_visible()` — check visibility with cache
- `cached_text()` — get text content with cache

### Cache API

```rust
exec.enable_caching();               // Turn on caching
exec.disable_caching();              // Turn off + clear cache
exec.clear_cache();                   // Clear all entries
exec.set_cache_ttl(ms: u64);         // Set TTL in ms
let stats = exec.get_cache_stats();  // CacheStats { size, hits, misses, evictions, hit_rate }
```

Cache TTL is propagated to called tasks via `called_executor.cache_ttl = self.cache_ttl`.

## Action Profiler & Execution Reports

### ActionProfiler (aggregate per-type)

```rust
pub struct ActionProfiler {
    pub action_type: String,
    pub total_executions: u64,
    pub total_duration: Duration,
    pub min_duration: Option<Duration>,
    pub max_duration: Option<Duration>,
    pub failures: u64,
}
```

### Getting profiler stats

```rust
let stats = exec.get_profiler_stats();
// Returns HashMap<String, serde_json::Value> with per-action-type stats
// {
//   "Click": { "total_executions": 5, "average_duration_ms": 120, "failures": 0, ... },
//   "Navigate": { "total_executions": 1, "average_duration_ms": 3500, ... }
// }
```

### ActionMetrics (per-action)

```rust
pub struct ActionMetrics {
    pub index: usize,
    pub action_type: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub duration: Option<Duration>,
    pub success: bool,
    pub error: Option<String>,
}
```

Used for detailed action-by-action execution tracing.

## Condition Evaluation

The `evaluate_condition()` method handles 20 condition types. It uses the `DslApi` trait for DOM queries and `self.variables` for variable checks.

### Condition categories

| Category | Conditions | Evaluation Method |
|---|---|---|
| DOM state | `ElementExists`, `ElementVisible` | `api.exists()`, `api.visible()` |
| Text matching | `TextEquals`, `TextMatches` | `api.text()` + contains/substring |
| Variable state | `VariableEquals`, `VariableDefined`, `VariableNotDefined` | HashMap lookup |
| Numeric | `NumericGreaterThan`, `NumericLessThan`, `NumericRange` | `f64::parse()` + comparison |
| Date | `DateBefore`, `DateAfter` | `chrono::NaiveDate::parse_from_str()` |
| Array | `ArrayContains`, `ArrayLength` | Substring check, `.len()` |
| Compound | `And`, `Or` | Recursive evaluation of sub-conditions |
| Negation | `Not` | Wraps condition in `!` |
| Variable pattern | `VariableMatches` | Substring match on variable value |
| Constants | `True`, `False` | Direct boolean return |

All selector and value fields are passed through `substitute_variables()` before evaluation.

## Adding a New Action Handler

### Step 1: Add variant to `Action` enum in `types.rs`

```rust
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    // existing...
    MyNewAction {
        selector: String,
        parameter: Option<String>,
    },
}
```

### Step 2: Add dispatch in `executor.rs`

```rust
Action::MyNewAction { selector, parameter } => {
    self.execute_my_new_action(selector, parameter).await
}
```

### Step 3: Implement the handler

In the appropriate submodule (`actions/browser.rs`, `control_flow.rs`, or a new file):

```rust
impl<T: DslApi> super::DslExecutor<'_, T> {
    pub(super) async fn execute_my_new_action(
        &mut self,
        selector: &str,
        parameter: &Option<String>,
    ) -> Result<()> {
        let resolved_selector = self.substitute_variables(selector);
        self.api.my_new_api_method(&resolved_selector).await?;
        Ok(())
    }
}
```

### Step 4: Add tests

Use `MockDslApi` to test the action handler without a real browser:

```rust
#[tokio::test]
async fn test_my_new_action_calls_api() {
    let mock = MockDslApi::new();
    let mut exec = create_executor(&mock, vec![]);
    exec.execute_action(&Action::MyNewAction { ... }).await.unwrap();
    let calls = mock.get_calls();
    assert!(matches!(calls[0], MockCall::MyNewApiCall { ... }));
}
```

## Testing DSL Tasks

```powershell
# All DSL tests
cargo test --lib task::dsl

# Specific modules
cargo test --lib task::dsl::executor::tests
cargo test --lib task::dsl::control_flow::tests
cargo test --lib task::dsl::evaluator::tests
cargo test --lib task::dsl::cache::tests
cargo test --lib task::dsl::debug::tests
cargo test --lib task::dsl::profiling::tests
cargo test --lib task::dsl::types::tests

# Integration tests
cargo test dsl_integration
cargo test dsl_translation
```

## Common Pitfalls

1. **`Box::pin()` wrapping**: Every recursive `execute_action()` call inside control flow (If, Loop, Foreach, While, Retry, Try) must be wrapped in `Box::pin()` to work with async recursion. Without it, the future size grows unbounded.

2. **Foreach `Elements` fragility**: Using `:nth-of-type()` selectors for DOM element iteration breaks with mixed element types. Prefer `Array` collections with fixed values or `Variable` collections built from JavaScript execution results.

3. **Call variable isolation**: Called tasks CANNOT modify parent variables — only new variables are copied back. If you need to update a parent variable, use `Extract` with a new variable name in the called task, then reference that new variable in the parent.

4. **Parallel action limitations**: The current `Parallel` implementation does NOT fully execute actions concurrently — it only logs them. True parallel execution would require interior mutability (e.g., `Arc<Mutex<>>`) for the executor.

5. **Include condition not implemented**: Conditional includes (`include.condition`) are parsed but silently skipped at runtime. The condition field exists for future use.

6. **Circular include detection is path-based**: Two files with different paths that include each other's content (e.g., via symlinks) may not be detected as circular. Use unique file paths.

7. **Variable substitution precedence**: Substitution iterates over `self.variables` in HashMap iteration order. If one variable name is a substring of another (`user` and `user_name`), the longer name should be processed first. HashMap order is NOT guaranteed.

8. **`substitute_variables` is called before API calls**: Action handlers resolve `${variable}` placeholders BEFORE calling the API. If a placeholder has no matching variable, it remains as literal `${name}` in the selector/script.

9. **Breakpoint `clone()` drops conditions**: `Breakpoint.condition` is `Arc<dyn Fn()>` which cannot be cloned. After cloning, conditions are `None`.

10. **Cache TTL uses `Instant`**: The cache TTL is wall-clock time, not monotonic. System time changes (NTP, DST) may cause entries to expire prematurely or live longer than expected.
