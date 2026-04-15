# Agent 1 Spec: CLI Parser And Task Grouping

## Scope
Own parser behavior and task definition shaping.

## Files
- `src/cli.rs`
- parser-related tests in `src/tests/integration_test.rs` and `src/tests/edge_case_test.rs`

## Inputs
CLI args list such as:
- `cookiebot`
- `pageview=www.reddit.com`
- `pageview=url=https://example.com`
- `task=42`
- `then`

## Required Output Model
`TaskDefinition { name: String, payload: HashMap<String, String> }`

## Functional Requirements
- Split groups by `then` (case-insensitive).
- Strip `.js` from task names.
- Parse first `=` as task/value split.
- Numeric value -> `payload["value"] = "<digits>"`.
- URL-like value -> `payload["url"] = "https://..."` (unless protocol already present).
- Quoted value support: `task="value with spaces"`.

## Acceptance Tests
- Parse smoke command into:
  - Group 1: `cookiebot`, `pageview(url=https://www.reddit.com)`
  - Group 2: `cookiebot`
- `cookiebot` == `cookiebot.js`
- `task=42` stores `"42"` under `value`
- `pageview=reddit.com` stores `https://reddit.com`

## Out Of Scope
- Browser connections
- Task execution internals
- Deployment scripts

## Report Format
- Changed files
- Tests run
- Edge cases not covered
