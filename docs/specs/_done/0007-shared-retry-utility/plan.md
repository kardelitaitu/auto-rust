# Plan

## What Is the Solution

### 1. Create `src/utils/retry.rs`

A focused module with three public items:

**`RetryConfig`** — builder-style config struct with defaults:
- `max_attempts: u32` (default: 3)
- `base_delay_ms: u64` (default: 1000)
- `max_delay_ms: u64` (default: 30_000)
- `multiplier: f64` (default: 2.0, exponential)
- `jitter_factor: f64` (default: 0.2, 20% uniform jitter)

**`ExponentialBackoff`** — iterator yielding `Duration` delays:
- Formula: `min(base * multiplier^attempt, max_delay_ms)` + uniform jitter

**`retry_with_backoff`** — adapter that retries a fallible async fn:
- Signature: `pub async fn retry_with_backoff<T, E, F, Fut>(config: &RetryConfig, f: F) -> Result<T, E>`
- Retries on error up to `max_attempts`
- Logs retry attempts with delay info
- Respects `Retry-After` header when available

### 2. Module contents (~150 lines)

- `RetryConfig` struct with `Default` impl and builder methods
- `ExponentialBackoff` implementing `Iterator<Item = Duration>`
- `retry_with_backoff` async function
- `#[cfg(test)] mod tests` with 10+ unit tests:
  - Backoff yields increasing delays clamped to max
  - Jitter is within expected bounds
  - Retry succeeds on N-1 failures
  - Retry fails after max_attempts
  - Zero max_attempts returns immediately
  - Config edge cases (zero base, zero jitter)

### 3. Migrate `session/pool.rs`

Replace the inline retry loop in `discover_and_connect` (lines ~183-204):
- Remove manual `for attempt in 1..=self.max_retries` loop
- Call `retry_with_backoff(&config, || async { ... })` instead
- Preserve all existing error messages and logging behavior

### 4. Files to create/modify

| File | Action |
|------|--------|
| `src/utils/retry.rs` | **Create** — shared retry module |
| `src/utils/mod.rs` | **Edit** — add `pub mod retry;` |
| `src/session/pool.rs` | **Edit** — replace inline retry with shared call |

### 5. Verification

- `cargo test --lib` — all tests pass (retry tests + all existing)
- `cargo clippy --all-targets --all-features` — no new warnings
- `check-fast.ps1` — passes
- Session pool discovery still works: single retry path updated, same behavior
