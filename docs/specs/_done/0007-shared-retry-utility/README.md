# Consolidate Retry/Backoff Logic into a Shared Utility Module

**Status:** `approved`

**Owner:** `spec-agent`
**Implementer:** `pending`

## Summary

The auto-rust codebase has at least 3 independent retry/backoff implementations
spread across `session/pool.rs`, `api/client.rs`, `utils/twitter/`, and
`llm/client.rs`. Each uses a different strategy (linear, exponential, custom),
different configuration shapes, and none are tested in isolation.

This proposal creates a single `src/utils/retry.rs` module with a well-defined
`RetryConfig`, `ExponentialBackoff` iterator, and `retry_with_backoff` adapter.
The first consumer migrated is `session/pool.rs` (linear backoff with no jitter).

## Scope

- **Create** `src/utils/retry.rs` — ~150 lines of Rust + tests
- **Migrate** `session/pool.rs` — replace inline linear backoff with shared retry
- **Test** — unit tests for ExponentialBackoff, RetryConfig edge cases,
  Retry-After parsing, and error propagation

## Next Steps

1. Implement `src/utils/retry.rs`
2. Migrate `session/pool.rs`
3. Run `cargo test --lib` and `check-fast.ps1`
4. Archive to `_done/`
