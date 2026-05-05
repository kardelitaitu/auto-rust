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

