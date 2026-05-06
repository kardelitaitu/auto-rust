# Twitter Engagement Module Refactoring

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary
The `twitteractivity_engagement.rs` file has grown into a "God Object" exceeding 1,400 lines, with its core function `process_candidate` handling over 700 lines of disparate logic (sentiment modulation, action dispatch, rate limit checks, and thread diving). This spec proposes splitting the file into a focused `engagement/` directory to adhere to the Single Responsibility Principle and drastically improve maintainability.

## Scope
- **In scope**: Splitting `twitteractivity_engagement.rs` into smaller modules (e.g., `engagement/dive.rs`, `engagement/actions.rs`, `engagement/limits.rs`). Refactoring `process_candidate` into a pipeline-based approach.
- **Out of scope**: Changing the underlying business rules for engagement or rewriting the DOM interaction logic inside `interact.rs`.

## Next Step
Extract `process_candidate` sub-routines into a new module structure.

# Baseline

## What I Find
The `twitteractivity_engagement.rs` file is currently **1,425 lines** long. The central function `process_candidate` spans **730 lines** and manages everything from sentiment modulation to hardcoded retry loops and deep nested iterations.

## What I Claim
This massive file violates the Single Responsibility Principle (SRP). It is extremely difficult to unit test effectively because `process_candidate` requires 4 complex arguments and modifies state across 6 distinct sub-domains. Modularizing this file will cut cognitive load and reduce the risk of bug introduction during future features.

## What Is the Proof
1. `process_candidate` takes `CandidateContext` but unpacks and mutates counters, action trackers, DOM state, and persona weights inline.
2. Hardcoded retry loops for actions like `like_tweet` or `retweet_tweet` are deeply nested inside a massive `for action in [selected_action]` loop.
3. The recent addition of Depth-First Engagement added another deep loop inside this structure, exacerbating the complexity.

