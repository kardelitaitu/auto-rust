# Twitter Depth-First Thread Engagement

Status: `done`

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

# Baseline

## Current Behavior

1.  **Strict 1:1 Engagement**: For every thread "dive", exactly one root tweet is engaged with.
2.  **Immediate Retreat**: Even if a thread has hundreds of high-quality replies, the bot navigates back to the "Home" feed immediately after the root action.
3.  **Navigation Overhead**: Transitioning from Home -> Status -> Home for every single action consumes significant time and increases the footprint of automation.

## Why It Needs Work

- **Efficiency Gap**: We are wasting the "cost" of the page load. Once we are in a thread, the most efficient thing is to engage with 2-3 high-quality items rather than just one.
- **Realism Gap**: Real users don't exclusively engage with the first tweet of every thread they open. They scroll down, read some comments, and like the ones they agree with.
- **Social Impact Gap**: Engaging with replies helps build community presence beyond just the main "influencer" tweets.

## Relevant Files

- `src/utils/twitter/twitteractivity_engagement.rs`
- `src/utils/twitter/twitteractivity_selectors.rs`

## Known Failure Modes

- Bottlenecked by navigation speed: the bot spends ~40% of its time just waiting for the home feed or status pages to load.
- Repetitive navigation patterns (Home -> Status -> Home) are easier for bot detection systems to fingerprint.

## Evidence

- Analysis of `process_candidate` shows an unconditional `goto_home(api)` call immediately after the root action.
- Log analysis shows "Dive Success" followed immediately by "Navigating back to home".

