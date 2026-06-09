# Baseline

## What I Find

`src/utils/twitter/sentiment/analyzer.rs` is 1,800 lines with these groups mixed together:

| Lines | Content |
|-------|---------|
| 1-120 | Imports + contextual analysis helpers (calculate_contextual_score, is_negated, get_intensifier_multiplier, analyze_contextual_modifiers, has_sarcasm_markers, is_excessive_punctuation) |
| 197-238 | 4 strategy structs: BasicKeywordStrategy, ContextStrategy, EmojiStrategy, DomainStrategy + their SentimentStrategy impls |
| 381-471 | 10 data types: Sentiment (enum), ThreadContext, ConversationIndicator (enum), UserReputation, TemporalFactors, EnhancedSentimentResult, ScoreBreakdown, SentimentConfig, SentimentAnalyzer, SentimentStats (at 837) |
| 473-799 | SentimentAnalyzer impl: new() (497), analyze_sentiment() (530), analyze_sentiment_sync() (551), analyze_enhanced() (560), analyze_thread_context() (607) |
| 802-903 | Public API: sentiment_score(), analyze_tweet_sentiment_sync(), SentimentStats, feed_sentiment_score(), analyze_sentiment_sync() |
| 905-971 | Thread context extraction: extract_thread_context(), extract_user_reputation(), extract_temporal_factors(), detect_conversation_indicators() |
| 821-831 | extract_tweet_text() helper |
| 937, 957 | 2 TODO stubs: author_prestige() and tweet_recency() |
| 1043-1800 | `#[cfg(test)] mod tests` (~757 lines) |

Existing `strategies/` directory contains separate strategy files (basic.rs, context.rs, emoji.rs, domain.rs) but the analyzer.rs still defines the strategy structs inline.

## What I Claim

Extracting the 4 type groups (data types, strategy structs, analyzer core, helpers) will reduce the main file from 1,800 to ≤50 lines (re-exports only). The existing `strategies/` directory supports this pattern. Fixing the 2 TODO stubs addresses a known feature gap.

## What Is the Proof

1. **1,800-line monolith**: The analyzer is the 5th largest file in the project. Types, strategies, core logic, and helpers are all in one file with no submodule separation.

2. **2 TODO stubs**: `author_prestige()` (line 937) and `tweet_recency()` (line 957) are documented as stubs — `"TODO: Implement actual extraction"`. These are known gaps affecting sentiment quality.

3. **Existing strategies/ pattern**: The directory already has `strategies/basic.rs`, `strategies/context.rs`, `strategies/emoji.rs`, `strategies/domain.rs` — but the strategy structs themselves are still defined inline in analyzer.rs rather than in their respective files.
