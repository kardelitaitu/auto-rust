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