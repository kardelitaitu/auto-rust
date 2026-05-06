# Twitter Sentiment Analysis Consolidation

Status: `done`

Owner: `spec-agent`
Implementer: `pending`

## Summary
The Twitter activity module currently contains 6 distinct sentiment analysis modules totaling over 4,400 lines of code. This initiative consolidates these into a single cohesive `SentimentAnalyzer` architecture using the Strategy Pattern to simplify the API and reduce code duplication.

## Scope
- **In scope**: Consolidating `sentiment.rs`, `sentiment_context.rs`, `sentiment_domains.rs`, `sentiment_emoji.rs`, `sentiment_enhanced.rs`, and `sentiment_llm.rs`.
- **Out of scope**: Altering the underlying logic or thresholds of the sentiment scoring algorithms themselves.

## Next Step
Analyze the 6 modules to extract common types and traits.

# Baseline

## What I Find
The sentiment analysis capability is heavily fragmented. There are 6 different modules (`sentiment.rs`, `sentiment_context.rs`, `sentiment_domains.rs`, `sentiment_emoji.rs`, `sentiment_enhanced.rs`, `sentiment_llm.rs`) responsible for scoring tweet sentiment.

## What I Claim
This extreme fragmentation creates an overly complex internal API surface and results in redundant operations (e.g., multiple modules tokenizing and parsing the same string separately). Consolidating this will reduce the codebase size by roughly 40-50% while making it easier to integrate new semantic signals later.

## What Is the Proof
1. The combined line count of these modules exceeds 4,400 lines.
2. The orchestrator has to manually select between "basic" and "enhanced" sentiment analysis (`if task_config.enhanced_sentiment_enabled`), leaking implementation details into the caller.
3. String tokenization, regex matching, and basic word-boundary checks are duplicated across `sentiment_domains.rs`, `sentiment_context.rs`, and `sentiment.rs`.

