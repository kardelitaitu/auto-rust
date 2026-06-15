# Remove Dead Fields and #[allow(dead_code)] from Predictive Scorer

## Baseline
The `PredictiveEngagementScorer` struct in `src/adaptive/predictive_scorer.rs:12` has two fields that are constructed in `new()` but NEVER read by any code path:

- `engagement_model: EngagementModel` (line 15, `#[allow(dead_code)]`)
- `feature_extractor: FeatureExtractor` (line 18, `#[allow(dead_code)]`)

The private method `predict_engagement()` (line 186) calls `EngagementModel::predict()` statically and `FeatureExtractor::extract_text_features()` etc. statically — it never accesses `self.engagement_model` or `self.feature_extractor`.

The remaining 10 structs (`EngagementModel`, `ModelWeights`, `ModelAccuracy`, `FeatureExtractor`, `TextFeatures`, `TemporalFeatures`, `UserFeatures`, `ContextFeatures`, `ActionRecommender`, `TimingRecommendations`) ARE used in production code paths via their static methods, but individual fields within them are only read in tests (not production code). Therefore `#[allow(dead_code)]` is retained on each struct.

## Implementation Steps
1. In `src/adaptive/predictive_scorer.rs`:
   a. Remove `engagement_model: EngagementModel` and its `#[allow(dead_code)]` from `PredictiveEngagementScorer`
   b. Remove `feature_extractor: FeatureExtractor` and its `#[allow(dead_code)]` from `PredictiveEngagementScorer`
   c. Remove both field constructions (`engagement_model: EngagementModel::new(), feature_extractor: FeatureExtractor::new(),`) from `new()`
   d. Retain `#[allow(dead_code)]` on all other structs whose fields are test-only
2. Run `cargo check --lib` to confirm no dead_code warnings
3. Run `cargo test` to confirm no regressions
4. Run `cargo clippy` to confirm no new warnings

## API Changes
No public API changes. `PredictiveEngagementScorer` is a `pub` struct but its fields are private (no `pub` on field names). The `new()` constructor signature is unchanged. All public methods (`benchmark_predict_engagement`, `predict_engagement`) are unchanged.

## Design Decisions and Risks
- **Why not remove the private structs too?** They are all used in production code: `EngagementModel::predict()` is called from `predict_engagement()`, `FeatureExtractor::*()` methods are called statically, `ActionRecommender` is used for recommendation and timing. Only the two stored-in-struct fields are dead.
- **Risk**: Very low. Removing dead fields and dead annotations is purely mechanical. The behavioral code path is untouched.
- Confidence: **High**
