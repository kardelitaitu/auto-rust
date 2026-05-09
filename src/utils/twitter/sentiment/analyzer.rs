//! Unified sentiment analyzer using the Strategy Pattern.
//! Provides configurable sentiment analysis with basic and enhanced modes.

use crate::internal::text::truncate_chars;
use crate::llm::client::LlmClient;

use super::SentimentStrategy;
use super::strategies::{domain, emoji, llm as llm_strategy};
use serde_json::Value;
use tracing::instrument;

// ============================================================================
// Strategy Constants and Functions
// ============================================================================

/// Negation patterns that flip sentiment polarity.
const NEGATION_PATTERNS: &[&str] = &[
    "not", "no", "never", "neither", "nobody", "nothing", "nor", "can't", "cant",
    "couldn't", "couldnt", "shouldn't", "shouldnt", "wouldn't", "wouldnt", "don't",
    "dont", "doesn't", "doesnt", "didn't", "didnt", "isn't", "isnt", "aren't",
    "arent", "wasn't", "wasnt", "weren't", "werent", "without", "lack", "lacking",
    "absent", "hardly", "barely", "scarcely", "little", "few", "nowhere", "nothing",
];

/// Intensifiers that amplify sentiment (multiplier > 1.0).
const INTENSIFIERS: &[(&str, f32)] = &[
    ("very", 1.5), ("really", 1.5), ("extremely", 2.0), ("incredibly", 2.0),
    ("absolutely", 2.0), ("totally", 1.8), ("completely", 1.8), ("utterly", 2.0),
    ("highly", 1.5), ("super", 1.5), ("so", 1.3), ("quite", 1.2), ("rather", 1.2),
    ("pretty", 1.2), ("damn", 1.8), ("fucking", 2.0), ("frigging", 1.8),
    ("bloody", 1.8), ("truly", 1.5), ("genuinely", 1.3), ("honestly", 1.3),
    ("actually", 1.2), ("especially", 1.5), ("particularly", 1.4),
    ("exceptionally", 2.0), ("remarkably", 1.8), ("extraordinarily", 2.0),
];

/// Sarcasm markers and patterns that indicate inverted meaning.
const SARCASM_PATTERNS: &[&str] = &[
    "oh great", "oh wonderful", "oh perfect", "oh good", "oh fantastic",
    "sure, because", "yeah right", "as if", "as though", "thanks, i hate it",
    "tanks, i hate it", "thx i hate it", "just what i needed",
    "exactly what i wanted", "because that's what i need",
    "because that's what i wanted", "thanks twitter", "thx twitter",
    "cool cool cool", "sure sure", "okay sure", "what could go wrong",
    "how hard could it be", "famous last words", "we'll see about that",
];

/// Calculate context-aware sentiment score for a word.
fn calculate_contextual_score(text: &str, base_score: f32, target_word: &str) -> f32 {
    let mut score = base_score;
    let multiplier = get_intensifier_multiplier(text, target_word);
    score *= multiplier;
    if is_negated(text, target_word) { score = -score; }
    score
}

fn is_negated(text: &str, target_word: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    let target_lower = target_word.to_lowercase();
    for (i, word) in words.iter().enumerate() {
        if word.to_lowercase() == target_lower {
            let start = i.saturating_sub(3);
            if words.iter().take(i).skip(start).any(|prev| {
                NEGATION_PATTERNS.iter().any(|&n| prev.to_lowercase() == n)
            }) { return true; }
        }
    }
    false
}

fn get_intensifier_multiplier(text: &str, target_word: &str) -> f32 {
    let words: Vec<&str> = text.split_whitespace().collect();
    let target_lower = target_word.to_lowercase();
    for (i, word) in words.iter().enumerate() {
        if word.to_lowercase() == target_lower {
            let start = i.saturating_sub(2);
            if let Some((_, multiplier)) = words.iter().take(i).skip(start).find_map(|prev| {
                INTENSIFIERS.iter().find(|(intensifier, _)| prev.to_lowercase() == *intensifier)
            }) { return *multiplier; }
        }
    }
    1.0
}

fn analyze_contextual_modifiers(text: &str) -> f32 {
    let mut modifier = 0.0;
    if has_sarcasm_markers(text) { modifier -= 2.0; }
    if is_excessive_punctuation(text) { modifier -= 0.5; }
    modifier
}

fn has_sarcasm_markers(text: &str) -> bool {
    let lower = text.to_lowercase();
    SARCASM_PATTERNS.iter().any(|&pattern| lower.contains(pattern))
}

fn is_excessive_punctuation(text: &str) -> bool {
    let exclamation_count = text.matches('!').count();
    let question_count = text.matches('?').count();
    text.contains("?!") || text.contains("!?") || exclamation_count > 2 || question_count > 2
}

// ============================================================================
// Strategy Structs and Impls
// ============================================================================

#[derive(Debug)]
pub struct BasicKeywordStrategy;

#[derive(Debug)]
pub struct ContextStrategy;

#[derive(Debug)]
pub struct EmojiStrategy;

#[derive(Debug)]
pub struct DomainStrategy;

impl SentimentStrategy for BasicKeywordStrategy {
    fn analyze(&self, text: &str) -> f32 {
        let mut score = 0.0;
        let lower = text.to_lowercase();
        for &word in POSITIVE_WORDS {
            if crate::utils::twitter::sentiment::utils::contains_word(&lower, word) {
                score += calculate_contextual_score(&lower, 1.0, word);
            }
        }
        for &word in NEGATIVE_WORDS {
            if crate::utils::twitter::sentiment::utils::contains_word(&lower, word) {
                score += calculate_contextual_score(&lower, -1.0, word);
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
        emoji::analyze_emoji_sentiment(text)
    }
}

impl SentimentStrategy for DomainStrategy {
    fn analyze(&self, text: &str) -> f32 {
        let d = domain::detect_domain(text);
        domain::analyze_domain_sentiment(text, d)
    }
}

// ============================================================================
// Keyword Lists
// ============================================================================

const POSITIVE_WORDS: &[&str] = &[
    "good", "great", "awesome", "amazing", "excellent", "love", "like", "nice",
    "wonderful", "fantastic", "best", "happy", "glad", "joy", "cool", "brilliant",
    "thank", "thanks", "appreciate", "beautiful", "perfect", "ideal", "superb",
    "outstanding", "impressive", "enjoy", "fun", "yes", "win", "won",
    "celebrate", "congrats", "congratulations", "well done", "welldone",
    "spot on", "correct", "right", "smart", "wise", "kind", "friendly",
    "helpful", "support", "bless", "marvelous", "pleasure", "delighted",
    "thrilled", "excited", "yay", "😊", "❤️", "🔥", "💯", "👏",
];

const NEGATIVE_WORDS: &[&str] = &[
    "bad", "terrible", "awful", "worst", "hate", "dislike", "horrible",
    "disgusting", "poor", "sad", "angry", "mad", "upset", "annoyed",
    "disappointed", "fail", "failed", "failure", "wrong", "error", "mistake",
    "bug", "broken", "useless", "waste", "sucks", "sucked", "suck", "hell",
    "shit", "damn", "fuck", "fucking", "idiot", "stupid", "dumb", "ridiculous",
    "absurd", "fake", "scam", "liar", "lies", "lying", "toxic", "abuse",
    "abusive", "harassment", "harassing", "block", "report", "spam", "spammer",
    "clown", "joke", "pathetic", "disaster", "mess", "nightmare", "regret",
    "depressing", "depressed", "anxious", "anxiety", "cry", "crying",
    "😢", "😡", "💩",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sentiment {
    Positive,
    Neutral,
    Negative,
}

#[derive(Debug, Clone)]
pub struct ThreadContext {
    pub reply_count: u32,
    pub avg_reply_sentiment: f32,
    pub is_reply: bool,
    pub is_quote: bool,
    pub thread_depth: u32,
    pub conversation_indicators: Vec<ConversationIndicator>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConversationIndicator {
    Agreement, Disagreement, Question, Clarification, Humor, Sarcasm, Support, Criticism,
}

#[derive(Debug, Clone)]
pub struct UserReputation {
    pub follower_count: u32,
    pub is_verified: bool,
    pub account_age_days: u32,
    pub engagement_rate: f32,
    pub is_influential: bool,
    pub trust_score: f32,
}

#[derive(Debug, Clone)]
pub struct TemporalFactors {
    pub hour_of_day: u8,
    pub day_of_week: u8,
    pub hours_since_post: f32,
    pub is_peak_hour: bool,
    pub trending_bias: f32,
}

#[derive(Debug, Clone)]
pub struct EnhancedSentimentResult {
    pub base_sentiment: Sentiment,
    pub final_sentiment: Sentiment,
    pub base_score: f32,
    pub final_score: f32,
    pub confidence: f32,
    pub score_breakdown: ScoreBreakdown,
}

#[derive(Debug, Clone, Default)]
pub struct ScoreBreakdown {
    pub text_score: f32,
    pub emoji_score: f32,
    pub domain_score: f32,
    pub context_score: f32,
    pub reputation_score: f32,
    pub temporal_score: f32,
}

#[derive(Debug, Clone)]
pub struct SentimentConfig {
    pub use_basic_keywords: bool,
    pub use_context: bool,
    pub use_emoji: bool,
    pub use_domain: bool,
    pub use_llm: bool,
    pub llm_min_confidence: f32,
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

impl Default for SentimentAnalyzer {
    fn default() -> Self { Self::new() }
}

impl SentimentAnalyzer {
    pub fn new() -> Self { Self::with_config(SentimentConfig::default()) }

    pub fn with_config(config: SentimentConfig) -> Self {
        let mut strategies: Vec<Box<dyn SentimentStrategy>> = Vec::new();
        if config.use_basic_keywords { strategies.push(Box::new(BasicKeywordStrategy)); }
        if config.use_context { strategies.push(Box::new(ContextStrategy)); }
        if config.use_emoji { strategies.push(Box::new(EmojiStrategy)); }
        if config.use_domain { strategies.push(Box::new(DomainStrategy)); }
        Self { config, llm_client: None, strategies }
    }

    pub fn with_llm_client(mut self, llm_client: LlmClient) -> Self {
        self.llm_client = Some(llm_client);
        self
    }

    #[instrument]
    pub async fn analyze_sentiment(&self, text: &str) -> Sentiment {
        let mut total_score = 0.0;
        for strategy in &self.strategies { total_score += strategy.analyze(text); }
        if self.config.use_llm {
            if let Some(llm) = &self.llm_client {
                let llm_sentiment = llm_strategy::analyze_sentiment_hybrid(
                    Some(llm), text, self.config.llm_probability, self.config.llm_min_confidence,
                ).await;
                total_score += sentiment_to_score(llm_sentiment) * 0.5;
            }
        }
        score_to_sentiment(total_score)
    }

    pub fn analyze_sentiment_sync(&self, text: &str) -> Sentiment {
        let mut total_score = 0.0;
        for strategy in &self.strategies { total_score += strategy.analyze(text); }
        score_to_sentiment(total_score)
    }

    pub fn analyze_enhanced(
        &self,
        tweet_text: &str,
        thread_context: Option<&ThreadContext>,
        user_reputation: Option<&UserReputation>,
        temporal_factors: Option<&TemporalFactors>,
    ) -> EnhancedSentimentResult {
        let base_sentiment = self.analyze_sentiment_sync(tweet_text);
        let base_score = sentiment_to_score(base_sentiment);
        let mut breakdown = ScoreBreakdown { text_score: base_score, ..Default::default() };
        let mut final_score = base_score;

        if let Some(context) = thread_context {
            let m = self.analyze_thread_context(context);
            breakdown.context_score = m;
            final_score += m;
        }
        if let Some(reputation) = user_reputation {
            let m = self.analyze_user_reputation(reputation);
            breakdown.reputation_score = m;
            final_score += m;
        }
        if let Some(temporal) = temporal_factors {
            let m = self.analyze_temporal_factors(temporal);
            breakdown.temporal_score = m;
            final_score += m;
        }

        let final_sentiment = score_to_sentiment(final_score);
        let confidence = self.calculate_confidence(&breakdown, base_score, final_score);

        EnhancedSentimentResult {
            base_sentiment, final_sentiment, base_score, final_score, confidence, score_breakdown: breakdown,
        }
    }

    fn analyze_thread_context(&self, context: &ThreadContext) -> f32 {
        let mut modifier = 0.0;
        if context.reply_count > 0 {
            let weight = if context.reply_count <= 5 { 0.3 } else if context.reply_count <= 20 { 0.2 } else { 0.1 };
            modifier += context.avg_reply_sentiment * weight;
            if context.reply_count > 10 { modifier += 0.1; }
        }
        modifier += match context.thread_depth {
            0 => 0.0, 1..=2 => 0.05, 3..=5 => 0.1, 6..=10 => 0.15, _ => 0.2,
        };
        if context.is_reply { modifier += 0.08; }
        if context.is_quote { modifier += 0.12; }
        
        let indicator_count = context.conversation_indicators.len() as f32;
        let indicator_weight = if indicator_count > 0.0 { (5.0 / indicator_count).min(1.0) } else { 1.0 };
        for indicator in &context.conversation_indicators {
            let base = match indicator {
                ConversationIndicator::Agreement => 0.08,
                ConversationIndicator::Disagreement => -0.08,
                ConversationIndicator::Question => 0.04,
                ConversationIndicator::Clarification => 0.06,
                ConversationIndicator::Humor => 0.1,
                ConversationIndicator::Sarcasm => -0.15,
                ConversationIndicator::Support => 0.12,
                ConversationIndicator::Criticism => -0.12,
            };
            modifier += base * indicator_weight;
        }
        modifier
    }

    fn analyze_user_reputation(&self, reputation: &UserReputation) -> f32 {
        let mut modifier = 0.0;
        if reputation.is_verified { modifier += 0.12; }
        modifier += if reputation.follower_count == 0 { -0.1 } else {
            let log_f = (reputation.follower_count as f32).ln().max(0.0);
            ((log_f * 0.03).min(0.2)) + (if reputation.follower_count < 50 { -0.05 } else { 0.0 })
        };
        modifier += if reputation.account_age_days < 7 { -0.15 } else if reputation.account_age_days < 30 { -0.08 }
            else if reputation.account_age_days < 90 { -0.03 } else if reputation.account_age_days < 365 { 0.02 }
            else if reputation.account_age_days < 1095 { 0.05 } else { 0.08 };
        modifier += if reputation.engagement_rate > 0.2 { 0.08 } else if reputation.engagement_rate > 0.1 { 0.04 }
            else if reputation.engagement_rate > 0.05 { 0.0 } else if reputation.engagement_rate > 0.01 { -0.02 } else { -0.08 };
        if reputation.is_influential { modifier += 0.15; }
        modifier += if reputation.trust_score > 0.8 { 0.15 } else if reputation.trust_score > 0.6 { 0.08 }
            else if reputation.trust_score > 0.4 { 0.0 } else if reputation.trust_score > 0.2 { -0.05 } else { -0.1 };
        modifier
    }

    fn analyze_temporal_factors(&self, temporal: &TemporalFactors) -> f32 {
        let mut modifier = 0.0;
        modifier += match temporal.hour_of_day {
            6..=9 => 0.08, 10..=12 => 0.05, 13..=15 => 0.02, 16..=18 => -0.01, 19..=21 => 0.03, 22..=23 => -0.03, 0..=3 => -0.08, 4..=5 => -0.05, _ => 0.0,
        };
        modifier += match temporal.day_of_week {
            0 => 0.02, 1 => 0.01, 2 => 0.0, 3 => 0.01, 4 => 0.04, 5 => 0.06, 6 => 0.05, _ => 0.0,
        };
        modifier += if temporal.hours_since_post < 0.5 { 0.12 } else if temporal.hours_since_post < 2.0 { 0.08 }
            else if temporal.hours_since_post < 6.0 { 0.04 } else if temporal.hours_since_post < 24.0 { 0.02 }
            else if temporal.hours_since_post < 72.0 { 0.0 } else { -0.02 };
        if temporal.is_peak_hour { modifier += 0.03; }
        modifier += temporal.trending_bias * 0.08;
        modifier
    }

    fn calculate_confidence(&self, breakdown: &ScoreBreakdown, base_score: f32, final_score: f32) -> f32 {
        let mut confidence = 0.5;
        confidence += (final_score.abs() * 0.2).min(0.2);
        confidence += self.calculate_factor_agreement(breakdown) * 0.2;
        confidence -= ((final_score - base_score).abs() * 0.1).min(0.1);
        confidence.clamp(0.0, 1.0)
    }

    fn calculate_factor_agreement(&self, breakdown: &ScoreBreakdown) -> f32 {
        let factors = vec![breakdown.text_score, breakdown.emoji_score, breakdown.domain_score, breakdown.context_score, breakdown.reputation_score, breakdown.temporal_score];
        let non_zero: Vec<f32> = factors.into_iter().filter(|&x| x != 0.0).collect();
        if non_zero.len() < 2 { return 0.0; }
        let pos = non_zero.iter().filter(|&&x| x > 0.0).count();
        let neg = non_zero.iter().filter(|&&x| x < 0.0).count();
        if pos > neg { pos as f32 / non_zero.len() as f32 } else { neg as f32 / non_zero.len() as f32 }
    }
}

pub fn sentiment_score(sentiment: Sentiment) -> i32 {
    match sentiment { Sentiment::Positive => 1, Sentiment::Neutral => 0, Sentiment::Negative => -1 }
}

pub async fn analyze_tweet_sentiment(analyzer: &SentimentAnalyzer, tweet_obj: &Value) -> Sentiment {
    let text = extract_tweet_text(tweet_obj);
    analyzer.analyze_sentiment(&text).await
}

pub fn analyze_tweet_sentiment_sync(analyzer: &SentimentAnalyzer, tweet_obj: &Value) -> Sentiment {
    let text = extract_tweet_text(tweet_obj);
    analyzer.analyze_sentiment_sync(&text)
}

fn extract_tweet_text(tweet_obj: &Value) -> String {
    if let Some(text) = tweet_obj.get("text").and_then(|v| v.as_str()) { return text.to_string(); }
    if let Some(full) = tweet_obj.get("full_text").and_then(|v| v.as_str()) { return full.to_string(); }
    if let Some(obj) = tweet_obj.as_object() {
        if let Some(rt) = obj.get("retweeted_status") { return extract_tweet_text(rt); }
    }
    truncate_chars(&tweet_obj.to_string(), 280)
}

#[derive(Debug, Clone, Default)]
pub struct SentimentStats { pub positive: u32, pub neutral: u32, pub negative: u32 }

impl SentimentStats {
    pub fn new() -> Self { Self::default() }
    pub fn add(&mut self, s: Sentiment) {
        match s { Sentiment::Positive => self.positive += 1, Sentiment::Neutral => self.neutral += 1, Sentiment::Negative => self.negative += 1 }
    }
    pub fn dominant(&self) -> Sentiment {
        if self.positive > self.neutral && self.positive > self.negative { Sentiment::Positive }
        else if self.negative > self.neutral && self.negative > self.positive { Sentiment::Negative }
        else { Sentiment::Neutral }
    }
    pub fn total(&self) -> u32 { self.positive + self.neutral + self.negative }
}

pub fn feed_sentiment_score(stats: &SentimentStats) -> f64 {
    let total = stats.total() as f64;
    if total == 0.0 { return 0.0; }
    (stats.positive as f64 / total) - (stats.negative as f64 / total)
}

fn sentiment_to_score(s: Sentiment) -> f32 {
    match s { Sentiment::Positive => 1.0, Sentiment::Neutral => 0.0, Sentiment::Negative => -1.0 }
}

fn score_to_sentiment(score: f32) -> Sentiment {
    if score > 0.3 { Sentiment::Positive } else if score < -0.3 { Sentiment::Negative } else { Sentiment::Neutral }
}

pub fn analyze_sentiment_sync(text: &str) -> Sentiment {
    SentimentAnalyzer::new().analyze_sentiment_sync(text)
}

pub fn extract_thread_context(tweet_obj: &Value) -> Option<ThreadContext> {
    let reply_count = tweet_obj.get("replies").and_then(|v| v.as_array()).map(|a| a.len() as u32).unwrap_or(0);
    let mut reply_scores = Vec::new();
    if let Some(replies) = tweet_obj.get("replies").and_then(|v| v.as_array()) {
        for reply in replies {
            if let Some(text) = reply.get("text").and_then(|v| v.as_str()) {
                reply_scores.push(sentiment_to_score(analyze_sentiment_sync(text)));
            }
        }
    }
    let avg_reply_sentiment = if reply_scores.is_empty() { 0.0 } else { reply_scores.iter().sum::<f32>() / reply_scores.len() as f32 };
    let tweet_text = extract_tweet_text(tweet_obj);
    Some(ThreadContext {
        reply_count, avg_reply_sentiment, is_reply: false, is_quote: false, thread_depth: 0,
        conversation_indicators: detect_conversation_indicators(&tweet_text),
    })
}

pub fn extract_user_reputation(_tweet_obj: &Value) -> Option<UserReputation> {
    Some(UserReputation { follower_count: 1000, is_verified: false, account_age_days: 365, engagement_rate: 0.05, is_influential: false, trust_score: 0.5 })
}

pub fn extract_temporal_factors(_tweet_obj: &Value) -> Option<TemporalFactors> {
    Some(TemporalFactors { hour_of_day: 12, day_of_week: 1, hours_since_post: 24.0, is_peak_hour: true, trending_bias: 0.0 })
}

pub fn detect_conversation_indicators(text: &str) -> Vec<ConversationIndicator> {
    let lower = text.to_lowercase();
    let mut indicators = Vec::new();
    if AGREEMENT_PATTERNS.iter().any(|&p| lower.contains(p)) { indicators.push(ConversationIndicator::Agreement); }
    if DISAGREEMENT_PATTERNS.iter().any(|&p| lower.contains(p)) { indicators.push(ConversationIndicator::Disagreement); }
    if QUESTION_PATTERNS.iter().any(|&p| lower.contains(p)) || text.contains('?') { indicators.push(ConversationIndicator::Question); }
    if CLARIFICATION_PATTERNS.iter().any(|&p| lower.contains(p)) { indicators.push(ConversationIndicator::Clarification); }
    if HUMOR_PATTERNS.iter().any(|&p| lower.contains(p)) { indicators.push(ConversationIndicator::Humor); }
    if SUPPORT_PATTERNS.iter().any(|&p| lower.contains(p)) { indicators.push(ConversationIndicator::Support); }
    if CRITICISM_PATTERNS.iter().any(|&p| lower.contains(p)) { indicators.push(ConversationIndicator::Criticism); }
    if SARCASM_INDICATORS.iter().any(|&p| lower.contains(p)) { indicators.push(ConversationIndicator::Sarcasm); }
    indicators
}

const AGREEMENT_PATTERNS: &[&str] = &["i agree", "totally agree", "absolutely", "exactly", "you're right", "well said"];
const DISAGREEMENT_PATTERNS: &[&str] = &["i disagree", "totally disagree", "you're wrong", "not sure", "doubt it"];
const QUESTION_PATTERNS: &[&str] = &["what if", "how come", "why is", "what do you", "can you explain"];
const CLARIFICATION_PATTERNS: &[&str] = &["to clarify", "let me explain", "what i mean", "in other words"];
const HUMOR_PATTERNS: &[&str] = &["lol", "haha", "😂", "🤣", "joke", "funny"];
const SUPPORT_PATTERNS: &[&str] = &["i support", "good luck", "keep going", "you're doing great"];
const CRITICISM_PATTERNS: &[&str] = &["that's bad", "you shouldn't", "that's wrong", "disappointing"];
const SARCASM_INDICATORS: &[&str] = &["oh sure", "yeah right", "as if", "oh please", "oh come on"];

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_sentiment_analyzer_basic() {
        let analyzer = SentimentAnalyzer::new();
        assert_eq!(analyzer.analyze_sentiment("This is amazing!").await, Sentiment::Positive);
    }
    #[tokio::test]
    async fn test_sentiment_analyzer_negative() {
        let analyzer = SentimentAnalyzer::new();
        assert_eq!(analyzer.analyze_sentiment("This is terrible!").await, Sentiment::Negative);
    }
}
