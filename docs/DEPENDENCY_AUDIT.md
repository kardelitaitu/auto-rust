# Dependency Audit Report

**Date:** 2026-05-01  
**Auditor:** Cascade AI  
**Scope:** `Cargo.toml` - 39 dependencies reviewed

## Executive Summary

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Direct Dependencies | 39 | 37 | -2 |
| Unused Dependencies | 3 identified | 0 removed | ✓ |
| Redundant Patterns | 1 (`lazy_static` + `once_cell`) | Consolidated | ✓ |

## Dependencies Removed

### 1. `urlencoding` (v2.1.3)
- **Status:** UNUSED
- **References in code:** 0
- **Action:** Removed from `Cargo.toml`
- **Impact:** Reduced attack surface, cleaner dependency tree

### 2. `humantime` (v2)
- **Status:** UNUSED
- **References in code:** 0
- **Action:** Removed from `Cargo.toml`
- **Impact:** Smaller dependency tree

### 3. `lazy_static` (v1.4)
- **Status:** CONSOLIDATED
- **References in code:** 1 (`twitteractivity_sentiment_llm.rs`)
- **Action:** Migrated to `once_cell::sync::Lazy`
- **Rationale:** `once_cell` is the modern standard, already used elsewhere (6 references)
- **Impact:** Single initialization pattern across codebase

## Migration Details

### `lazy_static` → `once_cell` Migration

**File:** `src/utils/twitter/twitteractivity_sentiment_llm.rs`

**Before:**
```rust
use std::collections::HashMap;

lazy_static::lazy_static! {
    static ref SENTIMENT_CACHE: SentimentCache = Arc::new(RwLock::new(HashMap::with_capacity(100)));
}
```

**After:**
```rust
use once_cell::sync::Lazy;
use std::collections::HashMap;

static SENTIMENT_CACHE: Lazy<SentimentCache> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::with_capacity(100)))
});
```

**Benefits:**
- Consistent with existing `once_cell` usage (6 other locations)
- No macro syntax, cleaner code
- Standard library future compatibility (`std::sync::LazyLock` coming in Rust 1.80+)

## Dependencies Verified (Kept)

The following dependencies were verified as actively used and retained:

| Dependency | Usage Count | Primary Location |
|------------|-------------|------------------|
| `do_over` | 2 | `internal/circuit_breaker.rs` |
| `webp` | 8 | `runtime/task_context.rs` (screenshots) |
| `enigo` | 5 | `config.rs` (native input backend) |
| `dashmap` | 7 | `session/mod.rs`, `state/overlay.rs` |
| `regex` | 2 | `utils/twitter/` |
| `rustc_hash` | 1 | `metrics.rs` |
| `log` | 129 | Across 45 files |
| `once_cell` | 6 (now 7) | Multiple modules |

## Recommendations for Future

### Short-term (Next Audit)
1. **Security Updates:** Check `reqwest`, `tokio`, `regex` for patches
2. **Version Updates:** Review `cargo outdated` for semver-compatible updates
3. **Feature Flags:** Audit heavy crates (`opentelemetry-*`, `image`) for unused features

### Medium-term
1. **Consider removing `do_over`** if circuit breaker logic can be inlined
2. **Evaluate `enigo`** necessity - only used for native input fallback
3. **Consolidate `tracing` + `log`** - consider migrating to `tracing` exclusively

### Long-term
1. **Rust 1.80+ Migration:** When MSRV allows, migrate `once_cell` to `std::sync::LazyLock`
2. **Binary Size:** Profile binary with `cargo bloat` to identify largest dependencies
3. **Supply Chain:** Consider `cargo vet` or `cargo crev` for supply chain security

## Verification

```bash
# All checks pass
cargo check              # ✓ PASSED
cargo test --lib         # ✓ PASSED (1949 tests)
cargo clippy            # ✓ PASSED (0 warnings)
./check.ps1             # ✓ ALL CHECKS PASSED
```

## Changelog

```
Cargo.toml
- Removed: urlencoding = "2.1.3"
- Removed: humantime = "2"
- Removed: lazy_static = "1.4"

src/utils/twitter/twitteractivity_sentiment_llm.rs
- Added: use once_cell::sync::Lazy;
- Changed: lazy_static! {} → static _: Lazy<_> = Lazy::new(|| ...)
```

---

**Next Audit Recommended:** 2026-06-01 (monthly cadence)
