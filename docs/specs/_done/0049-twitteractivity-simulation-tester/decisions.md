## Decisions

| Option | Pros | Cons | Decision |
|---|---|---|---|
| Separate simulation path | Clear intent, no browser dependency, easy to test | Needs a new planner and a new entry point | Chosen |
| Reuse live dry-run flag | Smaller surface change | Still too coupled to browser flow, not truly simulation-only | Rejected |

### Rationale

- The request is for a behavior simulator, not a browser dry-run.
- Deterministic in-memory planning is easier to validate and safer to keep fast.
- Keeping simulation separate avoids weakening the live task contract.
