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