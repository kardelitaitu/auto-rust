# Twitter Integration Test Extraction

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary
The core production files (like `twitteractivity_engagement.rs` and `twitteractivity_limits.rs`) contain hundreds of lines of complex integration and statistical tests at the bottom of the files. This initiative extracts those heavy tests into the `tests/` directory to clean up the production codebase and reduce compilation times.

## Scope
- **In scope**: Moving integration tests, statistical distribution tests, and property tests from `src/` to `tests/`. Creating a mock framework inside `tests/` to support them.
- **Out of scope**: Removing lightweight unit tests that verify purely isolated functionality.

## Next Step
Map all inline tests in the `twitter` module to identify which ones are heavy integration tests vs lightweight unit tests.

# Baseline

## What I Find
Production files contain massive testing blocks. For instance, `twitteractivity_engagement.rs` contains `decision_integration_tests`, `statistical_tests`, and `property_tests` from line 1115 to 1425 (over 300 lines). Similarly, `twitteractivity_limits.rs` contains complex state simulation tests.

## What I Claim
Placing heavy integration and statistical tests inside production files bloats the file sizes, making them harder to navigate, and slows down the compilation of the core library. Extracting them will lead to a leaner codebase and enforce cleaner separation of concerns.

## What Is the Proof
1. `twitteractivity_engagement.rs` includes large inline JSON payloads mimicking Twitter API responses inside its `[cfg(test)]` modules.
2. The compilation of `src/` is burdened by extensive `property_tests` and `statistical_tests` that verify probabilistic logic via large-scale iterations (e.g., 1000 trials).

