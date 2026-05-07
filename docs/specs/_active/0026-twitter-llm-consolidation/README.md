# Twitter LLM Module Consolidation

Status: `pending`

Owner: `spec-agent`
Implementer: `pending`

## Summary
The twitter module has 27 files, some very small (e.g., `twitteractivity_persona.rs` = 74 lines, `twitteractivity_errors.rs` = small). This spec proposes consolidating tiny modules where it makes sense to reduce file count and improve maintainability.

## Scope
- **In scope**: Merge small related modules (e.g., persona + constants, errors + retry, sentiment sub-modules consolidation).
- **Out of scope**: Changing functionality, merging large modules (>500 lines).

## Next Step
Analyze all 27 twitter module files and identify consolidation candidates.

# Baseline

## What I Find
The `src/utils/twitter/` directory has 27 files. Some modules are very small:
- `twitteractivity_persona.rs` = 74 lines
- `twitteractivity_errors.rs` = small
- Multiple sentiment sub-modules could be consolidated

## What I Claim
27 files is too many for the twitter module. Consolidating small related modules will reduce cognitive load and make the codebase easier to navigate.

## What Is the Proof
1. `twitteractivity_persona.rs` (74 lines) could be merged with `twitteractivity_constants.rs`
2. `twitteractivity_errors.rs` could be merged with `twitteractivity_retry.rs`
3. Sentiment sub-modules (5 files) could be consolidated into a single `sentiment.rs`
