# Agent 4 Spec: Utils Layer And Task Migration Baseline

## Scope
Own utility module completeness and baseline `cookiebot`/`pageview` task viability.

## Files
- `src/utils/*.rs`
- `task/cookiebot.rs`
- `task/pageview.rs`

## Functional Requirements
- Replace TODO stubs in high-use utility functions.
- Keep utility APIs consistent with orchestrator/task usage.
- `pageview` supports payload URL override and fallback URL file path.
- `cookiebot` supports repeatable navigation loop with bounded waits.

## Data Requirements
- Load URL lists from `data/*.txt` and ignore empty/comment lines.

## Acceptance Checks
- Utility unit tests for bounds and basic behavior pass.
- `pageview=www.reddit.com` executes URL normalization path.
- Missing data file returns clear error context.

## Out Of Scope
- CI pipeline
- Deployment packaging

## Report Format
- Utilities completed
- Task behavior parity notes vs Node.js
- Remaining TODOs
