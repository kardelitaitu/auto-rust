# Clean up dead code and fix quality issues in Twitter Activity

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Remove 17 dead-code items (~400 lines) and fix 12 quality issues across the Twitter Activity module. Includes removing unused functions (read_full_thread, navigate_to_tweet, check_selector_health, retry_with_fallback, etc.), fixing HOME_LOGO_SELECTOR (spurious backslashes in raw string literal), deduplicating build_persona_weights in simulation.rs, making LLM client/regex once-per-session, and fixing quote-style inconsistency in selector constants.

## Scope

- In scope:
  - Remove/relegate all 17 dead code items
  - Fix HOME_LOGO_SELECTOR and quote-style inconsistencies
  - Deduplicate persona weight building between simulation and real paths
  - Make Llm::new() once per session
  - Make regex static Lazy
- Out of scope:
  - Browser-level integration tests
  - Refactoring simulation engine architecture
  - Adding new code or features
