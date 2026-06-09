# Plan

## What Is the Solution

Delete all 10 dead type definitions and their constructors from `predictive_scorer.rs`:

| Delete | Lines | Reason |
|--------|-------|--------|
| `EngagementModel` struct + impl | ~17-28 | ML model never instantiated |
| `ModelWeights` struct | ~29-37 | Model parameters never used |
| `ModelAccuracy` struct | ~38-52 | Accuracy tracking never used |
| `TextFeatureExtractor` struct + impl | ~53-65 | Text features never extracted |
| `TemporalFeatureExtractor` struct + impl | ~66-81 | Temporal features never extracted |
| `UserFeatureExtractor` struct + impl | ~82-97 | User features never extracted |
| `ContextualFeatureExtractor` struct + impl | ~98-113 | Context features never extracted |
| `FeatureExtractor` struct + impl (new) | ~114-129 | Feature extractor never constructed |
| `ActionRecommendationEngine` struct + impl | ~130-140 | Action recommendation never used |
| `TimingRecommendation` struct | ~141-146 | Timing recommendation never used |

**After deletion**: Remove all 10 `#[allow(dead_code)]` annotations. The file reduces from 825→~525 lines with 0 dead_code warnings.

**Kept untouched**: `UserBehaviorProfile`, `ActionRecommender`, `PredictiveEngagementScorer`, all existing tests.
