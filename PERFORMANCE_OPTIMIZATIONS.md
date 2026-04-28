# Performance Optimization Checklist

## Phase 2: Code-Level Optimizations

- [ ] **2.1 Metrics Lock Contention**
  - [ ] Replace `RwLock<VecDeque>` with lock-free ring buffer
  - [ ] Add `crossbeam` dependency
  - [ ] Implement `ArrayQueue<TaskMetrics>` for history
  - [ ] Verify readers are never blocked

- [ ] **2.2 Clone Reduction (twitteractivity.rs)**
  - [ ] Audit `task_name.clone()` calls
  - [ ] Audit `session_id.clone()` calls
  - [ ] Replace `String` with `Arc<str>` in `TaskMetrics`
  - [ ] Update `task_breakdown` and `session_breakdown` to use `Arc<str>` keys

- [ ] **2.3 Data Structure Optimization**
  - [ ] Add `rustc-hash` dependency for `FxHashMap`
  - [ ] Replace `BTreeMap<TaskErrorKind, usize>` in `failure_breakdown`
  - [ ] Replace `BTreeMap<String, OutcomeBreakdown>` in `task_breakdown`
  - [ ] Replace `BTreeMap<String, OutcomeBreakdown>` in `session_breakdown`
  - [ ] Benchmark lookup performance before/after

## Phase 4: Memory Optimizations

- [ ] **4.1 String Interning**
  - [ ] Create `ERROR_KIND_STRINGS` static map
  - [ ] Pre-compute all `TaskErrorKind` string representations
  - [ ] Update `get_stats()` to use interned strings
  - [ ] Eliminate `format!("{:?}", kind)` allocations

- [ ] **4.2 JSON Zero-Copy**
  - [ ] Identify JSON parsing hot paths (LLM responses, tweet data)
  - [ ] Define borrowed structs with `&'a str` fields
  - [ ] Use `#[serde(borrow)]` attribute
  - [ ] Verify lifetime constraints are met

- [ ] **4.3 Pre-Allocated Vectors**
  - [ ] Add `Vec::with_capacity()` in `get_stats()` for `failure_breakdown`
  - [ ] Add `Vec::with_capacity()` in `get_stats()` for `task_breakdown`
  - [ ] Add `Vec::with_capacity()` in `get_stats()` for `session_breakdown`
  - [ ] Audit other collection allocations for size hinting

---

## Detailed Implementation Guide

### 2.1 Metrics Lock Contention

**Problem:** `RwLock<VecDeque>` blocks all readers during writes

**Current Code:**
```rust
// src/metrics.rs
task_history: Arc<RwLock<VecDeque<TaskMetrics>>>,

// In task_completed():
let mut history = self.task_history.write();  // Blocks readers!
history.push_back(metrics);
```

**Solution:**
```rust
use crossbeam::queue::ArrayQueue;

task_history: ArrayQueue<TaskMetrics>,  // Lock-free, fixed capacity

// In task_completed():
self.task_history.push(metrics);  // Non-blocking

// In get_stats():
let snapshot: Vec<_> = self.task_history.iter().cloned().collect();
```

**Dependencies:**
```toml
[dependencies]
crossbeam = "0.8"
```

---

### 2.2 Clone Reduction

**Problem:** String cloning on every task completion

**Current Code:**
```rust
// src/metrics.rs:225-227
task_breakdown
    .entry(metrics.task_name.clone())  // Allocates!
    .or_default()
    .record(metrics.status);
```

**Solution:**
```rust
pub struct TaskMetrics {
    pub task_name: Arc<str>,      // Was: String
    pub session_id: Arc<str>,    // Was: String
    // ...
}

// Clone is now O(1):
task_breakdown.entry(Arc::clone(&metrics.task_name))
```

**Files to Modify:**
- `src/metrics.rs` - `TaskMetrics` struct
- `src/task/twitteractivity.rs` - Metrics creation
- Any other task files creating `TaskMetrics`

---

### 2.3 Data Structure Optimization

**Problem:** `BTreeMap` provides ordering we don't need

**Current Code:**
```rust
// src/metrics.rs
task_breakdown: Arc<RwLock<BTreeMap<String, OutcomeBreakdown>>>,
```

**Solution:**
```rust
use rustc_hash::FxHashMap;

task_breakdown: Arc<RwLock<FxHashMap<String, OutcomeBreakdown>>>,
```

**Dependencies:**
```toml
[dependencies]
rustc-hash = "2"
```

**Performance Gain:**
- BTreeMap lookup: O(log n) ~ 5-10 comparisons
- FxHashMap lookup: O(1) ~ 1-2 operations

---

### 4.1 String Interning

**Problem:** Dynamic string formatting in hot path

**Current Code:**
```rust
// src/metrics.rs:295
.map(|(kind, count)| (format!("{:?}", kind), *count))  // Allocates!
```

**Solution:**
```rust
use once_cell::sync::Lazy;

static ERROR_KIND_STRINGS: Lazy<HashMap<TaskErrorKind, &'static str>> = 
    Lazy::new(|| {
        let mut m = HashMap::new();
        m.insert(TaskErrorKind::Timeout, "timeout");
        m.insert(TaskErrorKind::Navigation, "navigation");
        m.insert(TaskErrorKind::ElementNotFound, "element_not_found");
        m.insert(TaskErrorKind::ParseError, "parse_error");
        m.insert(TaskErrorKind::NetworkError, "network_error");
        m.insert(TaskErrorKind::ExecutionError, "execution_error");
        m.insert(TaskErrorKind::Unknown, "unknown");
        m
    });

// In get_stats():
let key = ERROR_KIND_STRINGS.get(&kind).unwrap_or("unknown");
```

---

### 4.2 JSON Zero-Copy

**Problem:** Owned strings for JSON fields

**Current Code:**
```rust
let tweet: Value = serde_json::from_str(&body)?;
let text = tweet["text"].as_str().unwrap().to_string();  // Copies!
```

**Solution:**
```rust
#[derive(Deserialize)]
struct Tweet<'a> {
    #[serde(borrow)]
    text: &'a str,
    #[serde(borrow)]
    author: &'a str,
    #[serde(borrow)]
    id: &'a str,
}

let tweet: Tweet = serde_json::from_str(&body)?;
// tweet.text points INTO body - zero copy
```

**Limitations:**
- Only works when data lifetime >= output lifetime
- Not suitable for stored/returned data
- Best for intermediate processing

---

### 4.3 Pre-Allocated Vectors

**Problem:** Vec growth reallocations

**Current Code:**
```rust
let failure_breakdown = self
    .failure_breakdown
    .read()
    .iter()
    .map(|(kind, count)| (key, *count))
    .collect();  // Grows: 0, 4, 8, 16...
```

**Solution:**
```rust
let breakdown = self.failure_breakdown.read();
let mut failure_breakdown = Vec::with_capacity(breakdown.len());
for (kind, count) in breakdown.iter() {
    failure_breakdown.push((key, *count));
}
```

**Impact:**
- Single allocation vs multiple reallocations
- ~10-20% faster for large collections
- Predictable memory usage

---

## Implementation Priority

| # | Optimization | Effort | Impact | Risk |
|---|--------------|--------|--------|------|
| 1 | 4.3 Pre-allocated vectors | Low | Medium | Low |
| 2 | 2.3 FxHashMap | Low | Medium | Low |
| 3 | 4.1 String interning | Low | Medium | Low |
| 4 | 2.2 Clone reduction | Medium | High | Medium |
| 5 | 2.1 Lock-free ring buffer | High | High | Medium |
| 6 | 4.2 JSON zero-copy | Medium | Medium | Medium |

## Testing Checklist

Before each optimization:
- [ ] `cargo test --lib` passes
- [ ] `cargo test --test integration` passes
- [ ] `cargo clippy` shows no new warnings
- [ ] Metrics behavior verified (if applicable)

After each optimization:
- [ ] Document change in commit message
- [ ] Run benchmark if available
- [ ] Measure compile time impact

## Dependencies to Add

```toml
[dependencies]
# For 2.1 Lock-free ring buffer
crossbeam = "0.8"

# For 2.3 FxHashMap
rustc-hash = "2"

# 4.1 String interning uses existing:
# - once_cell (already in deps)
```
