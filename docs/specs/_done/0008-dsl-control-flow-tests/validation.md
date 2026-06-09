## Acceptance Criteria
- control_flow.rs has 15+ new unit tests covering execute_if, execute_loop, execute_foreach, execute_retry, execute_parallel via MockDslApi
- Tests cover: If(condition=true) runs then-branch, If(condition=false) runs else-branch, Loop(count=N) iterates N times, Foreach iterates over collection, Retry retries on failure up to max_attempts, Parallel dispatches all actions
- MockDslApi is used for all new tests (no browser required)
- All tests pass: cargo test --lib task::dsl
- cargo clippy --all-targets --all-features is clean
- check-fast.ps1 passes

## Test Commands
- cargo test --lib task::dsl::control_flow
- cargo test --lib task::dsl
- cargo clippy --all-targets --all-features
- cargo fmt --all --check
- .\check-fast.ps1

## Visual Inspection
- control_flow.rs `#[cfg(test)]` module has 15+ new test functions
- No production code in control_flow.rs was modified
- No new files were created
- All tests use MockDslApi from api.rs (no browser dependencies)
