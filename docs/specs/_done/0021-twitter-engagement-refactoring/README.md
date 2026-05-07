# Twitter Engagement Module Refactoring

Status: `done`

Owner: `spec-agent`
Implementer: `implementation-agent`

## Summary
The `twitteractivity_engagement.rs` file has a `process_candidate` function spanning 762 lines (lines 95-857). This spec refactors within the file by extracting helper functions to reduce cognitive load. Extracted: `modulate_persona_by_sentiment` and `engage_replies`.

## Scope
- **In scope**: Extract helper functions from `process_candidate` within the same file. Reduced `process_candidate` from 762 to ~650 lines.
- **Out of scope**: Creating new directories, moving code to other files, changing underlying business rules.

## Next Step
Spec implemented. Maintenance only.

# Baseline

## What I Find
The `twitteractivity_engagement.rs` file is **1,325 lines** long. The central function `process_candidate` spans **762 lines** (lines 95-857) and handles sentiment modulation, action dispatch, retry logic, and depth-first engagement reply scanning.

## What I Claim
While not a "God Object" (twitter module is already well-modularized), the `process_candidate` function is too large and mixes multiple concerns inline. Extracting helpers will improve readability and testability without adding unnecessary abstraction.

## What Is the Proof
1. `process_candidate` handles 6+ concerns: sentiment analysis, smart decisions, action selection, thread diving, action execution (like/retweet/quote/reply/follow/bookmark), and depth-first reply engagement.
2. The depth-first engagement loop (lines 737-857) is 120 lines of nested logic.
3. Action execution block (lines 362-710) has repetitive patterns for each action type.

