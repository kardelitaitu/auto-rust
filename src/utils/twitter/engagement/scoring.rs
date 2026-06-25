//! Scoring logic for engagement decisions and sentiment modulation.

use super::super::twitteractivity_state::TaskConfig;
use crate::utils::twitter::sentiment::SentimentAnalyzer;
use crate::utils::twitter::{
    decision::{DecisionEngineFactory, DecisionStrategy, EngagementDecision, TweetContext},
    sentiment::Sentiment,
    twitteractivity_persona::PersonaWeights,
};
use log::info;
use serde_json::Value;

use crate::utils::twitter::twitteractivity_actions::extract_tweet_text;
use crate::utils::twitter::twitteractivity_types::TweetId;

pub async fn handle_engagement_decision(
    tweet: &Value,
    task_config: &TaskConfig,
    persona: &PersonaWeights,
    llm_api_key: Option<String>,
) -> Option<EngagementDecision> {
    if !task_config.smart_decision_enabled {
        return None;
    }

    // Extract tweet text
    let tweet_text = tweet.get("text").and_then(|v| v.as_str()).unwrap_or("");
    let tweet_id = tweet
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let author = tweet
        .get("author")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Extract replies from tweet data
    let mut replies: Vec<String> = Vec::new();
    if let Some(replies_array) = tweet.get("replies").and_then(|v| v.as_array()) {
        for reply_value in replies_array {
            if let Some(reply_obj) = reply_value.as_object() {
                if let Some(text_value) = reply_obj.get("text") {
                    if let Some(text_str) = text_value.as_str() {
                        replies.push(text_str.to_string());
                    }
                }
            }
        }
    }

    info!(
        "[twitter] Smart decision: tweet_id={} author=@{} replies={}",
        tweet_id,
        author,
        replies.len()
    );

    // Create context for decision engine
    let ctx = TweetContext {
        tweet_id: TweetId::from_unchecked(tweet_id),
        text: tweet_text.to_string(),
        author: author.to_string(),
        replies,
        persona: persona.clone(),
        task_config: task_config.clone(),
        tweet_age: "Recent".to_string(), // Default for feed view
    };

    // Use Factory to create appropriate engine
    // For feed scan, we typically use Legacy or Persona strategy unless LLM is explicitly requested
    let strategy = if task_config.llm_enabled {
        DecisionStrategy::Auto
    } else {
        DecisionStrategy::Legacy
    };

    let engine = DecisionEngineFactory::create(strategy, llm_api_key);

    Some(engine.decide(&ctx).await)
}

/// Cached SentimentAnalyzer instance (created once and reused).
pub(crate) static SENTIMENT_ANALYZER: std::sync::OnceLock<tokio::sync::Mutex<SentimentAnalyzer>> =
    std::sync::OnceLock::new();

/// Analyze tweet sentiment and modulate persona weights accordingly.
#[allow(clippy::cast_precision_loss)]
pub(crate) async fn modulate_persona_by_sentiment(
    tweet: &Value,
    task_config: &TaskConfig,
    persona: &PersonaWeights,
) -> (Sentiment, PersonaWeights) {
    let analyzer = SENTIMENT_ANALYZER
        .get_or_init(|| tokio::sync::Mutex::new(SentimentAnalyzer::new()))
        .lock()
        .await;
    let tweet_text = extract_tweet_text(tweet);
    let sentiment_result = if task_config.enhanced_sentiment_enabled {
        let thread_context = crate::utils::twitter::sentiment::extract_thread_context(tweet);
        let user_reputation = crate::utils::twitter::sentiment::extract_user_reputation(tweet);
        let temporal_factors = crate::utils::twitter::sentiment::extract_temporal_factors(tweet);
        analyzer.analyze_enhanced(
            &tweet_text,
            thread_context.as_ref(),
            user_reputation.as_ref(),
            temporal_factors.as_ref(),
        )
    } else {
        // Fallback to basic sentiment analysis
        let sentiment = analyzer.analyze_sentiment_sync(&tweet_text);
        crate::utils::twitter::sentiment::EnhancedSentimentResult {
            base_sentiment: sentiment,
            final_sentiment: sentiment,
            base_score: crate::utils::twitter::sentiment::sentiment_score(sentiment) as f32,
            final_score: crate::utils::twitter::sentiment::sentiment_score(sentiment) as f32,
            confidence: 0.7, // Default confidence for basic analysis
            score_breakdown: crate::utils::twitter::sentiment::ScoreBreakdown {
                text_score: crate::utils::twitter::sentiment::sentiment_score(sentiment) as f32,
                emoji_score: 0.0,
                domain_score: 0.0,
                context_score: 0.0,
                reputation_score: 0.0,
                temporal_score: 0.0,
            },
        }
    };

    let sentiment = sentiment_result.final_sentiment;
    let mut candidate_persona = persona.clone();
    // Modulate weights by sentiment with enhanced scoring
    candidate_persona.interest_multiplier = match sentiment {
        Sentiment::Negative => 0.3, // suppress engagement on negative tweets
        Sentiment::Positive => 1.4, // boost positive (lightly more than basic)
        Sentiment::Neutral => 1.0,
    };

    // Additional modulation based on confidence
    if sentiment_result.confidence > 0.8 {
        // High confidence - amplify the effect
        candidate_persona.interest_multiplier *= 1.1;
    } else if sentiment_result.confidence < 0.5 {
        // Low confidence - reduce the effect
        candidate_persona.interest_multiplier *= 0.9;
    }

    (sentiment, candidate_persona)
}
