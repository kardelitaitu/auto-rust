# Fix critical runtime bugs in Twitter Activity

Status: `done`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Three critical bugs in the Twitter Activity task: (1) the LLM API key is never passed to the decision engine, so LLM-powered engagement decisions silently fall back to rule-based heuristics; (2) extract_tweet_context() JS has malformed selectors that return zero replies and assigns the wrong author to all replies; (3) popup dismissal runs after login verification, causing false "not logged in" warnings. All three are small fixes with high impact.

## Scope

- In scope:
  - Thread LLM config through to DecisionEngineFactory
  - Fix JS selectors and author assignment in extract_tweet_context()
  - Reorder phase1_navigation to dismiss popups before login check
  - Fix or remove the disabled dismiss_signup_nag() call
- Out of scope:
  - Adding new LLM providers or strategies
  - Refactoring the decision engine architecture
  - Adding new popup types or dismissal methods

## Files

- spec.yaml
- baseline.md
- plan.md
- validation-checklist.md
- ci-commands.md
- decisions.md
- quality-rules.md
- implementation-notes.md

## Next Step

Implementer: run check-fast.ps1 while iterating and check.ps1 before push.
