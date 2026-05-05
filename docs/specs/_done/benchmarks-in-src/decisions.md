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

