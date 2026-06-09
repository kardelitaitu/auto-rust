# 0022-cleanup-predictive-scorer-dead-code

Status: `approved`

Owner: `spec-agent`
Implementer: `pending`

## Summary

Clean up 10 `#[allow(dead_code)]` annotations in `src/adaptive/predictive_scorer.rs` by removing the unused ML prediction types that were never wired into the system. The file was partially cleaned in spec 0013 (removed EngagementModel and FeatureExtractor fields from the struct), but the type definitions, constructors, and supporting structs remain.

This is a pure deletion — removing dead type definitions with no behavioral changes.

## Scope

- `src/adaptive/predictive_scorer.rs` only
- Remove: EngagementModel, ModelWeights, 5 feature extractor structs, 2 recommendation engine types
- Keep: ActionRecommender, UserBehaviorProfile, PredictiveEngagementScorer

## Next Steps

1. Review spec package
2. Remove dead types and their #[allow(dead_code)] annotations
3. Verify: cargo check, cargo test --lib predictive_scorer, cargo clippy
