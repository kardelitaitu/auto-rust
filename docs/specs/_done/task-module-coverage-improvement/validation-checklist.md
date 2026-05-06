# Validation Checklist

- [x] `twitteractivity.rs` has tests for payload/config helpers and entry-point selection.
  - `config_tests` module covers `TaskConfig::from_payload` parsing
  - `navigation_tests` module covers `select_entry_point` helper
  - Additional tests in `twitteractivity_state.rs` cover payload parsing edge cases
- [x] `twitterfollow.rs` has tests for candidate ordering and URL or username extraction.
  - 35+ tests covering URL normalization, tweet URL detection, username extraction
  - Extensive locator candidate ordering tests (scoped vs global vs generic)
  - URL extraction from various payload fields
  - Duration and backoff delay helpers
- [x] `twitterintent.rs` has tests for intent parsing and payload URL extraction.
  - 30+ tests covering intent type detection, parameter extraction
  - URL extraction with CLI truncation handling
  - Confirm selector logic for all intent types
  - Flow verification tests
- [x] All tests are deterministic and focused on helper behavior.
- [x] Public task behavior remains stable (2243 tests pass).
- [x] `./check-fast.ps1` passes.
- [x] `./check.ps1` passes.
