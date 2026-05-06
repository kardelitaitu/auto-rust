//! Unified sentiment analyzer using the Strategy Pattern.
//! Provides configurable sentiment analysis with basic and enhanced modes.

use crate::internal::text::truncate_chars;
use crate::llm::client::LlmClient;
use crate::utils::twitter::twitteractivity_sentiment_llm;

use super::SentimentStrategy;
use log;
use serde_json::Value;
use tracing::instrument;

// ============================================================================
// Strategy Constants and Functions
// ============================================================================

/// Negation patterns that flip sentiment polarity.
const NEGATION_PATTERNS: &[&str] = &[
    "not",
    "no",
    "never",
    "neither",
    "nobody",
    "nothing",
    "nor",
    "can't",
    "cant",
    "couldn't",
    "couldnt",
    "shouldn't",
    "shouldnt",
    "wouldn't",
    "wouldnt",
    "don't",
    "dont",
    "doesn't",
    "doesnt",
    "didn't",
    "didnt",
    "isn't",
    "isnt",
    "aren't",
    "arent",
    "wasn't",
    "wasnt",
    "weren't",
    "werent",
    "without",
    "lack",
    "lacking",
    "absent",
    "hardly",
    "barely",
    "scarcely",
    "little",
    "few",
    "nowhere",
    "nothing",
];

/// Intensifiers that amplify sentiment (multiplier > 1.0).
const INTENSIFIERS: &[(&str, f32)] = &[
    ("very", 1.5),
    ("really", 1.5),
    ("extremely", 2.0),
    ("incredibly", 2.0),
    ("absolutely", 2.0),
    ("totally", 1.8),
    ("completely", 1.8),
    ("utterly", 2.0),
    ("highly", 1.5),
    ("super", 1.5),
    ("so", 1.3),
    ("quite", 1.2),
    ("rather", 1.2),
    ("pretty", 1.2),
    ("damn", 1.8),
    ("fucking", 2.0),
    ("frigging", 1.8),
    ("bloody", 1.8),
    ("truly", 1.5),
    ("genuinely", 1.3),
    ("honestly", 1.3),
    ("actually", 1.2),
    ("especially", 1.5),
    ("particularly", 1.4),
    ("exceptionally", 2.0),
    ("remarkably", 1.8),
    ("extraordinarily", 2.0),
];

/// Sarcasm markers and patterns that indicate inverted meaning.
const SARCASM_PATTERNS: &[&str] = &[
    "oh great",
    "oh wonderful",
    "oh perfect",
    "oh good",
    "oh fantastic",
    "sure, because",
    "yeah right",
    "as if",
    "as though",
    "thanks, i hate it",
    "tanks, i hate it",
    "thx i hate it",
    "just what i needed",
    "exactly what i wanted",
    "because that's what i need",
    "because that's what i wanted",
    "thanks twitter",
    "thx twitter",
    "cool cool cool",
    "sure sure",
    "okay sure",
    "what could go wrong",
    "how hard could it be",
    "famous last words",
    "we'll see about that",
];

/// Calculate context-aware sentiment score for a word.
/// Combines base sentiment with negation and intensifier effects.
fn calculate_contextual_score(text: &str, base_score: f32, target_word: &str) -> f32 {
    let mut score = base_score;

    // Apply intensifier multiplier
    let multiplier = get_intensifier_multiplier(text, target_word);
    score *= multiplier;

    // Apply negation (flip polarity)
    if is_negated(text, target_word) {
        score = -score;
    }

    score
}

/// Detect if a word is negated in the given text.
fn is_negated(text: &str, target_word: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    let target_lower = target_word.to_lowercase();

    for (i, word) in words.iter().enumerate() {
        let word_lower = word.to_lowercase();
        if word_lower == target_lower {
            // Check up to 3 words before for negation
            let start = i.saturating_sub(3);
            if words
                .iter()
                .take(i)
                .skip(start)
                .any(|prev| NEGATION_PATTERNS.iter().any(|&n| prev.to_lowercase() == n))
            {
                return true;
            }
        }
    }
    false
}

/// Get the intensifier multiplier for a word.
fn get_intensifier_multiplier(text: &str, target_word: &str) -> f32 {
    let words: Vec<&str> = text.split_whitespace().collect();
    let target_lower = target_word.to_lowercase();

    for (i, word) in words.iter().enumerate() {
        let word_lower = word.to_lowercase();
        if word_lower == target_lower {
            // Check up to 2 words before for intensifier
            let start = i.saturating_sub(2);
            if let Some((_, multiplier)) = words.iter().take(i).skip(start).find_map(|prev| {
                INTENSIFIERS
                    .iter()
                    .find(|(intensifier, _)| prev.to_lowercase() == *intensifier)
            }) {
                return *multiplier;
            }
        }
    }
    1.0
}

/// Analyze overall contextual sentiment modifiers in text.
/// Returns a modifier score that adjusts the final sentiment.
fn analyze_contextual_modifiers(text: &str) -> f32 {
    let mut modifier = 0.0;

    // Sarcasm heavily penalizes sentiment
    if has_sarcasm_markers(text) {
        modifier -= 2.0;
    }

    // Excessive punctuation indicates strong emotion (usually negative in context)
    if is_excessive_punctuation(text) {
        modifier -= 0.5;
    }

    modifier
}

/// Detect sarcasm markers in text.
fn has_sarcasm_markers(text: &str) -> bool {
    let lower = text.to_lowercase();
    SARCASM_PATTERNS
        .iter()
        .any(|&pattern| lower.contains(pattern))
}

/// Detect excessive punctuation that may indicate sarcasm or strong emotion.
fn is_excessive_punctuation(text: &str) -> bool {
    let exclamation_count = text.matches('!').count();
    let question_count = text.matches('?').count();

    // Multiple ?! or !? combinations
    text.contains("?!") || text.contains("!?") || exclamation_count > 2 || question_count > 2
}

// ============================================================================
// Strategy Structs and Impls
// ============================================================================

/// Basic keyword-based sentiment strategy.
/// Uses predefined positive and negative word lists.
#[derive(Debug)]
pub struct BasicKeywordStrategy;

/// Contextual sentiment strategy.
/// Handles negation, intensifiers, and sarcasm detection.
#[derive(Debug)]
pub struct ContextStrategy;

/// Emoji sentiment strategy.
/// Analyzes emoji sentiment using the emoji lexicon.
#[derive(Debug)]
pub struct EmojiStrategy;

/// Domain-specific sentiment strategy.
/// Analyzes sentiment based on domain keywords (Tech, Crypto, Gaming, etc.).
#[derive(Debug)]
pub struct DomainStrategy;

impl SentimentStrategy for BasicKeywordStrategy {
    fn analyze(&self, text: &str) -> f32 {
        let mut score = 0.0;
        let lower = text.to_lowercase();

        // Count positive words
        for &word in POSITIVE_WORDS {
            if crate::utils::twitter::sentiment::utils::contains_word(&lower, word) {
                // Apply contextual modifiers
                let contextual_score = calculate_contextual_score(&lower, 1.0, word);
                score += contextual_score;
            }
        }

        // Count negative words
        for &word in NEGATIVE_WORDS {
            if crate::utils::twitter::sentiment::utils::contains_word(&lower, word) {
                let contextual_score = calculate_contextual_score(&lower, -1.0, word);
                score += contextual_score;
            }
        }

        score
    }
}

impl SentimentStrategy for ContextStrategy {
    fn analyze(&self, text: &str) -> f32 {
        analyze_contextual_modifiers(text)
    }
}

impl SentimentStrategy for EmojiStrategy {
    fn analyze(&self, text: &str) -> f32 {
        crate::utils::twitter::twitteractivity_sentiment_emoji::analyze_emoji_sentiment(text)
    }
}

impl SentimentStrategy for DomainStrategy {
    fn analyze(&self, text: &str) -> f32 {
        let domain = crate::utils::twitter::twitteractivity_sentiment_domains::detect_domain(text);
        crate::utils::twitter::twitteractivity_sentiment_domains::analyze_domain_sentiment(
            text, domain,
        )
    }
}

// ============================================================================
// Keyword Lists
// ============================================================================

const POSITIVE_WORDS: &[&str] = &[
    "good",
    "great",
    "awesome",
    "amazing",
    "excellent",
    "love",
    "like",
    "nice",
    "wonderful",
    "fantastic",
    "best",
    "happy",
    "glad",
    "joy",
    "cool",
    "brilliant",
    "thank",
    "thanks",
    "appreciate",
    "beautiful",
    "perfect",
    "ideal",
    "superb",
    "outstanding",
    "impressive",
    "enjoy",
    "fun",
    "yes",
    "win",
    "won",
    "celebrate",
    "congrats",
    "congratulations",
    "well done",
    "welldone",
    "spot on",
    "correct",
    "right",
    "smart",
    "wise",
    "kind",
    "friendly",
    "helpful",
    "support",
    "bless",
    "marvelous",
    "pleasure",
    "delighted",
    "thrilled",
    "excited",
    "yay",
    "😊",
    "❤️",
    "🔥",
    "💯",
    "👏",
];

const NEGATIVE_WORDS: &[&str] = &[
    "bad",
    "terrible",
    "awful",
    "worst",
    "hate",
    "dislike",
    "horrible",
    "disgusting",
    "poor",
    "sad",
    "angry",
    "mad",
    "upset",
    "annoyed",
    "disappointed",
    "fail",
    "failed",
    "failure",
    "wrong",
    "error",
    "mistake",
    "bug",
    "broken",
    "useless",
    "waste",
    "sucks",
    "sucked",
    "suck",
    "hell",
    "shit",
    "damn",
    "fuck",
    "fucking",
    "idiot",
    "stupid",
    "dumb",
    "ridiculous",
    "absurd",
    "fake",
    "scam",
    "liar",
    "lies",
    "lying",
    "toxic",
    "abuse",
    "abusive",
    "harassment",
    "harassing",
    "block",
    "report",
    "spam",
    "spammer",
    "clown",
    "joke",
    "pathetic",
    "disaster",
    "mess",
    "nightmare",
    "regret",
    "depressing",
    "depressed",
    "anxious",
    "anxiety",
    "cry",
    "crying",
    "😢",
    "😡",
    "💩",
];

/// Simple sentiment polarity with enhanced scoring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
}

/// Thread context information for sentiment analysis.
#[derive(Debug, Clone)]
pub struct ThreadContext {
    /// Number of replies in the thread
    pub reply_count: u32,
    /// Average sentiment of replies
    pub avg_reply_sentiment: f32,
    /// Whether this tweet is a reply itself
    pub is_reply: bool,
    /// Whether this tweet is a quote
    pub is_quote: bool,
    /// Thread depth (how many levels deep in conversation)
    pub thread_depth: u32,
    /// Conversation flow indicators
    pub conversation_indicators: Vec<ConversationIndicator>,
}

/// Indicators of conversation flow and context.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConversationIndicator {
    Agreement,
    Disagreement,
    Question,
    Clarification,
    Humor,
    Sarcasm,
    Support,
    Criticism,
}

/// User reputation metrics for sentiment analysis.
#[derive(Debug, Clone)]
pub struct UserReputation {
    /// Follower count
    pub follower_count: u32,
    /// Whether the account is verified
    pub is_verified: bool,
    /// Account age in days (approximate)
    pub account_age_days: u32,
    /// Recent engagement rate (likes/retweets per tweet)
    pub engagement_rate: f32,
    /// Whether user is considered influential in their domain
    pub is_influential: bool,
    /// Trust score based on various factors (0.0-1.0)
    pub trust_score: f32,
}

/// Temporal factors affecting sentiment.
#[derive(Debug, Clone)]
pub struct TemporalFactors {
    /// Hour of day (0-23)
    pub hour_of_day: u8,
    /// Day of week (0=Monday, 6=Sunday)
    pub day_of_week: u8,
    /// How recent the tweet is (hours ago)
    pub hours_since_post: f32,
    /// Whether this time period has high activity
    pub is_peak_hour: bool,
    /// Current trending topics sentiment bias
    pub trending_bias: f32,
}

/// Enhanced sentiment analysis result with detailed scoring.
#[derive(Debug, Clone)]
pub struct EnhancedSentimentResult {
    /// Base sentiment from text analysis
    pub base_sentiment: Sentiment,
    /// Final sentiment after all enhancements
    pub final_sentiment: Sentiment,
    /// Base sentiment score
    pub base_score: f32,
    /// Final sentiment score after modifications
    pub final_score: f32,
    /// Confidence in the sentiment analysis (0.0-1.0)
    pub confidence: f32,
    /// Breakdown of score contributions
    pub score_breakdown: ScoreBreakdown,
}

/// Breakdown of sentiment score contributions.
#[derive(Debug, Clone, Default)]
pub struct ScoreBreakdown {
    pub text_score: f32,
    pub emoji_score: f32,
    pub domain_score: f32,
    pub context_score: f32,
    pub reputation_score: f32,
    pub temporal_score: f32,
}

/// Configuration for sentiment analysis.
#[derive(Debug, Clone)]
pub struct SentimentConfig {
    /// Whether to use basic keyword analysis
    pub use_basic_keywords: bool,
    /// Whether to use contextual analysis (negation, intensifiers, sarcasm)
    pub use_context: bool,
    /// Whether to use emoji analysis
    pub use_emoji: bool,
    /// Whether to use domain-specific analysis
    pub use_domain: bool,
    /// Whether to use LLM analysis (requires LLM client)
    pub use_llm: bool,
    /// Minimum confidence threshold for LLM results (0.0-1.0)
    pub llm_min_confidence: f32,
    /// Probability of using LLM when enabled (0.0-1.0)
    pub llm_probability: f32,
}

impl Default for SentimentConfig {
    fn default() -> Self {
        Self {
            use_basic_keywords: true,
            use_context: true,
            use_emoji: true,
            use_domain: true,
            use_llm: false,
            llm_min_confidence: 0.7,
            llm_probability: 0.5,
        }
    }
}

/// Unified sentiment analyzer with configurable strategies.
pub struct SentimentAnalyzer {
    config: SentimentConfig,
    llm_client: Option<LlmClient>,
    strategies: Vec<Box<dyn SentimentStrategy>>,
}

impl std::fmt::Debug for SentimentAnalyzer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SentimentAnalyzer")
            .field("config", &self.config)
            .field("llm_client", &self.llm_client.as_ref().map(|_| "LlmClient"))
            .field("strategies", &self.strategies)
            .finish()
    }
}

impl SentimentAnalyzer {
    /// Create a new sentiment analyzer with default configuration.
    pub fn new() -> Self {
        Self::with_config(SentimentConfig::default())
    }

    /// Create a sentiment analyzer with custom configuration.
    pub fn with_config(config: SentimentConfig) -> Self {
        let mut strategies: Vec<Box<dyn SentimentStrategy>> = Vec::new();

        if config.use_basic_keywords {
            strategies.push(Box::new(BasicKeywordStrategy));
        }

        if config.use_context {
            strategies.push(Box::new(ContextStrategy));
        }

        if config.use_emoji {
            strategies.push(Box::new(EmojiStrategy));
        }

        if config.use_domain {
            strategies.push(Box::new(DomainStrategy));
        }

        Self {
            config,
            llm_client: None,
            strategies,
        }
    }

    /// Set the LLM client for enhanced analysis.
    pub fn with_llm_client(mut self, llm_client: LlmClient) -> Self {
        self.llm_client = Some(llm_client);
        self
    }

    /// Analyze the sentiment of a tweet's text content.
    ///
    /// # Arguments
    /// * `text` - The tweet text to analyze
    ///
    /// # Returns
    /// A `Sentiment` enum with the determined polarity
    #[instrument]
    pub async fn analyze_sentiment(&self, text: &str) -> Sentiment {
        let mut total_score = 0.0;

        // Apply all configured strategies
        for strategy in &self.strategies {
            total_score += strategy.analyze(text);
        }

        // Check for LLM analysis
        if self.config.use_llm {
            if let Some(llm) = &self.llm_client {
                let llm_sentiment = twitteractivity_sentiment_llm::analyze_sentiment_hybrid(
                    Some(llm),
                    text,
                    self.config.llm_probability,
                    self.config.llm_min_confidence,
                )
                .await;

                // Convert to score and add with weight
                let llm_score = sentiment_to_score(llm_sentiment);
                total_score += llm_score * 0.5; // Weight LLM results
            }
        }

        // Classify based on total score with hysteresis
        if total_score > 1.0 {
            Sentiment::Positive
        } else if total_score < -1.0 {
            Sentiment::Negative
        } else {
            Sentiment::Neutral
        }
    }

    /// Analyze sentiment synchronously (without LLM).
    /// Use this for performance-critical paths where async is not available.
    pub fn analyze_sentiment_sync(&self, text: &str) -> Sentiment {
        let mut total_score = 0.0;

        // Apply all configured strategies
        for strategy in &self.strategies {
            total_score += strategy.analyze(text);
        }

        // Classify based on total score
        if total_score > 1.0 {
            Sentiment::Positive
        } else if total_score < -1.0 {
            Sentiment::Negative
        } else {
            Sentiment::Neutral
        }
    }

    /// Analyze sentiment with enhanced contextual understanding.
    /// Includes thread context, user reputation, and temporal factors.
    ///
    /// # Arguments
    /// * `tweet_text` - The tweet text to analyze
    /// * `thread_context` - Optional thread context information
    /// * `user_reputation` - Optional user reputation metrics
    /// * `temporal_factors` - Optional temporal factors
    ///
    /// # Returns
    /// EnhancedSentimentResult with detailed analysis
    pub fn analyze_enhanced(
        &self,
        tweet_text: &str,
        thread_context: Option<&ThreadContext>,
        user_reputation: Option<&UserReputation>,
        temporal_factors: Option<&TemporalFactors>,
    ) -> EnhancedSentimentResult {
        // Get base sentiment analysis
        let base_sentiment = self.analyze_sentiment_sync(tweet_text);
        let base_score = sentiment_to_score(base_sentiment);

        // Initialize score breakdown
        let mut breakdown = ScoreBreakdown {
            text_score: base_score,
            emoji_score: 0.0,
            domain_score: 0.0,
            context_score: 0.0,
            reputation_score: 0.0,
            temporal_score: 0.0,
        };

        // Apply contextual modifiers
        let mut final_score = base_score;

        // Thread context analysis
        if let Some(context) = thread_context {
            let context_modifier = self.analyze_thread_context(context);
            breakdown.context_score = context_modifier;
            final_score += context_modifier;
        }

        // User reputation analysis
        if let Some(reputation) = user_reputation {
            let reputation_modifier = self.analyze_user_reputation(reputation);
            breakdown.reputation_score = reputation_modifier;
            final_score += reputation_modifier;
        }

        // Temporal factors analysis
        if let Some(temporal) = temporal_factors {
            let temporal_modifier = self.analyze_temporal_factors(temporal);
            breakdown.temporal_score = temporal_modifier;
            final_score += temporal_modifier;
        }

        // Calculate final sentiment
        let final_sentiment = score_to_sentiment(final_score);

        // Calculate confidence based on score magnitude and factor agreement
        let confidence = self.calculate_confidence(&breakdown, base_score, final_score);

        EnhancedSentimentResult {
            base_sentiment,
            final_sentiment,
            base_score,
            final_score,
            confidence,
            score_breakdown: breakdown,
        }
    }

    /// Analyze thread context and return sentiment modifier.
    fn analyze_thread_context(&self, context: &ThreadContext) -> f32 {
        let mut modifier = 0.0;

        // Reply sentiment influence with diminishing returns
        if context.reply_count > 0 {
            let reply_weight = if context.reply_count <= 5 {
                0.3 // Full weight for small threads
            } else if context.reply_count <= 20 {
                0.2 // Reduced weight for medium threads
            } else {
                0.1 // Minimal weight for large threads (avoid noise)
            };

            let reply_influence = context.avg_reply_sentiment * reply_weight;
            modifier += reply_influence;

            // High reply count indicates engagement (positive modifier)
            if context.reply_count > 10 {
                modifier += 0.1;
            }
        }

        // Thread depth analysis
        let depth_modifier = match context.thread_depth {
            0 => 0.0,       // Original tweet
            1..=2 => 0.05,  // Shallow conversation
            3..=5 => 0.1,   // Moderate depth
            6..=10 => 0.15, // Deep conversation
            _ => 0.2,       // Very deep threads (often controversial)
        };
        modifier += depth_modifier;

        // Reply vs original tweet dynamics
        if context.is_reply {
            modifier += 0.08; // Replies tend to be more opinionated
        }

        // Quote tweets analysis
        if context.is_quote {
            modifier += 0.12; // Quotes often share positive sentiment
        }

        // Conversation flow analysis with intensity scaling
        let indicator_count = context.conversation_indicators.len() as f32;
        let indicator_weight = if indicator_count > 0.0 {
            (5.0 / indicator_count).min(1.0) // Scale down for many indicators
        } else {
            1.0
        };

        for indicator in &context.conversation_indicators {
            let base_modifier = match indicator {
                ConversationIndicator::Agreement => 0.08,
                ConversationIndicator::Disagreement => -0.08,
                ConversationIndicator::Question => 0.04, // Questions show engagement
                ConversationIndicator::Clarification => 0.06, // Clarifications are helpful
                ConversationIndicator::Humor => 0.1,     // Humor is positive
                ConversationIndicator::Sarcasm => -0.15, // Sarcasm reduces positivity
                ConversationIndicator::Support => 0.12,  // Support is strongly positive
                ConversationIndicator::Criticism => -0.12, // Criticism is negative
            };
            modifier += base_modifier * indicator_weight;
        }

        modifier
    }

    /// Analyze user reputation and return sentiment modifier.
    fn analyze_user_reputation(&self, reputation: &UserReputation) -> f32 {
        let mut modifier = 0.0;

        // Verification status - significant credibility boost
        if reputation.is_verified {
            modifier += 0.12;
        }

        // Follower count influence with logarithmic scaling
        let follower_modifier = if reputation.follower_count == 0 {
            -0.1 // No followers suspicious
        } else {
            let log_followers = (reputation.follower_count as f32).ln().max(0.0);
            let scaled_modifier = (log_followers * 0.03).min(0.2); // Max 0.2 for very large accounts
            if reputation.follower_count < 50 {
                scaled_modifier - 0.05 // Small accounts penalty
            } else {
                scaled_modifier
            }
        };
        modifier += follower_modifier;

        // Account age with stability consideration
        let age_modifier = if reputation.account_age_days < 7 {
            -0.15 // Very new accounts
        } else if reputation.account_age_days < 30 {
            -0.08 // New accounts
        } else if reputation.account_age_days < 90 {
            -0.03 // Relatively new
        } else if reputation.account_age_days < 365 {
            0.02 // Established
        } else if reputation.account_age_days < 1095 {
            0.05 // Mature accounts
        } else {
            0.08 // Very established accounts
        };
        modifier += age_modifier;

        // Engagement rate analysis
        let engagement_modifier = if reputation.engagement_rate > 0.2 {
            0.08 // Highly engaged - authentic
        } else if reputation.engagement_rate > 0.1 {
            0.04 // Moderately engaged
        } else if reputation.engagement_rate > 0.05 {
            0.0 // Normal engagement
        } else if reputation.engagement_rate > 0.01 {
            -0.02 // Low engagement - potentially suspicious
        } else {
            -0.08 // Very low engagement - likely bot/spam
        };
        modifier += engagement_modifier;

        // Influential status in domain
        if reputation.is_influential {
            modifier += 0.15; // Significant boost for domain experts
        }

        // Trust score integration with diminishing returns
        let trust_contribution = if reputation.trust_score > 0.8 {
            0.15 // High trust
        } else if reputation.trust_score > 0.6 {
            0.08 // Good trust
        } else if reputation.trust_score > 0.4 {
            0.0 // Neutral trust
        } else if reputation.trust_score > 0.2 {
            -0.05 // Low trust
        } else {
            -0.1 // Very low trust
        };
        modifier += trust_contribution;

        modifier
    }

    /// Analyze temporal factors and return sentiment modifier.
    fn analyze_temporal_factors(&self, temporal: &TemporalFactors) -> f32 {
        let mut modifier = 0.0;

        // Time of day emotional patterns (based on psychological research)
        let hour_modifier = match temporal.hour_of_day {
            6..=9 => 0.08,    // Early morning - optimistic start
            10..=12 => 0.05,  // Late morning - productive
            13..=15 => 0.02,  // Early afternoon - neutral
            16..=18 => -0.01, // Late afternoon - winding down
            19..=21 => 0.03,  // Evening - social/relaxed
            22..=23 => -0.03, // Late evening - tired
            0..=3 => -0.08,   // Late night - emotional lows
            4..=5 => -0.05,   // Very early morning - fatigue
            _ => 0.0,         // Default for any invalid hours
        };
        modifier += hour_modifier;

        // Day of week sentiment patterns
        let day_modifier = match temporal.day_of_week {
            0 => 0.02, // Monday - fresh start
            1 => 0.01, // Tuesday - routine
            2 => 0.0,  // Wednesday - midweek
            3 => 0.01, // Thursday - anticipation
            4 => 0.04, // Friday - excitement
            5 => 0.06, // Saturday - leisure
            6 => 0.05, // Sunday - relaxation/wind down
            _ => 0.0,
        };
        modifier += day_modifier;

        // Recency and freshness analysis
        let recency_modifier = if temporal.hours_since_post < 0.5 {
            0.12 // Very recent (<30 min) - immediate reactions
        } else if temporal.hours_since_post < 2.0 {
            0.08 // Recent (<2 hours) - still fresh
        } else if temporal.hours_since_post < 6.0 {
            0.04 // Somewhat recent (<6 hours)
        } else if temporal.hours_since_post < 24.0 {
            0.02 // Same day
        } else if temporal.hours_since_post < 72.0 {
            0.0 // Few days old
        } else {
            -0.02 // Older tweets - less emotional impact
        };
        modifier += recency_modifier;

        // Peak activity periods
        if temporal.is_peak_hour {
            modifier += 0.03; // Peak hours amplify existing sentiment
        }

        // Trending topic influence
        let trending_modifier = temporal.trending_bias * 0.08; // Scale trending influence
        modifier += trending_modifier;

        modifier
    }

    /// Calculate confidence score for the sentiment analysis.
    fn calculate_confidence(
        &self,
        breakdown: &ScoreBreakdown,
        base_score: f32,
        final_score: f32,
    ) -> f32 {
        let mut confidence = 0.5; // Base confidence

        // Score magnitude indicates strength
        let score_magnitude = final_score.abs();
        confidence += (score_magnitude * 0.2).min(0.2);

        // Agreement between factors increases confidence
        let factor_agreement = self.calculate_factor_agreement(breakdown);
        confidence += factor_agreement * 0.2;

        // Distance from base score (large modifications decrease confidence)
        let modification_distance = (final_score - base_score).abs();
        confidence -= (modification_distance * 0.1).min(0.1);

        confidence.clamp(0.0, 1.0)
    }

    /// Calculate agreement between different scoring factors.
    fn calculate_factor_agreement(&self, breakdown: &ScoreBreakdown) -> f32 {
        let factors = vec![
            breakdown.text_score,
            breakdown.emoji_score,
            breakdown.domain_score,
            breakdown.context_score,
            breakdown.reputation_score,
            breakdown.temporal_score,
        ];

        let non_zero_factors: Vec<f32> = factors.into_iter().filter(|&x| x != 0.0).collect();

        if non_zero_factors.len() < 2 {
            return 0.0;
        }

        // Check how many factors agree on sentiment direction
        let positive_factors = non_zero_factors.iter().filter(|&&x| x > 0.0).count();
        let negative_factors = non_zero_factors.iter().filter(|&&x| x < 0.0).count();
        let total_factors = non_zero_factors.len();

        if positive_factors > negative_factors {
            positive_factors as f32 / total_factors as f32
        } else if negative_factors > positive_factors {
            negative_factors as f32 / total_factors as f32
        } else {
            0.5 // Equal split
        }
    }
}

/// Weighted sentiment score: positive = +1, neutral = 0, negative = -1.
/// Can be aggregated across multiple tweets.
pub fn sentiment_score(sentiment: Sentiment) -> i32 {
    match sentiment {
        Sentiment::Positive => 1,
        Sentiment::Neutral => 0,
        Sentiment::Negative => -1,
    }
}

/// Extracts text content from a tweet object and analyzes it.
/// The tweet object should contain a `text` field or nested text content.
pub async fn analyze_tweet_sentiment(analyzer: &SentimentAnalyzer, tweet_obj: &Value) -> Sentiment {
    let text = extract_tweet_text(tweet_obj);
    analyzer.analyze_sentiment(&text).await
}

/// Extracts raw text from a tweet JSON object.
/// Handles multiple possible text field locations.
fn extract_tweet_text(tweet_obj: &Value) -> String {
    if let Some(text) = tweet_obj.get("text").and_then(|v: &Value| v.as_str()) {
        return text.to_string();
    }
    if let Some(full_text) = tweet_obj.get("full_text").and_then(|v: &Value| v.as_str()) {
        return full_text.to_string();
    }
    if let Some(obj) = tweet_obj.as_object() {
        // Check nested legacy structure sometimes returned by Twitter
        if let Some(retweeted) = obj.get("retweeted_status") {
            return extract_tweet_text(retweeted);
        }
    }
    // Fallback: stringify the object and take first 280 chars
    truncate_chars(&tweet_obj.to_string(), 280)
}

/// Collects sentiment statistics for a batch of tweets.
#[derive(Debug, Clone, Default)]
pub struct SentimentStats {
    pub positive: u32,
    pub neutral: u32,
    pub negative: u32,
}

impl SentimentStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, sentiment: Sentiment) {
        match sentiment {
            Sentiment::Positive => self.positive += 1,
            Sentiment::Neutral => self.neutral += 1,
            Sentiment::Negative => self.negative += 1,
        }
    }

    pub fn dominant(&self) -> Sentiment {
        if self.positive >= self.neutral && self.positive >= self.negative {
            Sentiment::Positive
        } else if self.negative >= self.neutral && self.negative >= self.positive {
            Sentiment::Negative
        } else {
            Sentiment::Neutral
        }
    }

    pub fn total(&self) -> u32 {
        self.positive + self.neutral + self.negative
    }
}

/// Returns a composite sentiment score for the feed window.
/// Range: -1.0 to +1.0, normalized by count.
pub fn feed_sentiment_score(stats: &SentimentStats) -> f64 {
    let total = stats.total() as f64;
    if total == 0.0 {
        return 0.0;
    }
    let pos = stats.positive as f64 / total;
    let neg = stats.negative as f64 / total;
    pos - neg
}

/// Convert sentiment to numerical score.
fn sentiment_to_score(sentiment: Sentiment) -> f32 {
    match sentiment {
        Sentiment::Positive => 1.0,
        Sentiment::Neutral => 0.0,
        Sentiment::Negative => -1.0,
    }
}

/// Convert numerical score to sentiment enum.
fn score_to_sentiment(score: f32) -> Sentiment {
    if score > 0.3 {
        Sentiment::Positive
    } else if score < -0.3 {
        Sentiment::Negative
    } else {
        Sentiment::Neutral
    }
}

/// Synchronous sentiment analysis using default configuration.
/// For simple analysis without async context.
pub fn analyze_sentiment_sync(text: &str) -> Sentiment {
    SentimentAnalyzer::new().analyze_sentiment_sync(text)
}

/// Extract thread context from tweet data scraped from DOM.
/// Returns None if critical data is missing or malformed.
pub fn extract_thread_context(tweet_obj: &Value) -> Option<ThreadContext> {
    // Extract reply count from replies array if available
    let reply_count = if let Some(replies) = tweet_obj.get("replies").and_then(|v| v.as_array()) {
        replies.len() as u32
    } else {
        0
    };

    // For DOM-scraped data, we can't determine if it's a reply or quote directly
    // These would need to be inferred from the tweet text or additional DOM analysis
    let is_reply = false; // DOM scraping doesn't provide this info reliably
    let is_quote = false; // DOM scraping doesn't provide this info reliably

    // Thread depth is not available in DOM-scraped data
    let thread_depth = 0;

    // Extract and analyze reply sentiments
    let mut reply_sentiments = Vec::new();
    let mut _valid_replies = 0;

    if let Some(replies) = tweet_obj.get("replies").and_then(|v| v.as_array()) {
        for reply in replies {
            // Validate reply structure
            match (reply.get("author"), reply.get("text")) {
                (Some(author), Some(text)) => {
                    if let (Some(author_str), Some(text_str)) = (author.as_str(), text.as_str()) {
                        if !author_str.trim().is_empty() && !text_str.trim().is_empty() {
                            let sentiment = analyze_sentiment_sync(text_str);
                            reply_sentiments.push(sentiment_to_score(sentiment));
                            _valid_replies += 1;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    let avg_reply_sentiment = if reply_sentiments.is_empty() {
        0.0
    } else {
        reply_sentiments.iter().sum::<f32>() / reply_sentiments.len() as f32
    };

    // Detect conversation indicators from tweet text
    let tweet_text = extract_tweet_text(tweet_obj);
    let conversation_indicators = detect_conversation_indicators(&tweet_text);

    Some(ThreadContext {
        reply_count,
        avg_reply_sentiment,
        is_reply,
        is_quote,
        thread_depth,
        conversation_indicators,
    })
}

/// Extract user reputation from tweet data.
/// Note: DOM-scraped data has limited user information, so this provides minimal/default values with warnings.
pub fn extract_user_reputation(tweet_obj: &Value) -> Option<UserReputation> {
    // DOM-scraped data doesn't include full user objects
    // We only have basic tweet information, so user reputation analysis is limited

    log::warn!("User reputation: DOM-scraped tweet data lacks user profile information for tweet {:?}. Using default values.", tweet_obj.get("id"));

    // Since we don't have actual user data, return a neutral/default reputation
    // In a real implementation with API access, this would extract from user object
    let follower_count = 1000; // Default moderate follower count
    let is_verified = false; // Assume not verified by default
    let account_age_days = 365; // Default 1 year account age
    let engagement_rate = 0.05; // Default moderate engagement
    let is_influential = false; // Cannot determine from DOM data
    let trust_score = 0.5; // Neutral trust score

    Some(UserReputation {
        follower_count,
        is_verified,
        account_age_days,
        engagement_rate,
        is_influential,
        trust_score,
    })
}

/// Extract temporal factors from tweet data.
/// Note: DOM-scraped data doesn't include timestamps, so temporal analysis is limited.
pub fn extract_temporal_factors(tweet_obj: &Value) -> Option<TemporalFactors> {
    // DOM-scraped data doesn't include created_at timestamps
    // Temporal analysis would require API access or additional DOM scraping

    log::warn!("Temporal factors: DOM-scraped tweet data lacks timestamp information for tweet {:?}. Using neutral defaults.", tweet_obj.get("id"));

    // Return neutral/default temporal factors since we don't have actual timestamp data
    Some(TemporalFactors {
        hour_of_day: 12,        // Noon - neutral time
        day_of_week: 1,         // Tuesday - neutral weekday
        hours_since_post: 24.0, // 1 day ago - moderate recency
        is_peak_hour: true,     // Assume peak hours for neutral bias
        trending_bias: 0.0,     // No trending bias without data
    })
}

/// Detect conversation indicators from text content.
pub fn detect_conversation_indicators(text: &str) -> Vec<ConversationIndicator> {
    let lower = text.to_lowercase();
    let mut indicators = Vec::new();

    // Agreement indicators
    if AGREEMENT_PATTERNS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Agreement);
    }

    // Disagreement indicators
    if DISAGREEMENT_PATTERNS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Disagreement);
    }

    // Question indicators
    if QUESTION_PATTERNS.iter().any(|&p| lower.contains(p)) || text.contains('?') {
        indicators.push(ConversationIndicator::Question);
    }

    // Clarification indicators
    if CLARIFICATION_PATTERNS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Clarification);
    }

    // Humor indicators (basic detection)
    if HUMOR_PATTERNS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Humor);
    }

    // Support indicators
    if SUPPORT_PATTERNS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Support);
    }

    // Criticism indicators
    if CRITICISM_PATTERNS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Criticism);
    }

    // Sarcasm detection (more complex, placeholder for now)
    if SARCASM_INDICATORS.iter().any(|&p| lower.contains(p)) {
        indicators.push(ConversationIndicator::Sarcasm);
    }

    indicators
}

// Conversation pattern constants
const AGREEMENT_PATTERNS: &[&str] = &[
    "i agree",
    "totally agree",
    "absolutely",
    "exactly",
    "you're right",
    "well said",
    "couldn't agree more",
    "same here",
    "me too",
    "agreed",
    "spot on",
    "nail on the head",
    "perfectly said",
    "yes!",
    "definitely",
];

const DISAGREEMENT_PATTERNS: &[&str] = &[
    "i disagree",
    "totally disagree",
    "you're wrong",
    "not sure",
    "doubt it",
    "that's not right",
    "i think differently",
    "no way",
    "not really",
    "that's incorrect",
    "false",
    "wrong",
    "disagree with",
    "opposed to",
];

const QUESTION_PATTERNS: &[&str] = &[
    "what if",
    "how come",
    "why is",
    "what do you",
    "can you explain",
    "what's your take",
    "curious about",
    "wondering",
    "any thoughts",
    "what about",
    "how do",
    "when will",
];

const CLARIFICATION_PATTERNS: &[&str] = &[
    "to clarify",
    "let me explain",
    "what i mean",
    "in other words",
    "to be clear",
    "just to clarify",
    "for clarity",
    "to elaborate",
    "more precisely",
    "put it this way",
];

const HUMOR_PATTERNS: &[&str] = &[
    "lol",
    "haha",
    "😂",
    "🤣",
    "joke",
    "funny",
    "hilarious",
    "that's funny",
    "made me laugh",
    "rofl",
    "lmao",
];

const SUPPORT_PATTERNS: &[&str] = &[
    "i support",
    "good luck",
    "keep going",
    "you're doing great",
    "proud of you",
    "rooting for you",
    "stand with you",
    "got your back",
    "here for you",
    "cheering you on",
];

const CRITICISM_PATTERNS: &[&str] = &[
    "that's bad",
    "you shouldn't",
    "that's wrong",
    "disappointing",
    "terrible",
    "awful",
    "horrible",
    "worst",
    "pathetic",
    "ridiculous",
    "unacceptable",
    "shameful",
    "disgraceful",
];

const SARCASM_INDICATORS: &[&str] = &[
    "oh sure",
    "yeah right",
    "as if",
    "oh please",
    "oh come on",
    "whatever",
    "oh brother",
    "give me a break",
    "oh wow",
    "amazing",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sentiment_analyzer_basic() {
        let analyzer = SentimentAnalyzer::new();
        let sentiment = analyzer.analyze_sentiment("This is amazing!").await;
        assert_eq!(sentiment, Sentiment::Positive);
    }

    #[tokio::test]
    async fn test_sentiment_analyzer_negative() {
        let analyzer = SentimentAnalyzer::new();
        let sentiment = analyzer.analyze_sentiment("This is terrible!").await;
        assert_eq!(sentiment, Sentiment::Negative);
    }

    #[tokio::test]
    async fn test_sentiment_analyzer_neutral() {
        let analyzer = SentimentAnalyzer::new();
        let sentiment = analyzer.analyze_sentiment("This is okay.").await;
        assert_eq!(sentiment, Sentiment::Neutral);
    }

    #[test]
    fn test_sentiment_score() {
        assert_eq!(sentiment_score(Sentiment::Positive), 1);
        assert_eq!(sentiment_score(Sentiment::Neutral), 0);
        assert_eq!(sentiment_score(Sentiment::Negative), -1);
    }

    #[test]
    fn test_sentiment_stats() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Negative);
        stats.add(Sentiment::Neutral);

        assert_eq!(stats.positive, 1);
        assert_eq!(stats.negative, 1);
        assert_eq!(stats.neutral, 1);
        assert_eq!(stats.total(), 3);
        assert_eq!(stats.dominant(), Sentiment::Neutral);
    }

    #[test]
    fn test_feed_sentiment_score() {
        let mut stats = SentimentStats::new();
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Positive);
        stats.add(Sentiment::Negative);

        let score = feed_sentiment_score(&stats);
        assert!((score - 0.333333).abs() < 0.01); // (2-1)/3 = 1/3
    }
}
