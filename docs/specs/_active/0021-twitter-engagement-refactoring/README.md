# Twitter Engagement Module Refactoring

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary
The `twitteractivity_engagement.rs` file has a `process_candidate` function spanning 762 lines (lines 95-857). While the twitter module is already well-modularized (27 files in `src/utils/twitter/`), this core function mixes sentiment analysis, action selection, thread diving, and depth-first engagement in one large block. This spec proposes refactoring within the file to extract helper functions and reduce cognitive load.

## Scope
- **In scope**: Extract helper functions from `process_candidate` within `twitteractivity_engagement.rs` to reduce its size from 762 lines to ~200-300 lines. Keep all code in the same file.
- **Out of scope**: Creating new directory structure, moving code to other files, changing the underlying business rules.

## Next Step
Extract `process_candidate` sub-routines into helper functions within the same file.

# Baseline

## What I Find
The `twitteractivity_engagement.rs` file is **1,325 lines** long. The central function `process_candidate` spans **762 lines** (lines 95-857) and handles sentiment modulation, action dispatch, retry logic, and depth-first engagement reply scanning.

## What I Claim
While not a "God Object" (twitter module is already well-modularized), the `process_candidate` function is too large and mixes multiple concerns inline. Extracting helpers will improve readability and testability without adding unnecessary abstraction.

## What Is the Proof
1. `process_candidate` handles 6+ concerns: sentiment analysis, smart decisions, action selection, thread diving, action execution (like/retweet/quote/reply/follow/bookmark), and depth-first reply engagement.
2. The depth-first engagement loop (lines 737-857) is 120 lines of nested logic.
3. Action execution block (lines 362-710) has repetitive patterns for each action type.

