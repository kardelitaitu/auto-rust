# Twitter Depth-First Thread Engagement

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

The current `twitteractivity` task follows a "Dive and Retreat" pattern: it finds a candidate on the home feed, dives into the thread to perform one action, and immediately navigates back to "Home". This is inefficient and lacks human-like behavior (real users often browse and like replies within a thread).

This initiative implements **Depth-First Engagement**, allowing the automation to scan and engage with high-quality replies while already inside a thread dive. This increases engagement density, reduces navigation overhead, and makes the bot's behavior significantly more realistic.

## Scope

- **In scope**:
  - Updating `process_candidate` in `twitteractivity_engagement.rs` to support a "reply scanning" phase after root engagement.
  - Creating `js_identify_thread_replies` in `twitteractivity_selectors.rs` to find engageable comments within the current thread view.
  - Implementing a sub-loop for engaging with high-quality replies (likes only for safety) before returning to Home.
  - Ensuring reply engagement respects the same limits and persona weights as the main loop.
- **Out of scope**:
  - Recursive thread dives (diving into a reply of a reply).
  - Multi-page scrolling inside a thread (limit to what's visible or a small scroll).

## Files

- `spec.yaml`
- `baseline.md`
- `internal-api-outline.md`
- `plan.md`
- `validation-checklist.md`
- `ci-commands.md`
- `decisions.md`
- `quality-rules.md`
- `implementation-notes.md`

## Rules

- Keep it efficient: don't spend more than 30-60 seconds in a single thread.
- Prioritize safety: engage with replies primarily via "Like" to avoid template-based reply spamming.
- Run `spec-lint.ps1` before handoff.

## Next Step

Update the engagement logic to allow scanning for replies after a successful thread dive.
