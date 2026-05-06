# Baseline

## What I Find
The `src/utils/mouse.rs` file is nearly **2,900 lines** long, containing 127 functions and 232 lines of hardcoded mathematical constants/points.

## What I Claim
A utility module for mouse interactions should not rival the core orchestrator in size. This indicates that complex algorithms (like Bezier curve generation or Fitts's Law calculations) and heavy integration tests are dumped into a single utility file, severely impacting maintainability.

## What Is the Proof
1. The file mixes basic interactions (clicking, hovering) with highly complex mathematical trajectory generation (`random_in_range`, `gaussian` distributions).
2. It contains over 40 inline tests.
3. It exports multiple submodules (`native`, `trajectory`, `types`) but still keeps massive amounts of logic in the root file.
