## Acceptance Criteria
1. `#[allow(dead_code)]` is removed from `src/adaptive/predictive_scorer.rs` entirely (12 occurrences)
2. `engagement_model` and `feature_extractor` fields no longer exist on `PredictiveEngagementScorer`
3. `cargo check --lib` compiles without dead_code warnings
4. All existing tests pass unchanged
5. `cargo clippy` shows no new warnings

## Test Commands
- `cargo check --lib`
- `cargo test`
- `cargo clippy`

## Visual Inspection
After implementation, confirm:
1. In `PredictiveEngagementScorer` struct: only `action_recommender: ActionRecommender` remains
2. No `#[allow(dead_code)]` appears anywhere in `predictive_scorer.rs`
3. `PredictiveEngagementScorer::new()` only constructs `action_recommender: ActionRecommender::new()`
