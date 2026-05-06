# Twitter Engagement Module Refactoring

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary
The `twitteractivity_engagement.rs` file has grown into a "God Object" exceeding 1,400 lines, with its core function `process_candidate` handling over 700 lines of disparate logic (sentiment modulation, action dispatch, rate limit checks, and thread diving). This spec proposes splitting the file into a focused `engagement/` directory to adhere to the Single Responsibility Principle and drastically improve maintainability.

## Scope
- **In scope**: Splitting `twitteractivity_engagement.rs` into smaller modules (e.g., `engagement/dive.rs`, `engagement/actions.rs`, `engagement/limits.rs`). Refactoring `process_candidate` into a pipeline-based approach.
- **Out of scope**: Changing the underlying business rules for engagement or rewriting the DOM interaction logic inside `interact.rs`.

## Next Step
Extract `process_candidate` sub-routines into a new module structure.