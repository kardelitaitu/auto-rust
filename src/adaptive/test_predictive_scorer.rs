//! Tests for predictive scoring engine.
//! Validates feature extraction, entropy calculation, and scoring algorithms.

use crate::adaptive::predictive_scorer::{PredictiveScorer, ScoringFeatures, EngagementPrediction};

#[test]
fn test_predictive_scorer_creation() {
    let scorer = PredictiveScorer::new();
    assert!(scorer.feature_weights.is_empty());
}

#[test]
fn test_extract_features() {
    let scorer = PredictiveScorer::new();
    let features = scorer.extract_features(
        "test_tweet",
        10,
        5,
        0.8,
        &vec!["positive".to_string()],
    );
    
    assert!(features.text_length > 0);
    assert!(features.reply_count >= 0);
}

#[test]
fn test_calculate_entropy() {
    let scorer = PredictiveScorer::new();
    
    // Test with uniform distribution (high entropy)
    let uniform = vec![0.25, 0.25, 0.25, 0.25];
    let entropy_uniform = scorer.calculate_entropy(&uniform);
    assert!(entropy_uniform > 0.0);
    
    // Test with skewed distribution (low entropy)
    let skewed = vec![0.9, 0.05, 0.03, 0.02];
    let entropy_skewed = scorer.calculate_entropy(&skewed);
    assert!(entropy_skewed < entropy_uniform);
}

#[test]
fn test_calculate_score() {
    let scorer = PredictiveScorer::new();
    let features = ScoringFeatures {
        text_length: 100,
        reply_count: 5,
        sentiment_score: 0.8,
        engagement_rate: 0.7,
        temporal_factor: 0.5,
        user_influence: 0.6,
    };
    
    let score = scorer.calculate_score(&features);
    assert!(score >= 0.0 && score <= 1.0);
}

#[test]
fn test_predict_engagement() {
    let scorer = PredictiveScorer::new();
    let features = ScoringFeatures {
        text_length: 100,
        reply_count: 5,
        sentiment_score: 0.8,
        engagement_rate: 0.7,
        temporal_factor: 0.5,
        user_influence: 0.6,
    };
    
    let prediction = scorer.predict_engagement(&features);
    assert!(prediction.score >= 0.0 && prediction.score <= 1.0);
    assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
}

#[test]
fn test_update_model() {
    let mut scorer = PredictiveScorer::new();
    scorer.update_model(0.8, true);
    assert!(scorer.feature_weights.len() > 0);
}
