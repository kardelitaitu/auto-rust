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

