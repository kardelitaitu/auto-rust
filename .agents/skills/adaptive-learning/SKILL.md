# Adaptive Learning

Comprehensive guide to the adaptive automation systems — click learning engine, predictive engagement scoring, self-healing, and click timing adaptation.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                     Adaptive Systems                          │
│                                                              │
│  LearningEngine       PredictiveEngagementScorer    SelfHealingSystem
│  ┌─────────────┐     ┌──────────────────┐        ┌───────────────────┐
│  │ ClickLearning│     │ FeatureExtractors │        │ HealthMonitor     │
│  │ State       │     │ Text/User/Temporal│        │ FailureHistory    │
│  │ Persistence │     │ Context Features  │        │ RecoveryStrategies│
│  │ TTL Expiry  │     │ ActionRecommender │        │ RecoveryState     │
│  └─────────────┘     └──────────────────┘        └───────────────────┘
│           ↕                     ↕                         ↕
│    click_learning.rs    predictive_scorer.rs      self_healing/*.rs
└──────────────────────────────────────────────────────────────┘
```

The adaptive systems provide:
1. **Click Learning** — tracks selector success/failure, adapts timing profiles based on page context, element priority, fatigue, and historical performance
2. **Predictive Scorer** — ML-based engagement prediction using text, user, temporal, and context feature vectors + action recommendations
3. **Self-Healing** — health monitoring, failure history tracking, recovery state machine, and strategies for automatic recovery

---

## File Map

| File | Purpose |
|---|---|
| `src/adaptive/mod.rs` | Module root — exports `LearningEngine` |
| `src/adaptive/learning_engine.rs` | `LearningEngine` — click learning persistence service, TTL-based expiry, privacy controls, save/load/clear |
| `src/adaptive/predictive_scorer.rs` | `PredictiveEngagementScorer` — ML-based engagement prediction, 4 feature extractors (text/user/temporal/context), `ActionRecommender` with decision rules, `EngagementPrediction` |
| `src/adaptive/self_healing/mod.rs` | Module root — re-exports all self-healing submodules |
| `src/adaptive/self_healing/health.rs` | `HealthMonitor`, `SystemHealth` (Healthy/Degraded/Recovering/Critical/Offline), `HealthCheckResult`, `HealthCheckType`, `HealthCheckStatus` |
| `src/adaptive/self_healing/history.rs` | `FailureHistory`, `FailureRecord`, `FailurePattern`, `FailureType` (6 variants), `ImpactLevel` — VecDeque-based recent failures |
| `src/adaptive/self_healing/state.rs` | `RecoveryState`, `ActiveRecovery`, `RecoveryProgress`, `RecoveryMode` (Normal/Recovering/Degraded/Emergency), `RecoveryType`, `RecoveryStatus` |
| `src/adaptive/self_healing/strategy.rs` | `RecoveryStrategies` (connection/resource/error/performance), `RecoveryActionType` (6 variants), `ErrorCategory`, `ErrorSeverity`, `ErrorProcedure`, `ResourceScaling`, `ResourceCleanup`, `ConnectionRecovery` |
| `src/adaptive/self_healing/system.rs` | `SelfHealingSystem` — orchestrates health monitoring, failure detection (`button_missing > 10`), and recovery initiation |
| `src/runtime/task_context/click_learning.rs` | Core click learning types: `ClickTimingContext`, `ClickTimingProfile`, `ClickAdaptation`, `ClickLearningState`, `SelectorLearningStats`, `ClickPageContext`, `ClickElementPriority`, `ClickFatigueLevel`, timing profile computation, persistence helpers |
| `src/benchmarks/predictive_scorer.rs` | Criterion benchmarks for prediction speed (tweet length variants, recommendation variants, batch sizes) |

---

## Learning Engine (`learning_engine.rs`)

The `LearningEngine` is a service-based API that wraps `ClickLearningState` with persistence and lifecycle management.

### Construction

| Constructor | Description |
|---|---|
| `new(session_id, behavior_profile, enabled, ttl_days)` | Loads existing state from disk, prunes expired entries on init |
| `disabled()` | Creates no-op engine — all methods return Ok/defaults |

### Key Methods

| Method | Description |
|---|---|
| `record(selector, success)` | Records click result, auto-saves to disk |
| `adaptation_for(selector, context)` | Returns `ClickAdaptation` based on selector stats + context |
| `selector_stats(selector)` | Returns `SelectorLearningStats` for a selector |
| `clear()` | Resets state and deletes file from disk |
| `prune_expired()` | Removes selectors older than `ttl_days` (no-op if `ttl_days == 0`) |
| `save()` | Persists state to JSON file at `click-learning/{profile}/{session}.json` |
| `recent_success_rate()` | Success rate of last 32 interactions |
| `clear_all()` | Removes entire `click-learning/` directory |

### Path Scheme
```
click-learning/{sanitized_profile_name}/{sanitized_session_id}.json
```
- `sanitize_path_component()`: keeps `[a-zA-Z0-9_-]`, replaces others with `_`, trims leading/trailing `_`, defaults to `"default"` if empty
- Backward compatibility: sets `last_updated` to `Utc::now()` for entries missing it

### TTL Pruning
- `prune_expired()` runs on construction (if `ttl_days > 0`)
- Uses `chrono::Duration::days(ttl_days)` as cutoff
- Preserves entries with `last_updated.is_none()` (backward compat)
- Only saves to disk if entries were actually pruned

---

## Predictive Engagement Scorer (`predictive_scorer.rs`)

The scorer predicts engagement success probability and recommends optimal actions.

### Feature Extraction Pipeline

```
TextFeatures        UserFeatures        TemporalFeatures        ContextFeatures
├─ sentiment        ├─ reputation        ├─ hour (0-23)          ├─ thread_depth
├─ length           ├─ follower_count    ├─ day_of_week (0-6)    ├─ reply_count
├─ keywords (map)   ├─ following_count   ├─ is_peak              ├─ has_media
├─ readability      ├─ account_age       ├─ time_since_last      ├─ topic_category
└─ emotion          └─ engagement_rate   └─ posting_frequency    └─ trending_score
        │                   │                      │                      │
        └───────────────────┴──────────────────────┴──────────────────────┘
                                      ↓
                              FeatureVector (combined)
                                      ↓
                              predict_model()
                              (simplified: base 0.5, conf 0.8)
                                      ↓
                            EngagementPrediction
```

### Action Recommender Rules
| Condition | Recommended Action |
|---|---|
| `text.length > 140` | `"Reply"` |
| `reply_count > 5` | `"Retweet"` |
| `engagement_rate > 0.15` | `"Like"` |
| `is_peak` (peak hour) | `"Follow"` |
| Default | `"Skip"` |

### EngagementPrediction Fields
| Field | Range | Description |
|---|---|---|
| `success_probability` | 0.0 - 1.0 | Probability of success |
| `expected_engagement` | 0.0 - 1.0 | `probability * confidence` |
| `recommended_action` | — | Action type string |
| `optimal_time` | 0 - 23 | Best hour for engagement |
| `confidence` | 0.0 - 1.0 | Model confidence |
| `key_factors` | Vec<String> | Influencing factors (currently `["sentiment", "timing"]`) |

### Timing Recommendations
- Default optimal times: `[9, 12, 18]` (morning, noon, evening)
- Best days: `[1, 3, 5]` (Mon, Wed, Fri)
- Peak hour detection: uses the provided `is_peak` flag from temporal features

### Benchmarks
- Run with: `cargo bench --bench predictive_scorer`
- 3 benchmark groups:
  - `prediction` — tweet length variants (short/medium/long)
  - `recommendation_variants` — different tweet content types
  - `batch_predictions` — batch sizes of 10 and 50

---

## Click Learning (`click_learning.rs`)

### Page Context Classification

| URL Pattern | `ClickPageContext` |
|---|---|
| `x.com` or `twitter.com` | `Social` |
| `login`, `signup`, `form` | `Form` |
| `shop`, `cart`, `checkout` | `Commerce` |
| `article`, `news`, `blog` | `Content` |
| `/` only (2+ segments) | `Home` |
| Everything else | `Other` |

### Element Priority Classification

| Selector Contains | `ClickElementPriority` |
|---|---|
| `submit`, `confirm`, `primary`, `cta` | `Critical` |
| `ad`, `promo`, `secondary` | `Optional` |
| Everything else | `Normal` |

### Fatigue Levels

| Interaction Count | `ClickFatigueLevel` |
|---|---|
| 0 - 14 | `Rested` |
| 15 - 49 | `Normal` |
| 50+ | `Tired` |

### Timing Profile Computation

`ClickTimingProfile` is computed from:
1. **Base values**: `reaction_delay_ms`, `variance_pct`, `offset_px`
2. **Multipliers**: page (0.95-1.20) × priority (0.92-1.18) × fatigue (0.95-1.22) × quality × adaptation
3. **Clamping**: reaction_delay [70, 6000], variance [8, 80], offset [2, 24], attention_pause [40, 800], post_click [120, 900], timeout [3200, 7500]

### Click Learning State

| Field | Type | Description |
|---|---|---|
| `interaction_count` | `u64` | Total interactions |
| `total_attempts` | `u64` | Total click attempts |
| `total_successes` | `u64` | Total click successes |
| `recent_results` | `VecDeque<bool>` | Last 32 results (sliding window) |
| `selectors` | `HashMap<String, SelectorLearningStats>` | Per-selector stats |

### SelectorLearningStats

| Field | Type | Description |
|---|---|---|
| `attempts` | `u32` | Times this selector was clicked |
| `successes` | `u32` | Times click succeeded |
| `consecutive_failures` | `u32` | Current streak of failures |
| `last_updated` | `Option<DateTime<Utc>>` | For TTL management |

### ClickAdaptation (computed output)

| Field | Type | Conditions That Increase |
|---|---|---|
| `extra_stability_wait_ms` | `u64` | Complexity (120), low success rate (250), consecutive failures (380), tired (140) |
| `reaction_delay_multiplier` | `f64` | Complexity (1.08), low success rate (1.20), consecutive failures (1.22), tired (1.15) |
| `reaction_variance_boost_pct` | `u32` | Low success rate (+8), consecutive failures (+10), tired (+6) |
| `click_offset_adjustment_px` | `i32` | Complexity (+1), consecutive failures (+2) |
| `require_strict_verification` | `bool` | Low success rate, consecutive failures (≥2) |
| `prefer_coordinate_fallback` | `bool` | Low success rate, consecutive failures (≥2) |

---

## Self-Healing System (`self_healing/`)

### System Health States

```
Healthy → Degraded → Critical → Offline
                ↑         ↓
           Recovering ←──┘
```

### HealthMonitor
| Field | Type |
|---|---|
| `status` | `SystemHealth` |
| `checks` | `Vec<HealthCheckResult>` |
| `last_check` | `Instant` |
| `consecutive_failures` | `u32` |

### HealthCheckResult
| Field | Description |
|---|---|
| `check_id` | Unique identifier |
| `check_type` | Connection/Resource/Performance/ErrorRate/Api |
| `status` | Passed/Failed/Skipped |
| `error` | Optional error message |
| `recovery_action` | Optional `RecoveryActionType` to execute |

### FailureHistory
| Field | Description |
|---|---|
| `recent_failures` | `VecDeque<FailureRecord>` — recent failures in order |
| `patterns` | `Vec<FailurePattern>` — identified failure patterns |
| `mtbf` | Mean time between failures |
| `mttr` | Mean time to recovery |

### Failure Pattern Detection
- 6 failure types: Connection, Resource, Api, Timeout, Data, Unknown
- 4 impact levels: Low, Medium, High, Critical
- Pattern has `signature` (Vec<String>), `frequency`, and `impact`

### Recovery State Machine

| `RecoveryMode` | Description |
|---|---|
| `Normal` | Everything operating as expected |
| `Recovering` | Recovery action in progress |
| `Degraded` | Running at reduced capacity |
| `Emergency` | Critical state, immediate action needed |

### Recovery Strategies

| Strategy | Key Parameters |
|---|---|
| `ConnectionRecovery` | max_retries (3), retry_delay (1s), backoff_factor (2.0x), fallback_endpoints |
| `ResourceRecovery` | `ResourceScaling` (scale_up_threshold: 0.8, max_scale_factor: 2.0) + `ResourceCleanup` (interval: 300s, threshold: 0.9) |
| `ErrorRecovery` | Classifications + procedures; `select_recovery_action()` returns `RestartService` (simplified) |
| `PerformanceRecovery` | `PerformanceTuning` (target_level: 0.0, adjustment_factor: 0.0) |

### SelfHealingSystem

| Method | Description |
|---|---|
| `check_health()` | Returns `HealthCheckResult` — Critical if >5 failures, Healthy otherwise |
| `detect_and_recover(metrics)` | Checks `button_missing > 10`, initiates recovery if true |
| `record_adaptation()` | No-op (placeholder for future) |

### Detection Logic (currently simplified)
- `detect_failure()`: `metrics.button_missing > 10` → triggers recovery
- `initiate_recovery()`: calls `select_recovery_action()` → `execute_recovery()` → `update_recovery_state()`
- `execute_recovery()`: always returns success (simplified)

### RecoveryActionType Variants
| Variant | Description |
|---|---|
| `RestartService` | Restart the service |
| `ScaleResources(f32)` | Scale by a factor (e.g., 2.0 = double) |
| `SwitchToBackup(String)` | Switch to named backup |
| `ResetState` | Reset internal state |
| `AlertOperator(String)` | Notify human operator |
| `Custom(String)` | Custom action |

---

## Adding New Learning Signals

### New click adaptation signal
1. Add field to `ClickAdaptation` struct in `click_learning.rs`
2. Set conditions in `ClickLearningState::adaptation_for()`
3. Consume in `ClickTimingContext::timing_profile()` or downstream
4. Add persistence test in `click_learning.rs` tests

### New feature extractor
1. Add struct in `predictive_scorer.rs` (e.g., `NetworkFeatures`)
2. Add extract function (e.g., `extract_network_features()`)
3. Add to `FeatureVector` struct
4. Update `combine_features()` function
5. Adjust `predict_model()` or action rules if needed
6. Add proptests for the new feature type

### New failure detection metric
1. Add detection method to `SelfHealingSystem`
2. Check the metric in `detect_and_recover()`
3. Optionally create new `RecoveryActionType` variant
4. Add tests for both detection and recovery

---

## Testing

| Test Location | Command |
|---|---|
| Learning engine unit tests | `cargo test --lib adaptive::learning_engine::tests` |
| Predictive scorer unit tests | `cargo test --lib predictive_scorer::tests` |
| Predictive scorer proptests | `cargo test --lib predictive_scorer::property_tests` |
| Predictive scorer integration | `cargo test --lib predictive_scorer::integration_tests` |
| Click learning tests | `cargo test --lib task_context::click_learning::tests` |
| Self-healing health tests | `cargo test --lib self_healing::health::tests` |
| Self-healing history tests | `cargo test --lib self_healing::history::tests` |
| Self-healing state tests | `cargo test --lib self_healing::state::tests` |
| Self-healing strategy tests | `cargo test --lib self_healing::strategy::tests` |
| Self-healing system tests | `cargo test --lib self_healing::system::tests` |
| All adaptive module tests | `cargo test --lib adaptive::` |
| Click learning tests (full path) | `cargo test --lib runtime::task_context::click_learning::tests` |
| Benchmarks | `cargo bench --bench predictive_scorer` |

---

## Pitfalls

| # | Pitfall | Explanation |
|---|---|---|
| 1 | **predict_model is a stub** | The current `predict_model()` always returns `(0.5, 0.8, ["sentiment", "timing"])`. It's a placeholder for future ML integration — predictions are NOT data-driven. |
| 2 | **detect_failure is a simple threshold** | Only checks `button_missing > 10`, not the full metric set. Recovery always succeeds (simplified). This system is a foundation, not production-ready self-healing. |
| 3 | **Learning engine persistent by default** | `learning_data_path()` writes to `click-learning/` in the current working directory. If the CWD changes (e.g., different machine), data is lost. |
| 4 | **TTL prune only runs on construction** | Pruning runs once in `new()` but not periodically. Stale selectors accumulate until the engine is reconstructed. |
| 5 | **Fatigue threshold is hardcoded** | <15=Rested, 15-49=Normal, 50+=Tired. These aren't configurable and may not match all interaction patterns. |
| 6 | **Selector stats saturate** | Uses `saturating_add` to prevent overflow. At u32::MAX (~4.3B attempts), counters stop incrementing. |
| 7 | **Self-healing test-only patterns** | Many self-healing structs (`ShutdownSession`, `ManagedTabCleanup`) are behind `#[cfg(test)]` — they're not used in production. |
| 8 | **Benchmarks use default features** | Benchmarks use `UserBehaviorProfile::default()` and `TemporalFeatures::default()`, not real data profiles. |
| 9 | **ActionRecommender is rule-based** | The recommender is a simple if/else chain, not ML. Rules are based on hardcoded thresholds (140 chars, 5 replies, 0.15 engagement rate). |
| 10 | **self_healing/system is skeletal** | `record_adaptation()` is empty, `check_health()` always returns Passed unless >5 failures, recovery is always successful. This is a framework, not a complete system. |

> last audited 26-06-26 by docs-auditor
