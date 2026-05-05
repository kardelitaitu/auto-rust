//! Benchmarks for predictive engagement scoring
//!
//! Run with: `cargo bench --bench predictive_scorer`

use auto::adaptive::predictive_scorer::PredictiveEngagementScorer;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn benchmark_prediction(c: &mut Criterion) {
    let mut group = c.benchmark_group("prediction");
    let scorer = PredictiveEngagementScorer::new();

    let tweets = [
        "Short tweet",
        "This is a medium length tweet about technology and AI developments in the industry.",
        "🚀 Exciting news! Our team just launched a new feature that will change how people interact with social media. Check it out and share feedback. #innovation #tech #launch 🎉",
    ];

    for (idx, tweet) in tweets.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("tweet_length", idx), tweet, |b, tw| {
            b.iter(|| black_box(scorer.benchmark_predict_engagement(black_box(tw))))
        });
    }

    group.finish();
}

fn benchmark_recommendation_variants(c: &mut Criterion) {
    let mut group = c.benchmark_group("recommendation_variants");
    let scorer = PredictiveEngagementScorer::new();

    let tweets = [
        "Great article about Rust programming! Check it out.",
        "Huge launch today! We shipped a major upgrade with better performance and reliability.",
        "Quick reminder: the maintenance window starts at 10pm UTC.",
    ];

    for (idx, tweet) in tweets.iter().enumerate() {
        group.bench_with_input(BenchmarkId::new("variant", idx), tweet, |b, tw| {
            b.iter(|| {
                let prediction = scorer.benchmark_predict_engagement(black_box(tw));
                black_box(prediction.recommended_action)
            })
        });
    }

    group.finish();
}

fn benchmark_batch_predictions(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_predictions");
    let scorer = PredictiveEngagementScorer::new();

    let tweets_10: Vec<String> = (0..10)
        .map(|i| format!("Tweet number {} about technology and AI developments", i))
        .collect();

    let tweets_50: Vec<String> = (0..50)
        .map(|i| format!("Tweet number {} about technology and AI developments", i))
        .collect();

    group.bench_function("batch_10", |b| {
        b.iter(|| {
            for t in &tweets_10 {
                black_box(scorer.benchmark_predict_engagement(black_box(t)));
            }
        })
    });

    group.bench_function("batch_50", |b| {
        b.iter(|| {
            for t in &tweets_50 {
                black_box(scorer.benchmark_predict_engagement(black_box(t)));
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_prediction,
    benchmark_recommendation_variants,
    benchmark_batch_predictions
);
criterion_main!(benches);
