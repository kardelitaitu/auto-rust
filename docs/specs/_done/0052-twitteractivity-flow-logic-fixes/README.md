# Fix Twitter Activity flow logic and timing bugs

Status: `done`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Eight high-severity and ten medium-severity issues in the Twitter Activity task flow. Key problems: post-dive duplicate feed rescanning, non-interruptible sleep bypassing deadline, non-like actions gated behind dive decision, inconsistent PersonaStrategy multiplier, write-only actions_taken, non-standard CSS selectors in cookie dismissal. Fixes range from 1-20 lines each.

## Scope

- In scope:
  - Reset next_candidate_scan after thread dive
  - Interruptible/min-bound sleep in main loop
  - Decouple should_dive from non-like action gating
  - Fix PersonaStrategy multiplier consistency
  - Remove write-only actions_taken
  - Fix cookie banner :contains() selectors
  - Seed select_entry_point with TaskConfig.seed
  - Replace 300s dive pause with actual duration
  - Lazy static regex compilation
- Out of scope:
  - New LLM provider integration
  - Refactoring engagement probability model
  - Adding new action types
