# Baseline

## What I Find

`src/adaptive/predictive_scorer.rs` (825 lines) has 10 `#[allow(dead_code)]` annotations protecting these unused types:

| Lines | Dead Type | Purpose |
|-------|-----------|---------|
| 18 | EngagementModel | ML prediction model (never used) |
| 29 | ModelWeights | Model parameter storage (never used) |
| 38 | ModelAccuracy | Accuracy tracking struct (never used) |
| 53 | TextFeatureExtractor | Text-based feature extraction (never used) |
| 66 | TemporalFeatureExtractor | Time-based feature extraction (never used) |
| 82 | UserFeatureExtractor | User-based feature extraction (never used) |
| 98 | ContextualFeatureExtractor | Context-based feature extraction (never used) |
| 114 | FeatureExtractor | Aggregated feature extraction (never used) |
| 130 | ActionRecommendationEngine | Action recommendation (never used) |
| 141 | TimingRecommendation | Timing recommendation (never used) |

Spec 0013 already removed these from `PredictiveEngagementScorer`'s struct fields (keeping only `action_recommender: ActionRecommender`), but the type definitions and constructors remain in the file.

**Actively used types** (no dead_code annotations):
- `UserBehaviorProfile` (line 10)
- `PredictiveEngagementScorer` (line ~170)
- `ActionRecommender` (line ~150)

## What I Claim

Removing all 10 dead type definitions will reduce predictive_scorer.rs by ~300 lines (825→~525), eliminate all dead_code warnings, and leave the file focused on its active concern: `PredictiveEngagementScorer` backed by `ActionRecommender`.

## What Is the Proof

1. **10 dead_code annotations** are the highest count in any single file — double the next worst (twitter_helpers.rs at 7).

2. **Spec 0013 already confirmed these are unused**: The struct fields were removed from `PredictiveEngagementScorer` in the prior cleanup pass. The type definitions are vestigial.

3. **No imports reference these types**: Searching the codebase, none of these structs are imported or instantiated outside predictive_scorer.rs. They're file-local dead weight.
