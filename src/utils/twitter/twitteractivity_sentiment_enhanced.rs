//! Enhanced sentiment analysis with contextual understanding.
//! Incorporates thread context, user reputation, and temporal factors beyond basic keyword matching.

use crate::utils::twitter::twitteractivity_sentiment::{analyze_sentiment, Sentiment};
use serde_json::Value;

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

/// Enhanced sentiment analyzer with contextual understanding.
pub struct EnhancedSentimentAnalyzer {
    /// Base sentiment analyzer (placeholder for future integration)
    #[allow(dead_code)]
    base_analyzer: (),
    /// Context analysis enabled
    context_enabled: bool,
    /// Reputation analysis enabled
    reputation_enabled: bool,
    /// Temporal analysis enabled
    temporal_enabled: bool,
    /// Configuration weights (placeholder for future weighted scoring)
    #[allow(dead_code)]
    weights: SentimentWeights,
}

/// Configuration weights for different sentiment factors.
#[derive(Debug, Clone)]
pub struct SentimentWeights {
    pub context_weight: f32,
    pub reputation_weight: f32,
    pub temporal_weight: f32,
    pub base_weight: f32,
}

impl Default for SentimentWeights {
    fn default() -> Self {
        Self {
            context_weight: 0.2,
            reputation_weight: 0.15,
            temporal_weight: 0.1,
            base_weight: 0.55,
        }
    }
}

impl Default for EnhancedSentimentAnalyzer {
    fn default() -> Self {
        Self {
            base_analyzer: (),
            context_enabled: true,
            reputation_enabled: true,
            temporal_enabled: true,
            weights: SentimentWeights::default(),
        }
    }
}

impl EnhancedSentimentAnalyzer {
    /// Create a new enhanced sentiment analyzer with custom configuration.
    pub fn new(
        context_enabled: bool,
        reputation_enabled: bool,
        temporal_enabled: bool,
        weights: SentimentWeights,
    ) -> Self {
        Self {
            base_analyzer: (),
            context_enabled,
            reputation_enabled,
            temporal_enabled,
            weights,
        }
    }

    /// Analyze sentiment with full contextual understanding.
    pub fn analyze_enhanced(
        &self,
        tweet_text: &str,
        thread_context: Option<&ThreadContext>,
        user_reputation: Option<&UserReputation>,
        temporal_factors: Option<&TemporalFactors>,
    ) -> EnhancedSentimentResult {
        // Get base sentiment analysis
        let base_sentiment = analyze_sentiment(tweet_text);
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
        let mut applied_modifiers = 0;

        // Thread context analysis
        if self.context_enabled {
            if let Some(context) = thread_context {
                let context_modifier = self.analyze_thread_context(context);
                breakdown.context_score = context_modifier;
                final_score += context_modifier;
                applied_modifiers += 1;
            } else {
                log::debug!("Enhanced sentiment: Thread context not available for analysis");
            }
        }

        // User reputation analysis
        if self.reputation_enabled {
            if let Some(reputation) = user_reputation {
                let reputation_modifier = self.analyze_user_reputation(reputation);
                breakdown.reputation_score = reputation_modifier;
                final_score += reputation_modifier;
                applied_modifiers += 1;
            } else {
                log::debug!("Enhanced sentiment: User reputation not available for analysis");
            }
        }

        // Temporal factors analysis
        if self.temporal_enabled {
            if let Some(temporal) = temporal_factors {
                let temporal_modifier = self.analyze_temporal_factors(temporal);
                breakdown.temporal_score = temporal_modifier;
                final_score += temporal_modifier;
                applied_modifiers += 1;
            } else {
                log::debug!("Enhanced sentiment: Temporal factors not available for analysis");
            }
        }

        // Log if no contextual modifiers were applied
        if applied_modifiers == 0
            && (self.context_enabled || self.reputation_enabled || self.temporal_enabled)
        {
            log::warn!("Enhanced sentiment: No contextual modifiers applied - all data sources unavailable");
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

            // Check if reply is in agreement/disagreement with parent
            // This would require analyzing the parent tweet sentiment
            // For now, assume neutral contribution
        }

        // Quote tweets analysis
        if context.is_quote {
            modifier += 0.12; // Quotes often share positive sentiment

            // Additional analysis could check quote context
            // (e.g., "Check this out" vs "Can you believe this?")
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

        // Thread polarization check
        if context.reply_count > 3 {
            let polarization = self.calculate_thread_polarization(context);
            modifier += polarization * 0.1; // Amplify polarized threads
        }

        modifier
    }

    /// Calculate thread polarization based on reply sentiment variance.
    fn calculate_thread_polarization(&self, context: &ThreadContext) -> f32 {
        // In a real implementation, this would analyze the variance in reply sentiments
        // High variance = polarized discussion, low variance = consensus
        // For now, return a placeholder based on reply count and average sentiment
        if context.reply_count > 10 && context.avg_reply_sentiment.abs() > 0.5 {
            0.3 // Highly polarized thread
        } else if context.reply_count > 5 && context.avg_reply_sentiment.abs() > 0.3 {
            0.1 // Moderately polarized
        } else {
            0.0 // Neutral
        }
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

        // Interaction between factors
        if reputation.is_verified && reputation.follower_count > 10000 {
            modifier += 0.05; // Verified large accounts get extra credibility
        }

        if reputation.account_age_days > 365 && reputation.engagement_rate > 0.1 {
            modifier += 0.03; // Established, engaged accounts are very trustworthy
        }

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

        // Seasonal/contextual factors could be added here
        // e.g., holiday periods, major events, etc.

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

/// Convert sentiment enum to numerical score.
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

/// Extract tweet text from tweet object (helper function).
/// Validates the text field and logs warnings for missing or invalid data.
fn extract_tweet_text(tweet_obj: &Value) -> String {
    let tweet_id = tweet_obj
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    if let Some(text) = tweet_obj.get("text").or_else(|| tweet_obj.get("full_text")) {
        if let Some(text_str) = text.as_str() {
            let trimmed = text_str.trim();
            if trimmed.is_empty() {
                log::warn!("Tweet text: Empty text content for tweet {}", tweet_id);
                return String::new();
            }
            if trimmed.len() > 5000 {
                log::warn!(
                    "Tweet text: Unusually long text ({} chars) for tweet {}, truncating",
                    trimmed.len(),
                    tweet_id
                );
                return trimmed.chars().take(5000).collect();
            }
            return trimmed.to_string();
        } else {
            log::warn!(
                "Tweet text: Text field is not a string for tweet {}",
                tweet_id
            );
        }
    } else {
        log::warn!(
            "Tweet text: Missing 'text' or 'full_text' field for tweet {}",
            tweet_id
        );
    }

    String::new()
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
    let mut valid_replies = 0;

    if let Some(replies) = tweet_obj.get("replies").and_then(|v| v.as_array()) {
        for reply in replies {
            // Validate reply structure
            match (reply.get("author"), reply.get("text")) {
                (Some(author), Some(text)) => {
                    if let (Some(author_str), Some(text_str)) = (author.as_str(), text.as_str()) {
                        if !author_str.trim().is_empty() && !text_str.trim().is_empty() {
                            let sentiment = analyze_sentiment(text_str);
                            reply_sentiments.push(sentiment_to_score(sentiment));
                            valid_replies += 1;
                        } else {
                            log::warn!(
                                "Thread context: Empty author or text in reply for tweet {:?}",
                                tweet_obj.get("id")
                            );
                        }
                    } else {
                        log::warn!(
                            "Thread context: Non-string author or text in reply for tweet {:?}",
                            tweet_obj.get("id")
                        );
                    }
                }
                _ => {
                    log::warn!(
                        "Thread context: Missing author or text field in reply for tweet {:?}",
                        tweet_obj.get("id")
                    );
                }
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

    // Log warnings for missing or incomplete data
    if reply_count > 0 && valid_replies == 0 {
        log::warn!(
            "Thread context: Tweet {:?} has {} replies but none are valid",
            tweet_obj.get("id"),
            reply_count
        );
    }

    if conversation_indicators.is_empty() && !tweet_text.is_empty() {
        log::debug!(
            "Thread context: No conversation indicators detected in tweet {:?}",
            tweet_obj.get("id")
        );
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_sentiment_analyzer() {
        let analyzer = EnhancedSentimentAnalyzer::default();

        let result = analyzer.analyze_enhanced("This is amazing!", None, None, None);

        assert_eq!(result.base_sentiment, Sentiment::Positive);
        assert!(result.confidence > 0.0);
        // Without contextual data, final sentiment should match base
        assert_eq!(result.final_sentiment, result.base_sentiment);
    }

    #[test]
    fn test_extract_thread_context_missing_data() {
        let tweet_data = serde_json::json!({
            "id": "test_tweet",
            "text": "Hello world?"
        });

        let context = extract_thread_context(&tweet_data).unwrap();
        assert_eq!(context.reply_count, 0);
        assert_eq!(context.avg_reply_sentiment, 0.0);
        assert!(context
            .conversation_indicators
            .contains(&ConversationIndicator::Question));
    }

    #[test]
    fn test_extract_tweet_text_validation() {
        // Valid text
        let tweet_data = serde_json::json!({"id": "test", "text": "Hello world"});
        assert_eq!(extract_tweet_text(&tweet_data), "Hello world");

        // Empty text
        let tweet_data = serde_json::json!({"id": "test", "text": ""});
        assert_eq!(extract_tweet_text(&tweet_data), "");

        // Missing text field
        let tweet_data = serde_json::json!({"id": "test"});
        assert_eq!(extract_tweet_text(&tweet_data), "");
    }

    #[test]
    fn test_sentiment_to_score() {
        assert_eq!(sentiment_to_score(Sentiment::Positive), 1.0);
        assert_eq!(sentiment_to_score(Sentiment::Neutral), 0.0);
        assert_eq!(sentiment_to_score(Sentiment::Negative), -1.0);
    }

    #[test]
    fn test_score_to_sentiment() {
        assert_eq!(score_to_sentiment(0.5), Sentiment::Positive);
        assert_eq!(score_to_sentiment(0.1), Sentiment::Neutral);
        assert_eq!(score_to_sentiment(-0.5), Sentiment::Negative);
    }
}
