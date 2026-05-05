# Release v0.1.1 - Enhanced DSL Features

**Release Date:** May 5, 2026  
**Tag:** `v0.1.1`

## Overview

This release adds advanced DSL capabilities including enhanced condition types, comprehensive debugging features, and performance optimizations to the declarative automation framework.

---

## What's New

### Enhanced Conditions (8 New Condition Types)

Extended the condition system for more powerful control flow:

| Condition | Description | Example |
|-----------|-------------|---------|
| `text_matches` | Regex pattern matching on element text | `selector: "#result", pattern: "^Success.*"` |
| `variable_matches` | Regex matching on variable values | `name: "status", pattern: "completed|done"` |
| `numeric_greater_than` | Numeric comparison | `name: "count", value: 10` |
| `numeric_less_than` | Numeric comparison | `name: "price", value: 100.0` |
| `numeric_range` | Range check (inclusive) | `name: "age", min: 18, max: 65` |
| `date_before` | Date comparison | `name: "expiry", date: "2026-12-31"` |
| `date_after` | Date comparison | `name: "created", date: "2026-01-01"` |
| `array_contains` | Check if array contains value | `name: "tags", value: "important"` |
| `array_length` | Validate array length | `name: "items", min: 1, max: 10` |

**Example Usage:**
```yaml
- if:
    condition:
      text_matches:
        selector: "#status"
        pattern: "Order #(\d+) confirmed"
    then:
      - log:
          message: "Order confirmed!"
          level: info

- if:
    condition:
      numeric_range:
        name: "total_price"
        min: 10.0
        max: 1000.0
    then:
      - click:
          selector: "#checkout-button"
```

### DSL Debugging Features

Comprehensive debugging capabilities for complex task development:

#### Breakpoint Types
- **Action Index Breakpoint** - Pause at specific action number
- **Action Type Breakpoint** - Pause on any action of a given type
- **Variable Watch Breakpoint** - Trigger when variables change
- **Conditional Breakpoint** - Custom condition function

#### Debug Event Types
- `ActionStart` - Action execution begins
- `ActionComplete` - Action finishes successfully  
- `ActionError` - Action fails with error
- `VariableSet` - Variable value changes
- `ConditionEvaluated` - Condition evaluation result

#### Debugging API
```rust
// Enable debug mode
let mut executor = DslExecutor::new(api, &task_def);
executor.with_debug_mode(true);

// Add breakpoints
executor.add_breakpoint(Breakpoint::on_action(5));  // Pause at action 5
executor.add_breakpoint(Breakpoint::on_action_type("click"));  // Pause on any click
executor.add_breakpoint(Breakpoint::watch_variable("user_id"));  // Watch for changes

// Execute with debugging
executor.execute().await?;

// Get execution trace
let events = executor.get_debug_events();
for event in events {
    println!("[{}] {:?}", event.timestamp, event.event_type);
}

// Inspect state mid-execution
let state = executor.inspect_state();
println!("Variables: {:?}", state["variables"]);

// Pause/Resume control
if executor.is_paused() {
    executor.resume();  // Continue execution
    // or
    executor.step();    // Execute one action, then pause again
}
```

### Performance Optimizations

#### Selector Caching
LRU cache for DOM queries with automatic TTL expiration:

```rust
// Caching is enabled by default
let mut executor = DslExecutor::new(api, &task_def);

// Control caching
executor.enable_caching();   // Enable (default)
executor.disable_caching();  // Disable and clear cache
executor.clear_cache();      // Clear without disabling

// Monitor cache performance
let stats = executor.get_cache_stats();
println!("Cache hit rate: {:.1}%", stats.hit_rate * 100.0);
println!("Size: {}/{} entries", stats.size, stats.hits + stats.misses);
```

**Cache Features:**
- **LRU Eviction** - 100 entry capacity with automatic eviction
- **TTL Support** - 5-second default expiration for cache entries
- **Hit Rate Tracking** - Performance metrics for optimization
- **Smart Invalidation** - Automatic cache clearing on mutations

#### Action Profiling
Per-action-type performance tracking:

```rust
// Get profiling data
let profiler_stats = executor.get_profiler_stats();
for (action_type, stats) in profiler_stats {
    println!("{}: {} executions, avg {}ms",
        action_type,
        stats["total_executions"],
        stats["average_duration_ms"].as_u64().unwrap_or(0)
    );
}
```

**Profiled Metrics:**
- Total executions per action type
- Total/average/min/max duration
- Failure rate tracking
- JSON export for analysis

---

## Updated DSL Action Reference

Complete list of all 23 DSL actions (5 new since v0.1.0):

| Action | Description | Category |
|--------|-------------|----------|
| `navigate` | Navigate to URL | Navigation |
| `click` | Click element | Interaction |
| `type` | Type text | Interaction |
| `wait` | Pause execution | Control |
| `wait_for` | Wait for element | Control |
| `scroll_to` | Scroll to element | Navigation |
| `extract` | Get element text | Data |
| `clear` | Clear input | Interaction |
| `hover` | Hover element | Interaction |
| `select` | Select dropdown | Interaction |
| `right_click` | Right-click | Interaction |
| `double_click` | Double-click | Interaction |
| `screenshot` | Capture screen | Data |
| `log` | Log message | Debugging |
| `if` | Conditional block | Control Flow |
| `loop` | Repeat actions | Control Flow |
| `call` | Call other task | Composition |
| `foreach` | Iterate collection | Control Flow |
| `while` | Loop while condition | Control Flow |
| `retry` | Retry with backoff | Control Flow |
| `try` | Error handling | Control Flow |
| `parallel` | Execute actions concurrently ⚡NEW | Control Flow |
| `execute` | Execute JavaScript | Advanced |

---

## Complete Condition Reference

| Condition | Type | Example |
|-----------|------|---------|
| `element_visible` | Element State | `selector: "#modal"` |
| `element_exists` | Element State | `selector: "#result"` |
| `variable_equals` | Variable | `name: "status", value: "active"` |
| `text_matches` | Regex ⚡NEW | `selector: "#msg", pattern: "^Hello"` |
| `variable_matches` | Regex ⚡NEW | `name: "email", pattern: ".*@.*\\..*"` |
| `numeric_greater_than` | Numeric ⚡NEW | `name: "count", value: 5` |
| `numeric_less_than` | Numeric ⚡NEW | `name: "price", value: 100.0` |
| `numeric_range` | Numeric ⚡NEW | `name: "score", min: 0, max: 100` |
| `date_before` | Date/Time ⚡NEW | `name: "expiry", date: "2026-12-31"` |
| `date_after` | Date/Time ⚡NEW | `name: "created", date: "2026-01-01"` |
| `array_contains` | Array ⚡NEW | `name: "tags", value: "urgent"` |
| `array_length` | Array ⚡NEW | `name: "items", min: 1, max: 10` |

---

## Statistics

| Metric | Value | Change |
|--------|-------|--------|
| **Version** | 0.1.1 | +0.0.1 |
| **Total Tests** | 2196 | +79 |
| **DSL Actions** | 23 | +5 |
| **Condition Types** | 12 | +8 |
| **Commits** | 55+ | +5 |

---

## Breaking Changes

None. All changes are backward compatible.

---

## Migration from v0.1.0

No migration needed. All v0.1.0 tasks continue to work unchanged.

New features are opt-in via:
- Enhanced conditions in `if`/`while` actions
- Debug mode via `with_debug_mode(true)`
- Caching enabled by default (can be disabled)

---

## What's Next (v0.2.0+)

| Feature | Priority | Status |
|---------|----------|--------|
| ~~Parallel action execution~~ | High | ✅ Complete (v0.1.1) |
| Plugin system | Medium | Planned |
| More built-in actions | Medium | Planned |
| Visual task builder | Low | Planned |
| Task marketplace | Low | Planned |

---

*Released with 🦀 Rust 2021 edition*
