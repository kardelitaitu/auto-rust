//! Predictive engagement scorer using ML-based predictions.
//! Provides engagement success probability and optimal action recommendations.

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct UserBehaviorProfile {
    pub successful_actions: HashMap<String, u32>,
}

/// Predictive engagement scorer that uses ML models for engagement prediction.
pub struct PredictiveEngagementScorer {
    /// Action recommendation engine
    action_recommender: ActionRecommender,
}

/// Text-based feature extraction.
#[derive(Debug, Clone, Default)]
struct TextFeatures {
    /// Sentiment score (reserved for future sentiment-weighted decisions)
    #[cfg_attr(not(test), allow(dead_code))]
    sentiment: f32,
    /// Text length (used by ActionRecommender::get_best_action)
    length: usize,
    /// Keyword presence (reserved for future keyword matching)
    #[cfg_attr(not(test), allow(dead_code))]
    keywords: HashMap<String, f32>,
    /// Readability score (reserved for future content quality scoring)
    #[cfg_attr(not(test), allow(dead_code))]
    readability: f32,
    /// Emotion score (reserved for future affect-aware engagement)
    #[cfg_attr(not(test), allow(dead_code))]
    emotion: f32,
}

/// Temporal feature extraction.
#[derive(Debug, Clone)]
struct TemporalFeatures {
    /// Hour of day (0-23) — used by get_optimal_timing
    hour: u8,
    /// Day of week (0-6) — reserved for future day-aware decisions
    #[cfg_attr(not(test), allow(dead_code))]
    day_of_week: u8,
    /// Is peak hour — used by get_best_action
    is_peak: bool,
    /// Time since last post (reserved for future rate limiting)
    #[cfg_attr(not(test), allow(dead_code))]
    time_since_last: f32,
    /// Posting frequency (reserved for future cadence analysis)
    #[cfg_attr(not(test), allow(dead_code))]
    posting_frequency: f32,
}

/// User-based feature extraction.
#[derive(Debug, Clone)]
struct UserFeatures {
    /// User reputation score (reserved for future trust weighting)
    #[cfg_attr(not(test), allow(dead_code))]
    reputation: f32,
    /// Follower count (reserved for future influence scoring)
    #[cfg_attr(not(test), allow(dead_code))]
    follower_count: u32,
    /// Following count (reserved for future follow-back analysis)
    #[cfg_attr(not(test), allow(dead_code))]
    following_count: u32,
    /// Account age in days (reserved for future account maturity scoring)
    #[cfg_attr(not(test), allow(dead_code))]
    account_age: u32,
    /// Engagement rate — used by get_best_action
    engagement_rate: f32,
}

/// Contextual feature extraction.
#[derive(Debug, Clone, Default)]
struct ContextFeatures {
    /// Thread depth (reserved for future thread-awareness)
    #[cfg_attr(not(test), allow(dead_code))]
    thread_depth: u32,
    /// Reply count — used by get_best_action
    reply_count: u32,
    /// Has media (reserved for future media-aware decisions)
    #[cfg_attr(not(test), allow(dead_code))]
    has_media: bool,
    /// Topic category (reserved for future topic analysis)
    #[cfg_attr(not(test), allow(dead_code))]
    topic_category: String,
    /// Trending score (reserved for future trending detection)
    #[cfg_attr(not(test), allow(dead_code))]
    trending_score: f32,
}

/// Combined feature vector for prediction.
struct FeatureVector {
    text: TextFeatures,
    user: UserFeatures,
    temporal: TemporalFeatures,
    context: ContextFeatures,
}

/// Action recommendation engine.
struct ActionRecommender {
    /// Action type rankings (reserved for future ML integration)
    #[cfg_attr(not(test), allow(dead_code))]
    action_rankings: HashMap<String, f32>,
    /// Timing recommendations
    timing_recommendations: TimingRecommendations,
    /// Content suggestions (reserved for future content generation)
    #[cfg_attr(not(test), allow(dead_code))]
    content_suggestions: Vec<String>,
}

/// Timing recommendations for engagement.
struct TimingRecommendations {
    /// Optimal posting times
    optimal_times: Vec<u8>,
    /// Recommended posting frequency (reserved for future frequency modulation)
    #[cfg_attr(not(test), allow(dead_code))]
    recommended_frequency: f32,
    /// Best days for engagement (reserved for future day-aware scheduling)
    #[cfg_attr(not(test), allow(dead_code))]
    best_days: Vec<u8>,
}

/// Engagement prediction result.
#[derive(Debug, Clone)]
pub struct EngagementPrediction {
    /// Probability of success (0.0 to 1.0)
    pub success_probability: f32,
    /// Expected engagement score
    pub expected_engagement: f32,
    /// Recommended action type
    pub recommended_action: String,
    /// Optimal posting time (hour)
    pub optimal_time: u8,
    /// Confidence level (0.0 to 1.0)
    pub confidence: f32,
    /// Key factors influencing prediction
    pub key_factors: Vec<String>,
}

// ============================================================================
// Standalone extraction functions (formerly FeatureExtractor::static methods)
// ============================================================================

fn extract_text_features(text: &str) -> TextFeatures {
    TextFeatures {
        sentiment: 0.5,
        length: text.len(),
        keywords: HashMap::new(),
        readability: 0.7,
        emotion: 0.6,
    }
}

fn extract_user_features(profile: &UserBehaviorProfile) -> UserFeatures {
    UserFeatures {
        reputation: 0.7,
        follower_count: profile.successful_actions.get("like").copied().unwrap_or(0),
        following_count: 100,
        account_age: 365,
        engagement_rate: 0.1,
    }
}

fn extract_temporal_features(temporal: &TemporalFeatures) -> TemporalFeatures {
    temporal.clone()
}

fn extract_context_features(context: &ContextFeatures) -> ContextFeatures {
    context.clone()
}

fn combine_features(
    text: TextFeatures,
    user: UserFeatures,
    temporal: TemporalFeatures,
    context: ContextFeatures,
) -> FeatureVector {
    FeatureVector {
        text,
        user,
        temporal,
        context,
    }
}

// ============================================================================
// Prediction helper
// ============================================================================

/// Simplified prediction logic.
/// In production, this would use actual ML model inference.
fn predict_model(_features: &FeatureVector) -> (f32, f32, Vec<String>) {
    let base_score = 0.5;
    let confidence = 0.8;
    let key_factors = vec!["sentiment".to_string(), "timing".to_string()];
    (base_score, confidence, key_factors)
}

// ============================================================================
// PredictiveEngagementScorer
// ============================================================================

impl PredictiveEngagementScorer {
    /// Create a new predictive engagement scorer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            action_recommender: ActionRecommender::new(),
        }
    }

    /// Predict engagement success for a tweet.
    fn predict_engagement(
        &self,
        tweet_text: &str,
        user_profile: &UserBehaviorProfile,
        temporal_context: &TemporalFeatures,
        context_features: &ContextFeatures,
    ) -> EngagementPrediction {
        // Extract features
        let text_features = extract_text_features(tweet_text);
        let all_features = combine_features(
            text_features,
            extract_user_features(user_profile),
            extract_temporal_features(temporal_context),
            extract_context_features(context_features),
        );

        // Make prediction
        let (probability, confidence, key_factors) = predict_model(&all_features);

        // Get action recommendation
        let recommended_action = ActionRecommender::get_best_action(&all_features);

        // Get optimal timing
        let optimal_time = self.action_recommender.get_optimal_timing(temporal_context);

        // Calculate expected engagement
        let expected_engagement = probability * confidence;

        EngagementPrediction {
            success_probability: probability,
            expected_engagement,
            recommended_action,
            optimal_time,
            confidence,
            key_factors,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn benchmark_predict_engagement(&self, tweet_text: &str) -> EngagementPrediction {
        let user_profile = UserBehaviorProfile::default();
        let temporal_context = TemporalFeatures::default();
        let context_features = ContextFeatures::default();

        self.predict_engagement(
            tweet_text,
            &user_profile,
            &temporal_context,
            &context_features,
        )
    }
}

impl Default for PredictiveEngagementScorer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Default impls for feature types
// ============================================================================

impl Default for TemporalFeatures {
    fn default() -> Self {
        Self {
            hour: 12,
            day_of_week: 1,
            is_peak: false,
            time_since_last: 3600.0,
            posting_frequency: 0.1,
        }
    }
}

impl Default for UserFeatures {
    fn default() -> Self {
        Self {
            reputation: 0.5,
            follower_count: 1000,
            following_count: 100,
            account_age: 365,
            engagement_rate: 0.05,
        }
    }
}

// ============================================================================
// ActionRecommender
// ============================================================================

impl ActionRecommender {
    fn new() -> Self {
        Self {
            action_rankings: HashMap::new(),
            timing_recommendations: TimingRecommendations::default(),
            content_suggestions: vec![],
        }
    }

    fn get_best_action(features: &FeatureVector) -> String {
        if features.text.length > 140 {
            "Reply".to_string()
        } else if features.context.reply_count > 5 {
            "Retweet".to_string()
        } else if features.user.engagement_rate > 0.15 {
            "Like".to_string()
        } else if features.temporal.is_peak {
            "Follow".to_string()
        } else {
            "Skip".to_string()
        }
    }

    fn get_optimal_timing(&self, temporal_context: &TemporalFeatures) -> u8 {
        if temporal_context.is_peak {
            temporal_context.hour
        } else {
            self.timing_recommendations
                .optimal_times
                .first()
                .copied()
                .unwrap_or(12)
        }
    }
}

impl Default for ActionRecommender {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TimingRecommendations {
    fn default() -> Self {
        Self {
            optimal_times: vec![9, 12, 18],
            recommended_frequency: 0.5,
            best_days: vec![1, 3, 5],
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prediction_basic() {
        let scorer = PredictiveEngagementScorer::new();
        let user_profile = UserBehaviorProfile::default();
        let temporal = TemporalFeatures::default();
        let context = ContextFeatures::default();

        let prediction =
            scorer.predict_engagement("test tweet", &user_profile, &temporal, &context);

        assert!(prediction.success_probability >= 0.0);
        assert!(prediction.success_probability <= 1.0);
        assert!(!prediction.recommended_action.is_empty());
    }

    #[test]
    fn test_feature_extraction() {
        let features = extract_text_features("Hello world!");

        assert_eq!(features.length, 12);
        assert!(features.sentiment >= 0.0);
    }

    #[test]
    fn test_scorer_new() {
        let scorer = PredictiveEngagementScorer::new();
        // Verify scorer is created without panicking
        let _ = scorer.predict_engagement(
            "test",
            &UserBehaviorProfile::default(),
            &TemporalFeatures::default(),
            &ContextFeatures::default(),
        );
    }

    #[test]
    fn test_prediction_confidence_bounds() {
        let scorer = PredictiveEngagementScorer::new();
        let prediction = scorer.predict_engagement(
            "test",
            &UserBehaviorProfile::default(),
            &TemporalFeatures::default(),
            &ContextFeatures::default(),
        );

        assert!(prediction.confidence >= 0.0);
        assert!(prediction.confidence <= 1.0);
    }

    #[test]
    fn test_prediction_expected_engagement() {
        let scorer = PredictiveEngagementScorer::new();
        let prediction = scorer.predict_engagement(
            "test",
            &UserBehaviorProfile::default(),
            &TemporalFeatures::default(),
            &ContextFeatures::default(),
        );

        assert!(prediction.expected_engagement >= 0.0);
        assert!(prediction.expected_engagement <= 1.0);
    }

    #[test]
    fn test_prediction_optimal_time() {
        let scorer = PredictiveEngagementScorer::new();
        let prediction = scorer.predict_engagement(
            "test",
            &UserBehaviorProfile::default(),
            &TemporalFeatures::default(),
            &ContextFeatures::default(),
        );

        assert!(prediction.optimal_time < 24);
    }

    #[test]
    fn test_prediction_key_factors() {
        let scorer = PredictiveEngagementScorer::new();
        let prediction = scorer.predict_engagement(
            "test",
            &UserBehaviorProfile::default(),
            &TemporalFeatures::default(),
            &ContextFeatures::default(),
        );

        assert!(!prediction.key_factors.is_empty());
    }

    #[test]
    fn test_text_features_default() {
        let features = TextFeatures::default();
        assert_eq!(features.length, 0);
        assert_eq!(features.sentiment, 0.0);
        assert!(features.keywords.is_empty());
        assert_eq!(features.readability, 0.0);
        assert_eq!(features.emotion, 0.0);
    }

    #[test]
    fn test_temporal_features_default() {
        let features = TemporalFeatures::default();
        assert_eq!(features.hour, 12);
        assert_eq!(features.day_of_week, 1);
        assert!(!features.is_peak);
        assert_eq!(features.time_since_last, 3600.0);
        assert_eq!(features.posting_frequency, 0.1);
    }

    #[test]
    fn test_user_features_default() {
        let features = UserFeatures::default();
        assert_eq!(features.follower_count, 1000);
        assert_eq!(features.following_count, 100);
        assert_eq!(features.reputation, 0.5);
        assert_eq!(features.account_age, 365);
        assert_eq!(features.engagement_rate, 0.05);
    }

    #[test]
    fn test_context_features_default() {
        let features = ContextFeatures::default();
        assert_eq!(features.thread_depth, 0);
        assert_eq!(features.reply_count, 0);
        assert!(!features.has_media);
        assert!(features.topic_category.is_empty());
        assert_eq!(features.trending_score, 0.0);
    }

    #[test]
    fn test_action_recommender_new() {
        let recommender = ActionRecommender::new();
        assert!(recommender.action_rankings.is_empty());
        assert!(recommender.content_suggestions.is_empty());
    }

    #[test]
    fn test_timing_recommendations_default() {
        let timing = TimingRecommendations::default();
        assert_eq!(timing.optimal_times.len(), 3);
        assert_eq!(timing.optimal_times[0], 9);
        assert_eq!(timing.best_days.len(), 3);
        assert_eq!(timing.recommended_frequency, 0.5);
    }

    #[test]
    fn test_text_features_length_calculation() {
        let features = extract_text_features("Hello");
        assert_eq!(features.length, 5);
    }

    #[test]
    fn test_user_features_extraction() {
        let profile = UserBehaviorProfile::default();
        let features = extract_user_features(&profile);
        assert_eq!(features.account_age, 365);
    }

    #[test]
    fn test_temporal_features_extraction() {
        let temporal = TemporalFeatures {
            hour: 15,
            ..Default::default()
        };
        let features = extract_temporal_features(&temporal);
        assert_eq!(features.hour, 15);
    }

    #[test]
    fn test_context_features_extraction() {
        let context = ContextFeatures {
            reply_count: 10,
            ..Default::default()
        };
        let features = extract_context_features(&context);
        assert_eq!(features.reply_count, 10);
    }

    #[test]
    fn test_feature_combination() {
        let vector = combine_features(
            TextFeatures::default(),
            UserFeatures::default(),
            TemporalFeatures::default(),
            ContextFeatures::default(),
        );
        assert_eq!(vector.user.follower_count, 1000);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashMap;

    fn build_user_profile(like_count: u32) -> UserBehaviorProfile {
        let mut successful_actions = HashMap::new();
        successful_actions.insert("like".to_string(), like_count);

        UserBehaviorProfile { successful_actions }
    }

    proptest! {
        #[test]
        fn predict_engagement_stays_finite_and_bounded(
            tweet_text in any::<String>(),
            like_count in any::<u32>(),
            hour in 0u8..24,
            day_of_week in 0u8..7,
            is_peak in any::<bool>(),
            time_since_last in any::<f32>(),
            posting_frequency in any::<f32>(),
            thread_depth in any::<u32>(),
            reply_count in any::<u32>(),
            has_media in any::<bool>(),
            topic_category in any::<String>(),
            trending_score in any::<f32>(),
        ) {
            let scorer = PredictiveEngagementScorer::new();
            let user_profile = build_user_profile(like_count);
            let temporal = TemporalFeatures {
                hour,
                day_of_week,
                is_peak,
                time_since_last,
                posting_frequency,
            };
            let context = ContextFeatures {
                thread_depth,
                reply_count,
                has_media,
                topic_category,
                trending_score,
            };

            let prediction = scorer.predict_engagement(&tweet_text, &user_profile, &temporal, &context);

            prop_assert!(prediction.success_probability.is_finite());
            prop_assert!((0.0..=1.0).contains(&prediction.success_probability));
            prop_assert!(prediction.confidence.is_finite());
            prop_assert!((0.0..=1.0).contains(&prediction.confidence));
            prop_assert!(prediction.expected_engagement.is_finite());
            prop_assert!((0.0..=1.0).contains(&prediction.expected_engagement));
        }

        #[test]
        fn feature_extraction_and_combination_handle_generated_inputs(
            tweet_text in any::<String>(),
            like_count in any::<u32>(),
            hour in 0u8..24,
            day_of_week in 0u8..7,
            is_peak in any::<bool>(),
            time_since_last in any::<f32>(),
            posting_frequency in any::<f32>(),
            thread_depth in any::<u32>(),
            reply_count in any::<u32>(),
            has_media in any::<bool>(),
            topic_category in any::<String>(),
            trending_score in any::<f32>(),
        ) {
            let _scorer = PredictiveEngagementScorer::new();
            let user_profile = build_user_profile(like_count);
            let temporal = TemporalFeatures {
                hour,
                day_of_week,
                is_peak,
                time_since_last,
                posting_frequency,
            };
            let context = ContextFeatures {
                thread_depth,
                reply_count,
                has_media,
                topic_category,
                trending_score,
            };

            let text_features = extract_text_features(&tweet_text);
            let user_features = extract_user_features(&user_profile);
            let temporal_features = extract_temporal_features(&temporal);
            let context_features = extract_context_features(&context);
            let combined = combine_features(
                text_features,
                user_features,
                temporal_features,
                context_features,
            );

            prop_assert_eq!(combined.text.length, tweet_text.len());
            prop_assert_eq!(combined.context.thread_depth, context.thread_depth);
            prop_assert_eq!(combined.context.reply_count, context.reply_count);
        }
    }

    #[test]
    fn test_empty_string_regression() {
        let scorer = PredictiveEngagementScorer::new();
        let user_profile = UserBehaviorProfile::default();
        let temporal = TemporalFeatures::default();
        let context = ContextFeatures::default();

        let prediction = scorer.predict_engagement("", &user_profile, &temporal, &context);
        let features = extract_text_features("");

        assert_eq!(features.length, 0);
        assert!(prediction.success_probability.is_finite());
        assert!(prediction.expected_engagement.is_finite());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_predictive_scoring_integration() {
        let scorer = PredictiveEngagementScorer::new();
        let user_profile = UserBehaviorProfile {
            successful_actions: [("like".to_string(), 5)].into(),
        };
        let temporal = TemporalFeatures {
            hour: 12,
            is_peak: true,
            ..Default::default()
        };
        let context = ContextFeatures {
            reply_count: 5,
            ..Default::default()
        };

        let prediction =
            scorer.predict_engagement("Great content!", &user_profile, &temporal, &context);

        assert!(prediction.success_probability > 0.0);
        assert!(prediction.confidence > 0.0);
    }
}
