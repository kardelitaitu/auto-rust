# Plan

## What Is the Solution

1. **New Directory**: Create `src/utils/twitter/sentiment/` analogous to the recent `decision/` refactoring.
2. **Unified Interface**: Create a `SentimentAnalyzer` interface that encapsulates the logic for basic vs. enhanced processing based on configuration.
3. **Strategy Pattern**: Convert the domain, emoji, context, and LLM evaluations into strategies that plug into the unified analyzer.
4. **Remove Duplication**: Centralize string parsing and tokenization into a shared utility within the `sentiment` module.
5. **Update Consumers**: Refactor `twitteractivity_engagement.rs` to rely exclusively on the new unified interface, completely removing the branch for `enhanced_sentiment_enabled`.

# internal api outline

Implementation details to be defined during active development.

# decisions

## Architecture Decisions

1. **Strategy Pattern**: Chose Strategy Pattern over inheritance to allow runtime configuration of analysis components and easy extensibility.

2. **Unified Interface**: Single SentimentAnalyzer class instead of multiple separate functions to provide consistent API and reduce cognitive load.

3. **Backward Compatibility**: Maintained EnhancedSentimentResult structure to avoid breaking changes in consumer code.

4. **Async Support**: Added async analysis methods for LLM integration while keeping sync methods for performance-critical paths.

5. **Modular Organization**: Split into analyzer.rs, strategies.rs, and utils.rs for clear separation of concerns.

## Design Trade-offs

- **Complexity vs Flexibility**: Strategy pattern adds some complexity but enables easy addition of new analysis types.
- **Performance vs Features**: Sync methods for basic analysis, async for enhanced features with LLM.
- **Code Size vs Maintainability**: Some duplication in constants but centralized in strategies.rs for easier maintenance.

