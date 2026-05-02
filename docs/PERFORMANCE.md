# Performance Baseline and Benchmarks

This document tracks performance baselines for critical hot paths in the auto-rust codebase. Use benchmarks to detect regressions and validate optimizations.

## Running Benchmarks

```bash
# Run all benchmarks
cargo criterion

# Run specific benchmark
cargo bench --bench trajectory
cargo bench --bench accessibility_locator
cargo bench --bench predictive_scorer

# Run with HTML report
cargo criterion --output-format verbose
```

Benchmark results are saved to `target/criterion/` with interactive HTML reports.

---

## Trajectory Generation (`benches/trajectory.rs`)

Mouse trajectory generation for human-like cursor movement.

### Benchmarked Functions

| Function | Description | Typical Use |
|----------|-------------|-------------|
| `generate_bezier_curve()` | Cubic Bezier with Gaussian control points | Standard mouse movements |
| `generate_arc_curve()` | Quadratic arc with curvature control | Curved movements |
| `generate_muscle_path()` | Jitter simulation for human-like paths | Precision movements |

### Expected Performance (Baseline)

| Test | Target Time | Notes |
|------|-------------|-------|
| Bezier (10 steps) | < 5 µs | Short animations |
| Bezier (100 steps) | < 50 µs | Standard movements |
| Bezier (200 steps) | < 100 µs | Long, smooth paths |
| Arc (0.3 curvature) | < 30 µs | Typical curve |
| Muscle (short) | < 20 µs | Small movements |
| Muscle (medium) | < 50 µs | Standard movements |
| Muscle (long) | < 100 µs | Cross-screen movements |

### Performance Characteristics

- **Time complexity**: O(n) where n = number of steps
- **Memory**: Allocates Vec with capacity = steps + 1
- **Hot path**: Called on every cursor movement (~50-200 times per task)

---

## Accessibility Locator Parsing (`benches/accessibility_locator.rs`)

Selector parsing for accessibility-first element location.

### Benchmarked Functions

| Function | Description | Typical Use |
|----------|-------------|-------------|
| `parse_selector_input()` | Parse CSS or accessibility locator | Every element interaction |

### Selector Types Benchmarked

1. **CSS Selectors**: Standard CSS like `#id`, `.class`, `[attr=value]`
2. **Accessibility Locators**: `role=button[name='Save'][scope='main']`
3. **Complex Selectors**: Nested/deep selectors

### Expected Performance (Baseline)

| Test | Target Time | Notes |
|------|-------------|-------|
| Simple CSS (#button) | < 1 µs | Most common case |
| CSS with attributes | < 2 µs | Standard selectors |
| Deep CSS (5 levels) | < 3 µs | Complex DOM queries |
| Simple accessibility | < 5 µs | Role + name only |
| Full accessibility | < 10 µs | Role + name + scope + match |
| Long accessibility | < 15 µs | Very long strings |
| Error cases | < 2 µs | Fast validation failure |
| Batch 5 selectors | < 10 µs | Common batch size |

### Performance Characteristics

- **Time complexity**: O(n) where n = input length
- **Memory**: Minimal allocations (usually 1-2 per parse)
- **Hot path**: Called on every element lookup (~100-500 times per task)

---

## Predictive Engagement Scoring (`benches/predictive_scorer.rs`)

ML-based engagement prediction for Twitter automation.

### Benchmarked Functions

| Function | Description | Typical Use |
|----------|-------------|-------------|
| `predict_engagement()` | Calculate engagement probability | Every candidate tweet |
| `extract_text_features()` | Parse tweet text features | Feature extraction |
| `extract_user_features()` | Parse user profile features | Feature extraction |
| `recommend_action()` | Select best action from candidates | Action selection |

### Expected Performance (Baseline)

| Test | Target Time | Notes |
|------|-------------|-------|
| Short tweet | < 20 µs | < 50 chars |
| Medium tweet | < 30 µs | 50-150 chars |
| Long tweet | < 50 µs | > 150 chars |
| Feature extraction (short) | < 5 µs | Text features only |
| Feature extraction (long) | < 15 µs | Full feature set |
| Action rec (2 candidates) | < 10 µs | Like/Skip decision |
| Action rec (5 candidates) | < 15 µs | Full action set |
| Batch 10 predictions | < 300 µs | Typical feed processing |
| Batch 50 predictions | < 1500 µs | Large feed processing |

### Performance Characteristics

- **Time complexity**: O(n) where n = tweet text length
- **Memory**: Minimal allocations (fixed-size feature vectors)
- **Hot path**: Called for every tweet candidate (~50-200 per feed scroll)

---

## Interpreting Results

### Regression Detection

A performance regression is suspected when:

1. **> 20% slower** than baseline - investigate
2. **> 50% slower** than baseline - must fix before release
3. **> 100% slower** than baseline - critical regression

### Factors Affecting Benchmarks

- **CPU frequency scaling**: Run with `performance` governor
- **Background processes**: Close unnecessary applications
- **Thermal throttling**: Ensure adequate cooling
- **Memory pressure**: Free RAM before running

### Statistical Significance

Criterion uses robust statistical analysis:

- Minimum 100 iterations per benchmark
- Outlier detection and removal
- Confidence intervals for timing
- Comparison against previous runs

---

## Optimization Guidelines

### Trajectory Generation

✅ **DO:**
- Pre-allocate Vec with exact capacity
- Use stack-allocated arrays for small step counts
- Cache control point calculations for similar paths

❌ **DON'T:**
- Allocate per-point (use iterator chains)
- Use expensive RNG in hot path (pre-generate jitter)
- Compute distance repeatedly (cache it)

### Selector Parsing

✅ **DO:**
- Use fast-path for CSS selectors (simple prefix check)
- Avoid regex (use manual parsing)
- Cache parsed selectors for reuse

❌ **DON'T:**
- Clone strings unnecessarily (use &str views)
- Parse the same selector multiple times
- Use heavy validation in hot path

### Engagement Prediction

✅ **DO:**
- Pre-allocate feature vectors
- Use SIMD for dot products (if available)
- Cache user profile features

❌ **DON'T:**
- Allocate per-feature (use arrays)
- Compute features that don't change (cache them)
- Use String for numeric features

---

## Continuous Benchmarking

While CI benchmarks are disabled (noisy environment), run benchmarks:

1. **Before major releases** - establish new baseline
2. **After optimization PRs** - validate improvements
3. **When adding hot path code** - ensure no regression
4. **Weekly during development** - catch gradual degradation

Store historical results in `target/criterion/` and compare visually via HTML reports.

---

## Current Baseline (as of v0.0.4)

Run `cargo criterion` to generate current baseline for your hardware.

Last manual run: 2026-05-02
Hardware: Windows, Intel/AMD x64
Rust: 1.78

| Benchmark Suite | Status | Notes |
|-----------------|--------|-------|
| trajectory | ✅ Ready | All paths covered |
| accessibility_locator | ✅ Ready | CSS + accessibility locators |
| predictive_scorer | ✅ Ready | Feature extraction + prediction |

---

## Troubleshooting

### Benchmarks fail to compile

```bash
# Ensure criterion is installed
cargo add --dev criterion

# Check benchmark harness is disabled in Cargo.toml
# [[bench]]
# name = "trajectory"
# harness = false
```

### Results are inconsistent

```bash
# Disable CPU frequency scaling (Linux)
cpufreq-set -g performance

# Pin to specific CPU cores
cargo criterion --bind-to-cores 0-3

# Increase sample size for noisy benchmarks
cargo criterion --sample-size 500
```

### Criterion reports no change

Check that `target/criterion/` exists and contains previous run data. Delete directory to reset baseline.

---

## Related Documentation

- [ARCHITECTURE.md](./ARCHITECTURE.md) - System design and hot paths
- [ONBOARDING.md](./ONBOARDING.md) - Getting started guide
- `journal.md` - Development history and decisions
