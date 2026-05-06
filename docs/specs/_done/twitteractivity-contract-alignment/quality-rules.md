# Quality Rules

- The task entry point must not contain hidden fallback logic that contradicts the contract.
- Payload validation must be explicit; wrong types are errors, not silent defaults.
- The runtime and docs should move together, not drift independently.
- Changes should stay inside the TwitterActivity task boundary unless a shared helper is clearly required.
- Tests should prove the contract, not just the happy path.
