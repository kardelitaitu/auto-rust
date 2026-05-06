# Baseline

## What I Find
The `twitteractivity_engagement.rs` file is currently **1,425 lines** long. The central function `process_candidate` spans **730 lines** and manages everything from sentiment modulation to hardcoded retry loops and deep nested iterations.

## What I Claim
This massive file violates the Single Responsibility Principle (SRP). It is extremely difficult to unit test effectively because `process_candidate` requires 4 complex arguments and modifies state across 6 distinct sub-domains. Modularizing this file will cut cognitive load and reduce the risk of bug introduction during future features.

## What Is the Proof
1. `process_candidate` takes `CandidateContext` but unpacks and mutates counters, action trackers, DOM state, and persona weights inline.
2. Hardcoded retry loops for actions like `like_tweet` or `retweet_tweet` are deeply nested inside a massive `for action in [selected_action]` loop.
3. The recent addition of Depth-First Engagement added another deep loop inside this structure, exacerbating the complexity.