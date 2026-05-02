//! Benchmarks for predictive engagement scoring
//!
//! Run with: `cargo bench --bench predictive_scorer`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

/// Engagement prediction result
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct EngagementPrediction {
    success_probability: f64,
    confidence_score: f64,
    recommended_action: ActionType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ActionType {
    Like,
    Retweet,
    Reply,
    Follow,
    Skip,
}

/// User behavior profile
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct UserBehaviorProfile {
    avg_engagement_rate: f64,
    preferred_topics: Vec<String>,
    active_hours: Vec<u8>,
    device_type: DeviceType,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum DeviceType {
    Mobile,
    Desktop,
}

/// Predictive engagement scorer
struct PredictiveEngagementScorer {
    model_weights: ModelWeights,
}

struct ModelWeights {
    text_coefficients: Vec<f64>,
    user_coefficients: Vec<f64>,
    temporal_coefficients: Vec<f64>,
    bias: f64,
}

impl PredictiveEngagementScorer {
    fn new() -> Self {
        Self {
            model_weights: ModelWeights {
                text_coefficients: vec![0.3, 0.2, 0.15, 0.1, 0.05],
                user_coefficients: vec![0.4, 0.3, 0.2],
                temporal_coefficients: vec![0.25, 0.15],
                bias: -0.5,
            },
        }
    }

    /// Predict engagement for a tweet
    fn predict_engagement(
        &self,
        tweet_text: &str,
        user_profile: &UserBehaviorProfile,
    ) -> EngagementPrediction {
        // Extract text features (simplified)
        let text_features = self.extract_text_features(tweet_text);

        // Extract user features
        let user_features = self.extract_user_features(user_profile);

        // Extract temporal features
        let temporal_features = self.extract_temporal_features();

        // Calculate weighted sum
        let text_score: f64 = text_features
            .iter()
            .zip(&self.model_weights.text_coefficients)
            .map(|(f, w)| f * w)
            .sum();

        let user_score: f64 = user_features
            .iter()
            .zip(&self.model_weights.user_coefficients)
            .map(|(f, w)| f * w)
            .sum();

        let temporal_score: f64 = temporal_features
            .iter()
            .zip(&self.model_weights.temporal_coefficients)
            .map(|(f, w)| f * w)
            .sum();

        let raw_score = text_score + user_score + temporal_score + self.model_weights.bias;
        let probability = sigmoid(raw_score);

        // Determine recommended action
        let action = if probability > 0.8 {
            ActionType::Reply
        } else if probability > 0.6 {
            ActionType::Retweet
        } else if probability > 0.4 {
            ActionType::Like
        } else if probability > 0.2 {
            ActionType::Follow
        } else {
            ActionType::Skip
        };

        EngagementPrediction {
            success_probability: probability,
            confidence_score: (probability * (1.0 - probability)).sqrt() * 2.0,
            recommended_action: action,
        }
    }

    fn extract_text_features(&self, text: &str) -> Vec<f64> {
        let length = text.len() as f64;
        let word_count = text.split_whitespace().count() as f64;
        let emoji_count = text.chars().filter(|c| c.is_ascii_punctuation()).count() as f64;
        let hashtag_count = text.matches('#').count() as f64;
        let mention_count = text.matches('@').count() as f64;

        vec![
            (length / 280.0).min(1.0),      // Normalized length
            (word_count / 50.0).min(1.0),   // Normalized word count
            (emoji_count / 5.0).min(1.0),   // Normalized emoji count
            (hashtag_count / 3.0).min(1.0), // Normalized hashtag count
            (mention_count / 5.0).min(1.0), // Normalized mention count
        ]
    }

    fn extract_user_features(&self, profile: &UserBehaviorProfile) -> Vec<f64> {
        vec![
            profile.avg_engagement_rate,
            (profile.preferred_topics.len() as f64 / 10.0).min(1.0),
            (profile.active_hours.len() as f64 / 24.0),
        ]
    }

    fn extract_temporal_features(&self) -> Vec<f64> {
        // Simplified - in real implementation, use actual time
        vec![
            0.5, // Hour of day normalized
            0.7, // Day of week normalized
        ]
    }
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Recommend best action from candidates
fn recommend_action(
    scorer: &PredictiveEngagementScorer,
    tweet_text: &str,
    user_profile: &UserBehaviorProfile,
    candidates: &[ActionType],
) -> ActionType {
    let prediction = scorer.predict_engagement(tweet_text, user_profile);

    // Filter to available candidates
    if candidates.contains(&prediction.recommended_action) {
        prediction.recommended_action
    } else {
        // Pick highest probability from available
        candidates
            .iter()
            .max_by(|a, b| {
                let score_a = action_priority(**a);
                let score_b = action_priority(**b);
                score_a.partial_cmp(&score_b).unwrap()
            })
            .copied()
            .unwrap_or(ActionType::Skip)
    }
}

fn action_priority(action: ActionType) -> f64 {
    match action {
        ActionType::Reply => 1.0,
        ActionType::Retweet => 0.8,
        ActionType::Like => 0.6,
        ActionType::Follow => 0.4,
        ActionType::Skip => 0.0,
    }
}

fn benchmark_prediction(c: &mut Criterion) {
    let mut group = c.benchmark_group("prediction");
    let scorer = PredictiveEngagementScorer::new();

    let profile = UserBehaviorProfile {
        avg_engagement_rate: 0.15,
        preferred_topics: vec!["tech".to_string(), "ai".to_string()],
        active_hours: vec![9, 12, 18, 20],
        device_type: DeviceType::Mobile,
    };

    let tweets = [
        "Short tweet",  // 11 chars
        "This is a medium length tweet about technology and AI developments in the industry.",  // 80 chars
        "🚀 Exciting news! Our team just launched a new feature that will revolutionize how you interact with social media. Check it out and let us know what you think! #innovation #tech #launch 🎉",  // 200+ chars with emoji
    ];

    for (idx, tweet) in tweets.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("tweet_length", idx), tweet, |b, tw| {
            b.iter(|| scorer.predict_engagement(black_box(tw), black_box(&profile)))
        });
    }

    group.finish();
}

fn benchmark_feature_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("feature_extraction");
    let scorer = PredictiveEngagementScorer::new();

    let short_text = "Hello world!";
    let medium_text = "This is a tweet about technology and AI developments. Check out the latest news! #tech #AI";
    let long_text = "🚀 Big announcement! We're launching something amazing today. This has been months in the making and we can't wait for you to try it. The team worked incredibly hard to bring this to life. Let us know your thoughts! #launch #startup #innovation 🎉✨";

    group.bench_function("text_short", |b| {
        b.iter(|| scorer.extract_text_features(black_box(short_text)))
    });

    group.bench_function("text_medium", |b| {
        b.iter(|| scorer.extract_text_features(black_box(medium_text)))
    });

    group.bench_function("text_long", |b| {
        b.iter(|| scorer.extract_text_features(black_box(long_text)))
    });

    let profile = UserBehaviorProfile {
        avg_engagement_rate: 0.2,
        preferred_topics: vec!["tech".to_string(), "ai".to_string(), "rust".to_string()],
        active_hours: vec![8, 12, 18, 20, 22],
        device_type: DeviceType::Desktop,
    };

    group.bench_function("user_profile", |b| {
        b.iter(|| scorer.extract_user_features(black_box(&profile)))
    });

    group.finish();
}

fn benchmark_action_recommendation(c: &mut Criterion) {
    let mut group = c.benchmark_group("action_recommendation");
    let scorer = PredictiveEngagementScorer::new();

    let profile = UserBehaviorProfile {
        avg_engagement_rate: 0.2,
        preferred_topics: vec!["tech".to_string()],
        active_hours: vec![12, 18],
        device_type: DeviceType::Mobile,
    };

    let tweet = "Great article about Rust programming! Check it out.";

    let candidates_2 = [ActionType::Like, ActionType::Skip];
    let candidates_3 = [ActionType::Like, ActionType::Retweet, ActionType::Skip];
    let candidates_5 = [
        ActionType::Like,
        ActionType::Retweet,
        ActionType::Reply,
        ActionType::Follow,
        ActionType::Skip,
    ];

    group.bench_with_input(
        BenchmarkId::new("candidates", 2),
        &candidates_2,
        |b, cands| {
            b.iter(|| {
                recommend_action(
                    black_box(&scorer),
                    black_box(tweet),
                    black_box(&profile),
                    black_box(cands),
                )
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("candidates", 3),
        &candidates_3,
        |b, cands| {
            b.iter(|| {
                recommend_action(
                    black_box(&scorer),
                    black_box(tweet),
                    black_box(&profile),
                    black_box(cands),
                )
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("candidates", 5),
        &candidates_5,
        |b, cands| {
            b.iter(|| {
                recommend_action(
                    black_box(&scorer),
                    black_box(tweet),
                    black_box(&profile),
                    black_box(cands),
                )
            })
        },
    );

    group.finish();
}

fn benchmark_batch_predictions(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_predictions");
    let scorer = PredictiveEngagementScorer::new();

    let profile = UserBehaviorProfile {
        avg_engagement_rate: 0.15,
        preferred_topics: vec!["tech".to_string()],
        active_hours: vec![9, 18],
        device_type: DeviceType::Mobile,
    };

    let tweets_10: Vec<String> = (0..10)
        .map(|i| format!("Tweet number {} about technology and AI developments", i))
        .collect();

    let tweets_50: Vec<String> = (0..50)
        .map(|i| format!("Tweet number {} about technology and AI developments", i))
        .collect();

    group.bench_function("batch_10", |b| {
        b.iter(|| {
            for t in &tweets_10 {
                black_box(scorer.predict_engagement(black_box(t), black_box(&profile)));
            }
        })
    });

    group.bench_function("batch_50", |b| {
        b.iter(|| {
            for t in &tweets_50 {
                black_box(scorer.predict_engagement(black_box(t), black_box(&profile)));
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_prediction,
    benchmark_feature_extraction,
    benchmark_action_recommendation,
    benchmark_batch_predictions
);
criterion_main!(benches);
