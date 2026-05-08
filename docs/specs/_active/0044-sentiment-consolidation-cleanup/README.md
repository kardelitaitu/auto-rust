# Sentiment Consolidation Cleanup

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Spec `0016-twitter-sentiment-consolidation` aimed to unify 6 fragmented sentiment modules into a cohesive `sentiment/` directory using the Strategy Pattern. However, the implementation was incomplete. The implementer created the `SentimentAnalyzer` facade but left the old `twitteractivity_sentiment_*.rs` files intact, simply delegating to them instead of migrating the code. This left "zombie code" in the codebase, negating the file-reduction benefits and bloating the `mod.rs` exports. This spec finalizes the migration by moving the logic into proper strategy modules, deleting the old files, and cleaning up the public API.

## Scope

- In scope:
  - Creating the `src/utils/twitter/sentiment/strategies/` directory.
  - Moving logic from `twitteractivity_sentiment_emoji.rs`, `twitteractivity_sentiment_domains.rs`, and `twitteractivity_sentiment_llm.rs` into respective files under `strategies/`.
  - Deleting all six old `twitteractivity_sentiment_*.rs` files.
  - Removing the old module exports from `src/utils/twitter/mod.rs`.
- Out of scope:
  - Modifying the underlying sentiment scoring math or LLM prompts.
  - Changing consumer usage (which already correctly points to the new `SentimentAnalyzer`).

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

Wait for the implementer agent to execute the file migrations and deletions.
