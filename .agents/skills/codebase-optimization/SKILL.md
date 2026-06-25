# codebase-optimization

> last audited 26-06-26 by docs-auditor

Expert skill for **optimizing Rust code** in the auto-rust codebase. Covers profiling, benchmarking, linting, build tuning, memory, async performance, and dependency hygiene. Designed to produce reliable, measurable improvements with minimal risk.

## When to use

- User says "this module is slow, optimize it"
- User asks "how can we speed up the build?"
- User wants to reduce memory usage or allocations
- User wants to audit dependencies for bloat or duplicates
- User wants to add or improve benchmarks
- User asks "make the code more idiomatic/efficient"
- User wants to profile a hot path

Do **not** use this skill when the goal is to add features, fix bugs, or refactor for readability alone — those belong to other skills.

## Guiding principles

1. **Measure first, optimize second.** Never optimize without a benchmark or profile. Use data, not hunches.
2. **Profile at the right level of granularity.** Start with wall-clock time, then drill into CPU profiles, allocation counts, and hot loops.
3. **One change at a time.** Change one thing, benchmark, compare. Multiple changes in one shot obscure cause and effect.
4. **Preserve correctness.** Every optimization must pass `cargo test --lib` before and after. Add regression tests for the optimized path.
5. **Respect the project's safety guarantees.** The orchestrator relies on `catch_unwind` for task isolation — don't change panic behavior without understanding the implications.

---

## 1. Profiling — find the real bottleneck

### Quick profile (wall-clock)

```powershell
# Time a specific module's tests
Measure-Command { cargo test --lib <module_name> }

# Time the full test suite (baseline)
Measure-Command { cargo test --lib }

# Time a specific binary's startup
Measure-Command { cargo run --bin auto -- --help }
```

### Criterion benchmarks (precise, comparable)

The project uses **Criterion.rs** for benchmarks in `src/benchmarks/`:

| Benchmark | Command | Measures |
|---|---|---|
| Trajectory | `cargo bench --bench trajectory` | Bezier/arc/muscle path generation speed (10-200 steps) |
| Accessibility Locator | `cargo bench --bench accessibility_locator --features accessibility-locator` | CSS selector vs accessibility locator parse speed |
| Predictive Scorer | `cargo bench --bench predictive_scorer` | Engagement prediction throughput (single + batch) |

**To add a new benchmark:**
```rust
// src/benchmarks/my_module.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_my_fn(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_module");
    group.bench_function("input_size_small", |b| {
        b.iter(|| black_box(my_function(black_box(small_input))))
    });
    group.bench_function("input_size_large", |b| {
        b.iter(|| black_box(my_function(black_box(large_input))))
    });
    group.finish();
}

criterion_group!(benches, benchmark_my_fn);
criterion_main!(benches);
```

Then register in `Cargo.toml`:
```toml
[[bench]]
name = "my_module"
path = "src/benchmarks/my_module.rs"
harness = false
```

### Flamegraph / CPU profiling

For deeper CPU profiling (Windows):
```powershell
# Install WPR (Windows Performance Recorder) from Windows SDK or use:
cargo install flamegraph

# CPU flamegraph of a specific test
cargo flamegraph --test --lib -- <test_name>

# CPU flamegraph of the auto binary
cargo flamegraph --bin auto -- --help
```

### Allocation profiling

```powershell
# Count allocations with dhat (requires nightly)
# 1. Add dhat as a dev-dependency
# 2. Add `#[global_allocator] static ALLOC: dhat::Alloc = dhat::Alloc;` to the test
# 3. Run: cargo test --lib <test>
# 4. Read dhat-heap.json or stderr for allocation stats

# Alternative: use `--profile=release` build and measure RSS
cargo build --profile release --bin auto
# Check binary size
ls -la target/release/auto.exe
```

---

## 2. Linting — enforce performance patterns

### Run the full lint suite

```powershell
# Fast check (what you should run during iteration)
.\check-fast.ps1

# Full check (complete CI pipeline — run before pushing)
.\check.ps1

# Standalone clippy with performance lints
cargo clippy --all-targets --all-features
```

### Project-enforced clippy lints

These are enabled at the workspace level in `Cargo.toml`:
```toml
[lints.clippy]
inefficient_to_string = "warn"    # Catches .to_string() on &str
unnecessary_to_owned = "warn"     # Catches unnecessary .to_owned()/.clone()
```

### Extra clippy pedantic lints to consider for optimization

When investigating a performance issue, run with these:

```powershell
# Ban .unwrap() in production code
cargo clippy --lib -- -D warnings -D clippy::unwrap_used

# Ban .expect() in production code
cargo clippy --lib -- -D warnings -D clippy::expect_used

# Ban unwrap/expect in binary targets
cargo clippy --bins -- -D warnings -D clippy::unwrap_used -D clippy::expect_used

# Pedantic audit (all warnings, not denied)
cargo clippy -- -W clippy::pedantic

# Performance-focused subset of pedantic
cargo clippy -- -W clippy::large_enum_variant `
    -W clippy::large_stack_arrays `
    -W clippy::box_collection `
    -W clippy::redundant_allocation `
    -W clippy::rc_buffer `
    -W clippy::vec_box `
    -W clippy::option_option `
    -W clippy::type_complexity
```

### Apply clippy auto-fixes

```powershell
# Auto-fix what clippy can fix automatically
cargo clippy --fix --all-targets --all-features

# Always format after auto-fix
cargo fmt --all
```

---

## 3. Common optimization patterns in this codebase

### 3a. Reduce allocations (the #1 Rust performance win)

**Bad — allocates per iteration:**
```rust
for item in items {
    let s = format!("prefix_{}", item);  // allocates String
    process(&s);
}
```

**Good — reuse buffer:**
```rust
let mut s = String::with_capacity(64);
for item in items {
    s.clear();
    s.push_str("prefix_");
    s.push_str(item);
    process(&s);
}
```

**Bad — unnecessary clone:**
```rust
fn process(data: &[u8]) {
    let owned = data.to_vec();  // allocates! Use &[u8]
    // ...
}
```

**Checklist for reducing allocations:**
- [ ] Prefer `&str` over `String` in function parameters
- [ ] Prefer `&[T]` over `Vec<T>` in function parameters
- [ ] Use `Cow<'_, str>` when you sometimes need ownership
- [ ] Use `String::with_capacity()` when building strings
- [ ] Use `Vec::with_capacity()` when building vectors
- [ ] Reuse buffers across loop iterations (`clear()` + `extend()`)
- [ ] Check if `clone()` is necessary or if a borrow suffices

### 3b. Async — minimize await points and contention

**This codebase uses:**
- `tokio` with `rt-multi-thread` runtime
- `chromiumoxide` for CDP browser communication
- `futures` for combinators
- Channels via `tokio::sync` for session management

**Optimization rules for async code:**
- Batch small operations — one `evaluate()` with a compound JS expression is faster than 10 separate `evaluate()` calls
- Use `tokio::sync::Semaphore` for rate limiting (already used for session pool)
- Prefer `tokio::sync::oneshot` over `mpsc` for one-shot responses
- Avoid `tokio::spawn` for tiny tasks — the overhead exceeds the benefit
- Use `stream::Buffered` or `stream::BufferUnordered` for concurrent I/O with backpressure
- Use `futures::join!` for independent branches, never `join_all` on hot paths

**Checklist for async optimization:**
- [ ] Are there too many tiny `async` functions that could be sync?
- [ ] Are CDP calls batched where possible (single JS expression vs multiple queries)?
- [ ] Is `select!` used judiciously (not polling many futures per iteration)?
- [ ] Are channels bounded to prevent unbounded memory growth?
- [ ] Is the runtime configured for your workload (multi-thread vs current-thread)?

### 3c. Cache hot computations

**DOM query results** — the `SelectorCache` in `src/task/dsl/cache.rs` caches DOM query results (exists, visible, text, count) with LRU eviction and TTL. When performing repeated selector queries, always route through the cache.

```rust
// Good — uses the existing SelectorCache
use crate::task::dsl::cache::{SelectorCache, SelectorCacheEntry};
let mut cache = SelectorCache::new();
let entry = cache.get(selector).unwrap_or_else(|| {
    let result = query_element(selector);
    cache.insert(selector.to_string(), SelectorCacheEntry::new(
        result.exists, result.visible, result.text, result.count,
    ));
    cache.get(selector).unwrap()
});
```

**Predictive scoring** — `PredictiveEngagementScorer` in `src/adaptive/` creates fresh instances. If called in a loop, consider caching the scorer.

### 3d. String operations

**Formatting** — the workspace has `inefficient_to_string = "warn"`. Watch for:
- `some_string.to_string()` → use `some_string.clone()` or pass `&str`
- `format!("{}", x)` → use `x.to_string()` or `x.to_owned()`
- `format!("{x}")` is fine in Rust 2021 edition — no allocation overhead vs alternatives

**Selectors** — the project builds many selector strings dynamically. Consider:
```rust
// Bad: allocates per call
let selector = format!("role=button[name='Follow @{}']", username);

// Good: reuse a format string with known capacity
fn follow_selector(username: &str) -> String {
    // prefix = "role=button[name='Follow @']" 
    let mut s = String::with_capacity(32 + username.len());
    s.push_str("role=button[name='Follow @");
    s.push_str(username);
    s.push_str("']");
    s
}
```

### 3e. Serialization

This codebase uses `serde_json`, `serde_yaml`, and `toml`. Optimization tips:
- Use `serde_json::json!()` macro for construction (compile-time, no intermediate values)
- Use `serde_json::Value` sparingly — prefer strongly-typed structs with `#[derive(Deserialize)]`
- For hot deserialization paths, use `serde_json::from_reader` (streaming) not `from_str`
- `serde_yaml` is known to be slower than JSON — avoid it on hot paths

---

## 4. Build profile tuning

The project has three profiles in `Cargo.toml`:

| Profile | Use case | Key settings |
|---|---|---|
| `dev` (default) | Iteration / TDD | `opt-level = 0`, `incremental = true`, `codegen-units = 256`, `overflow-checks = false` |
| `release` | Production / benchmarks | `opt-level = 3`, `lto = "thin"`, `codegen-units = 8`, `strip = "symbols"`, `overflow-checks = true` |
| `tdd` (custom) | Max compile speed | Inherits `dev`, strips debuginfo |

### Optimize your build

```powershell
# Fastest possible compilation (for quick iteration)
cargo build --profile tdd

# Check compilation only (no codegen)
cargo check

# Release build with optimizations (for benchmarking)
cargo build --release

# Run release tests
cargo test --profile release
```

**Note on `overflow-checks`:** `dev` disables overflow checks for speed, but `release` enables them. If you hit overflow bugs only in release, add `overflow-checks = true` to your test config with `#[cfg(test)]` gates.

### Build time checklist

- [ ] Are you using `cargo check` instead of `cargo build` for feedback?
- [ ] Are you using `codegen-units = 256` in dev (already set)?
- [ ] Are proc-macros (like `serde` derives) causing slow compiles? Consider `serde = { ..., features = ["derive"] }` (already set)
- [ ] Is incremental compilation enabled (already set)?
- [ ] For CI, use `--profile release` with `--timings` to find bottlenecks

---

## 5. Memory optimization

### Current memory architecture

- **Session pool** — bounded via `max_workers_per_session` config
- **SelectorCache** — LRU with capacity limit
- **Task context** — per-session, short-lived
- **Logging** — `env_logger` with async, unbounded by default

### Memory optimization checklist

- [ ] **Remove redundant cloning** — search for `.clone()` calls and check if the clone is necessary. Many `clone()` calls in the codebase are on `Arc` types which are cheap, but `String`/`Vec` clones are expensive.
- [ ] **Use `Arc<str>`** instead of `Arc<String>` for shared string data (saves an indirection)
- [ ] **Check enum sizes** — `clippy::large_enum_variant` can spot enums where one variant is much larger than others
- [ ] **Use `Box<[T]>`** instead of `Vec<T>` for owned slices that never grow
- [ ] **Use `SmallVec` or `arrayvec`** for small collections with a known upper bound
- [ ] **Check for `Arc<Mutex<T>>` patterns** — if the mutex is heavily contended, consider `tokio::sync::RwLock` or sharding
- [ ] **Monitor with process explorer** — track memory usage of `auto.exe` over time to detect leaks
- [ ] **Check for unbounded channel growth** — all `tokio::sync::mpsc` channels should have a reasonable buffer bound

### Selector string optimization (specific to this codebase)

The codebase builds many locator strings dynamically. These are short-lived (used once, then dropped), so allocation pressure from formatting is the concern:

```rust
// Current pattern (acceptable for most cases)
let selector = format!("role=button[name='Follow @{}']", username);

// Optimization for hot paths: pre-build common patterns
const FOLLOW_PREFIX: &str = "role=button[name='Follow @";
fn follow_selector_fast(username: &str) -> String {
    let mut s = String::with_capacity(FOLLOW_PREFIX.len() + username.len() + 2); // + "']"
    s.push_str(FOLLOW_PREFIX);
    s.push_str(username);
    s.push_str("']");
    s
}
```

---

## 6. Dependency optimization

### Audit dependencies

```powershell
# Check for duplicate versions
cargo deny check bans

# Check for security advisories
cargo deny check advisories

# Check for licensing issues
cargo deny check licenses

# Show dependency tree for a specific crate
cargo tree -i <crate_name>

# Find duplicate dependencies
cargo tree -d
```

### Current known issues (from deny.toml)

The project already tracks:
- `serde_yml 0.0.13` — has an unsoundness advisory (`RUSTSEC-2025-0068`), migration tracked in `docs/TODO.md`
- `async-std 1.13.2` — marked unmaintained (`RUSTSEC-2025-0052`), transitive dep only (all async uses tokio directly)

### Dependency trimming checklist

- [ ] Can any `dep:` features be removed? (check `cargo tree -e features`)
- [ ] Are there multiple versions of the same crate? (`cargo tree -d`)
- [ ] Can any optional dependencies be moved to `dev-dependencies`?
- [ ] Are there unused dependencies? (install `cargo-udeps` and run `cargo +nightly udeps`)
- [ ] Are there feature flags pulling in unnecessary transitive deps? (check `cargo tree -i <crate> --all-features`)
- [ ] Is the `tokio` feature set minimal? (currently: `rt-multi-thread`, `time`, `sync`, `fs`, `signal`, `macros`)

---

## 7. Optimization workflow — step by step

### Step 1: Identify the hot path

```powershell
# Run the relevant benchmark
cargo bench --bench trajectory

# Or time a specific test
Measure-Command { cargo test --lib twitterfollow -- 2>&1 > $null }
```

### Step 2: Read the code being optimized

Use `file-picker` to find the relevant module. Read the full file before writing any optimization. Understand:
- What data structures are used
- Where allocations happen
- Where async boundaries are
- What the existing test coverage looks like

### Step 3: Apply the change

Apply one optimization at a time following the patterns in Section 3.

### Step 4: Measure the improvement

```powershell
# Before: run the benchmark (note the time)
cargo bench --bench trajectory

# After: make change, run again
cargo bench --bench trajectory
```

Criterion automatically compares against previous runs in `target/criterion/`. The report shows:
- % change in throughput
- Statistical significance (no overlap in confidence intervals = real improvement)

### Step 5: Verify correctness

```powershell
# Full regression check
cargo check
cargo test --lib
cargo fmt --all --check
```

### Step 6: Commit with evidence

```powershell
git commit -m "perf: optimize <module> by <N>%

Benchmark results:
- Before: 123.45 µs
- After:   67.89 µs  (1.82x faster)

Derived from <benchmark_name> criterion bench."
```

---

## 8. Project-specific optimization targets

| Module | Current benchmarks | Known optimization potential |
|---|---|---|
| Trajectory (`src/utils/mouse/trajectory.rs`) | ✅ Bezier, arc, muscle path at 10-200 steps | Arc/muscle are fast, Bezier at 200 steps may benefit from pre-computed control points |
| Accessibility locator (`src/utils/accessibility_locator.rs`) | ✅ CSS + accessibility parse throughput | Already has SelectorCache — hot path is cache lookup, not parse |
| Predictive scorer (`src/adaptive/predictive_scorer.rs`) | ✅ Single + batch prediction throughput | Batch predictions are untuned for parallelism |
| Task dispatcher (`src/task/mod.rs`) | ❌ No benchmark | Match dispatch + timeout wrapping — likely fine, but worth measuring for high-concurrency scenarios |
| DSL executor (`src/task/dsl/executor.rs`) | ❌ No benchmark | Action dispatch overhead — may benefit from action type specialization |
| Session pool (`src/session/pool.rs`) | ❌ No benchmark | Pool acquire/release latency under load |

### Quick wins (least risk, highest impact)

1. **Add `#[inline]` to small hot functions** — especially `Point::new()`, geometry helpers, and timing utilities
2. **Pre-size collections** — `Vec::with_capacity()` in loops that build selector candidate lists
3. **Remove redundant `clone()` calls** — especially in selector builder helpers and test utilities
4. **Replace `format!` in hot selector paths** with manual `String::with_capacity()` + `push_str()`
5. **Batch CDP evaluate calls** — combine multiple JS queries into one expression

---

## 9. The optimization checklist (run this for every optimization)

- [ ] ✅ **Measured baseline** (benchmark or timing before change)
- [ ] ✅ **One change at a time** (not mixing optimizations)
- [ ] ✅ **Tests pass before + after** (`cargo test --lib`)
- [ ] ✅ **No new clippy warnings** (`cargo clippy --all-targets --all-features`)
- [ ] ✅ **Format preserved** (`cargo fmt --all --check`)
- [ ] ✅ **Benchmark shows improvement** (>5% to be meaningful, or justified by readability/maintainability tradeoff)
- [ ] ✅ **No safety compromise** (panic behavior, error handling, overflow checks preserved)
- [ ] ✅ **Commit message includes benchmark evidence**

---

## Common pitfalls

1. **Optimizing the wrong thing.** Profile first. A 50% improvement in a function that takes 0.1% of runtime is meaningless.
2. **Premature `unsafe`.** If you need `unsafe` for performance, wrap it in a safe API, justify with a benchmark, and add a safety comment. Prefer safe alternatives first.
3. **Ignoring `cargo fmt`.** Optimized code often gets less readable — `cargo fmt` at least ensures consistency.
4. **Removing error handling for speed.** Never trade correctness for speed. Use `anyhow::Result` consistently.
5. **Micro-optimizing in a non-hot path.** The `#[inline]` hint is noise in cold code. Only apply to functions proven hot by profiling.
6. **Changing panic behavior.** The orchestrator uses `catch_unwind` for task isolation — never switch to `panic=abort` without deep analysis.
7. **Oversized generics / monomorphization.** Generic functions with many type parameters expand code size. If a function is called with 5+ concrete types, consider `Box<dyn Trait>` or `enum dispatch`.
8. **Too many `async` boundaries.** If a function does minimal I/O, make it synchronous. Each `await` point adds state machine overhead.
