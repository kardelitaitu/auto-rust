# Plan

## What Is the Solution

Extract 4 groups from `sentiment/analyzer.rs`:

| New/Updated File | Content | Source Lines | Target |
|------------------|---------|-------------|--------|
| `types.rs` | Sentiment, ThreadContext, ConversationIndicator, UserReputation, TemporalFactors, EnhancedSentimentResult, ScoreBreakdown, SentimentConfig, SentimentStats | 381-471, 837-871 | ≤400 |
| `strategies/basic.rs` | BasicKeywordStrategy + SentimentStrategy impl (move from analyzer.rs) | 197-209 | ≤100 |
| `strategies/context.rs` | ContextStrategy + impl (move from analyzer.rs) | 226-231 | ≤100 |
| `strategies/emoji.rs` | EmojiStrategy + impl (move from analyzer.rs) | 232-237 | ≤100 |
| `strategies/domain.rs` | DomainStrategy + impl (move from analyzer.rs) | 238-243 | ≤100 |
| `analyzer_core.rs` | SentimentAnalyzer struct + impl (new, analyze_sentiment, analyze_sentiment_sync, analyze_enhanced, analyze_thread_context) | 473-799 | ≤500 |
| `helpers.rs` | Contextual analysis helpers + public API + thread extraction | 1-120, 802-971 | ≤500 |
| `analyzer.rs` | Module re-exports only | shrunken | ≤50 |

**TODO fixes**: Implement `author_prestige()` (extract from follower count, verification status) and `tweet_recency()` (extract from timestamp vs current time) in `helpers.rs`.

**Test distribution**: All tests (1043-1800) move to a `#[cfg(test)] mod tests` in `analyzer_core.rs`.
