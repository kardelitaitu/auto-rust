# Baseline

## What I Find
Production files contain massive testing blocks. For instance, `twitteractivity_engagement.rs` contains `decision_integration_tests`, `statistical_tests`, and `property_tests` from line 1115 to 1425 (over 300 lines). Similarly, `twitteractivity_limits.rs` contains complex state simulation tests.

## What I Claim
Placing heavy integration and statistical tests inside production files bloats the file sizes, making them harder to navigate, and slows down the compilation of the core library. Extracting them will lead to a leaner codebase and enforce cleaner separation of concerns.

## What Is the Proof
1. `twitteractivity_engagement.rs` includes large inline JSON payloads mimicking Twitter API responses inside its `[cfg(test)]` modules.
2. The compilation of `src/` is burdened by extensive `property_tests` and `statistical_tests` that verify probabilistic logic via large-scale iterations (e.g., 1000 trials).