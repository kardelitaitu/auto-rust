# Plan

## What Is the Solution

1. **New Directory**: Create `src/utils/twitter/sentiment/` analogous to the recent `decision/` refactoring.
2. **Unified Interface**: Create a `SentimentAnalyzer` interface that encapsulates the logic for basic vs. enhanced processing based on configuration.
3. **Strategy Pattern**: Convert the domain, emoji, context, and LLM evaluations into strategies that plug into the unified analyzer.
4. **Remove Duplication**: Centralize string parsing and tokenization into a shared utility within the `sentiment` module.
5. **Update Consumers**: Refactor `twitteractivity_engagement.rs` to rely exclusively on the new unified interface, completely removing the branch for `enhanced_sentiment_enabled`.