# Benchmarks in src

Status: `done`

Owner: `spec-agent`
Implementer: `implementation-agent`

## Summary

Move the Criterion benchmark harnesses from the root `benches/` folder into `src/`
while keeping them as separate benchmark targets, not runtime library modules.

This folder is the current worked example for the spec system.

## Scope

- In scope:
  - Move benchmark entrypoints into `src/benchmarks/`
  - Extract any shared setup into `src/benchmark_support/` if needed
  - Repoint Cargo benchmark targets to the new paths
  - Rewire benchmarks to real library code instead of copied mini-implementations
  - Remove the root `benches/` folder after parity is verified
- Out of scope:
  - Changing benchmark goals or metrics
  - Adding new benchmark categories
  - Moving benchmark code into `lib.rs`

## Next Step

Archived after verification.
