# Plan

## What Is the Solution

1. **New Integration Files**: Create new test files in the `tests/` directory, such as `tests/twitteractivity_engagement_tests.rs` and `tests/twitteractivity_limits_tests.rs`.
2. **Migration**: Move all test modules marked as `integration_tests`, `statistical_tests`, and `property_tests` out of the `src/` files and into these new external test files.
3. **Mock Framework**: Since external tests rely on public API exposure, we will need to create a dedicated mock data provider inside `tests/` that supplies the necessary JSON payloads simulating Twitter API responses.
4. **Validation**: Ensure that `cargo test` passes seamlessly and that the file size of `src/utils/twitter/twitteractivity_engagement.rs` is reduced substantially.