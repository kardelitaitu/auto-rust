## Acceptance Criteria
- src/utils/retry.rs exists with RetryConfig, retry_with_backoff, ExponentialBackoff, and unit tests
- RetryConfig supports: max_attempts, base_delay_ms, max_delay_ms, multiplier, jitter_factor
- ExponentialBackoff iterator yields delays: base * multiplier^n, clamped to max_delay_ms, with uniform jitter
- retry_with_backoff retries on error up to max_attempts
- session/pool.rs discover_and_connect uses the shared retry (replacing linear backoff)
- All existing tests continue to pass: cargo test --lib
- cargo clippy --all-targets --all-features is clean
- check-fast.ps1 passes

## Test Commands
- cargo test --lib utils::retry
- cargo test --lib session::pool
- cargo clippy --all-targets --all-features
- cargo fmt --all --check
- .\check-fast.ps1

## Visual Inspection
- src/utils/retry.rs exists with RetryConfig, ExponentialBackoff, retry_with_backoff, and #[cfg(test)] mod tests
- src/utils/mod.rs has `pub mod retry;`
- src/session/pool.rs imports ExponentialBackoff and RetryConfig from utils::retry
- src/session/pool.rs discover_and_connect and discover_with_filters use ExponentialBackoff instead of manual linear backoff
