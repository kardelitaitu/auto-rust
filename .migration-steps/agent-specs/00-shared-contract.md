# Shared Contract For All Agents

## Mission
Implement the Rust orchestrator migration with behavior compatible with the canonical smoke command:

`cargo run cookiebot pageview=www.reddit.com then cookiebot`

## Global Rules
- Keep parser output string-normalized in this migration track (`HashMap<String, String>`).
- Do not change CLI grammar without updating docs and tests together.
- Do not weaken reliability requirements (timeouts, retries, graceful shutdown).
- Keep code ASCII.

## Required Behavior
- `then` splits sequential groups (case-insensitive).
- `.js` suffix on task names is accepted and stripped.
- URL-like values are normalized with `https://` when protocol is missing.
- Numeric values are stored as string payload under `value`.

## Done Criteria (global)
- Smoke command is parsed into 2 groups as expected.
- No panic on startup in idle mode (`cargo run`).
- All assigned tests for each agent pass.
- Agent provides a short change report: files touched, tests run, remaining risks.

## Coordination
- If a change impacts another agent's interface, document it in the report and keep backward compatibility where possible.
- Prefer additive changes over breaking refactors.
