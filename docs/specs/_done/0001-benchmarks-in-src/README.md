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

# Baseline

## Current State

- Benchmark harnesses live in root `benches/`.
- The three current files are:
  - `trajectory.rs`
  - `accessibility_locator.rs`
  - `predictive_scorer.rs`
- `Cargo.toml` already registers them with `[[bench]]`.

## Why This Needs Work

- `trajectory.rs` duplicates logic that already exists in `src/utils/mouse/trajectory.rs`.
- `accessibility_locator.rs` duplicates the parser that already exists in `src/utils/accessibility_locator.rs`.
- `predictive_scorer.rs` is a standalone benchmark copy and does not measure the library API directly.
- `src/utils/accessibility_locator.rs` is feature-gated, so the benchmark will need a feature-aware path.
- `src/adaptive/mod.rs` does not currently surface `predictive_scorer`, so that benchmark needs a small seam or export.

## Current Risk

- Moving files without fixing the duplicated logic would only relocate the problem.
- The benchmark suite would remain misleading if it keeps measuring toy copies instead of real code.

