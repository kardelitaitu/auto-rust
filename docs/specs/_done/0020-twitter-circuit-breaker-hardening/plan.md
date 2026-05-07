# Plan

## What Is the Solution

1. **Replace Primitives**: Replace `std::sync::Mutex` with `tokio::sync::Mutex` (which does not suffer from poisoning and is async-safe) or use an `std::sync::RwLock` if the state is heavily read-biased.
2. **Remove Unwraps**: Completely eliminate `.unwrap()` calls. If a lock fails, it should be handled gracefully (e.g., returning an `Err` or defaulting to an open circuit to fail safe) or use `.expect("Reason")` only if invariants absolutely guarantee safety.
3. **Validation**: Write tests simulating concurrent thread panics to ensure the circuit breaker does not poison the rest of the application.

# internal api outline

Implementation details to be defined during active development.

# decisions

Implementation details to be defined during active development.

