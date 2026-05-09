# Twitter LLM Modularization

Status: `done`

Owner: `spec-agent`
Implementer: `pending`

## Summary

The `src/utils/twitter/twitteractivity_llm.rs` file violates the Single Responsibility Principle by mixing three distinct concerns: calling external LLM APIs to generate text, validating/sanitizing that text for Twitter compliance (e.g., removing emojis and hashtags), and executing complex DOM interactions to post Quote Tweets. This spec proposes extracting the validation and execution logic into their own dedicated sub-modules, leaving `twitteractivity_llm.rs` responsible solely for prompt building and API communication.

## Scope

- In scope:
  - Creating `src/utils/twitter/twitteractivity_llm_validation.rs` to house `validate_reply`, `remove_emojis`, `truncate_to_word_boundary`, etc.
  - Creating `src/utils/twitter/twitteractivity_llm_execute.rs` to house the `quote_tweet` DOM interaction logic.
  - Updating `twitteractivity_llm.rs` to re-export these functions or call them internally.
  - Moving the relevant unit tests (`test_validate_reply_*`) to the new validation module.
- Out of scope:
  - Modifying the underlying logic of the LLM generation, validation rules, or the DOM execution flow.
  - Adding new LLM providers.

## Files

- `spec.yaml`
- `plan.md`
- `validation.md`
- `notes.md`

## Rules

- Keep the spec short.
- Run `spec-lint.ps1` before handoff.
- Use `.\check-fast.ps1` while iterating.
- Use the archive helper `.\spec-archive.ps1` to move to `_done/`.

## Next Step

Wait for the implementer agent to extract the modules.
