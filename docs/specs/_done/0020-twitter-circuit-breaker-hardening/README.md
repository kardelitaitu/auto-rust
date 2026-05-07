# Twitter Circuit Breaker Hardening

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary
The current `CircuitBreaker` implementation in `twitteractivity_retry.rs` uses standard library Mutexes (`std::sync::Mutex`) in an async context with unhandled `.unwrap()` calls. This is a severe stability vulnerability. This spec aims to harden the concurrency model by migrating to safe async primitives or eliminating the unwraps.

## Scope
- **In scope**: Replacing `std::sync::Mutex` in `twitteractivity_retry.rs` and safely managing state.
- **Out of scope**: Altering the underlying logic of the exponential backoff or the conditions that trigger the circuit breaker.

## Next Step
Audit `twitteractivity_retry.rs` and design the new state management approach.

# Baseline

## What I Find
The `CircuitBreaker` implementation in `twitteractivity_retry.rs` contains **7 instances of `.unwrap()`** directly on mutex lock acquisitions (e.g., `*self.is_open.lock().unwrap() = false;`). It uses synchronous `std::sync::Mutex` inside an async environment.

## What I Claim
This implementation is highly vulnerable to "lock poisoning." If any thread panics while holding the lock, the mutex becomes poisoned. Because of the `.unwrap()`, any subsequent task that tries to use the circuit breaker will also panic, causing a cascading failure that crashes the entire orchestrator. Furthermore, holding an `std::sync::Mutex` across await points (though not currently the case, it's a risk) can lead to deadlocks in Tokio.

## What Is the Proof
1. File `twitteractivity_retry.rs` at lines 106, 109, 113, 125, 126, 132, 134 uses `.lock().unwrap()`.
2. The `execute<T, F, Fut>` method operates asynchronously but relies on synchronous state structures.

