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

