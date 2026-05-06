# Plan

## Step 1

Lock the benchmark seams.

- Expose the smallest needed path for the scorer benchmark.
- Decide how the accessibility locator benchmark will build with its feature gate.

## Step 2

Move the harnesses into `src/benchmarks/`.

- Repoint `[[bench]]` entries in `Cargo.toml`.
- Keep each benchmark target thin.

## Step 3

Remove the duplicated benchmark-only logic.

- Replace copied helper code with real library calls.
- Add `src/benchmark_support/` only if shared fixtures are reused by more than one bench.

## Step 4

Verify and clean up.

- Run `cargo bench`.
- Run `./check.ps1`.
- Delete the root `benches/` folder after parity is confirmed.

## Rollback

Restore `Cargo.toml` bench paths and keep the root `benches/` folder if the moved
targets do not compile or if benchmark fidelity regresses.

# Internal API Outline

## Contract

Benchmark harnesses stay separate from runtime code, but they must call the real
library implementation paths.

## Inputs

- `crate::utils::mouse::trajectory::*`
- `crate::utils::accessibility_locator::*`
- `crate::adaptive::predictive_scorer::*` or the smallest public seam needed to reach it

## Outputs

- Criterion benchmark groups and measurements
- CSV or terminal benchmark output from the normal `cargo bench` flow

## State Changes

- Add `src/benchmarks/` for benchmark entrypoints.
- Add `src/benchmark_support/` only if shared fixture setup is needed.
- Update `Cargo.toml` bench paths.
- Add the minimum public module export or helper needed for the scorer benchmark.
- Keep `lib.rs` free of benchmark-specific imports.

## Error Paths

- Benchmark compile fails if a feature-gated module is not enabled.
- Benchmark compile fails if a moved harness still points at an old root path.
- Benchmark fidelity drops if copied logic is kept instead of real module calls.

## Invariants

- Benchmarks are source code, not generated output.
- Benchmarks do not become runtime API surface by default.
- No benchmark file should reimplement the production logic it is measuring.

# Decisions

## Keep benches in src, not lib.rs

- Chosen because the user wants the benchmark code under `src/`.
- Rejected `lib.rs` because it would pollute the runtime API surface.

## Use real library code

- Chosen because duplicated benchmark-only logic gives misleading results.
- Rejected keeping the current copies because they do not measure production paths.

## Keep the root benches folder only until parity

- Chosen because the migration needs a rollback point.
- Rejected deleting it first because it would make the move harder to recover.

## Accessibility locator handling

- Chosen path: keep the production feature gate, and make the benchmark aware of it.
- Alternative: remove the feature gate from the module.
- Tradeoff: keeping the gate is safer for runtime surface area; removing it is simpler for benches.

## Predictive scorer handling

- Chosen path: expose the minimum module seam needed for the benchmark.
- Alternative: retire the benchmark if the module is not a stable public surface.
- Tradeoff: exposing a small seam preserves the benchmark while keeping the API tight.

