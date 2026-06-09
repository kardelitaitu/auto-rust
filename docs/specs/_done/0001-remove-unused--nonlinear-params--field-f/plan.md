# Remove Unused `nonlinear_params` Field from `ModelWeights`

## Baseline
In `src/adaptive/predictive_scorer.rs`, the `ModelWeights` struct (line 34) contains a field `nonlinear_params: Option<NonlinearParams>` (line 40). The struct and its associated types (`NonlinearParams`, `ActivationFunction`, `LayerConfig`) are defined but never referenced in any logic. The `predict` method in `EngagementModel` (line 279) uses placeholder values instead of actual model weights, and no code accesses `nonlinear_params` for computation or serialization. The file also carries `#![allow(dead_code)]` at line 4, which suppresses compiler warnings about these unused types.

The `feature_extractor` and `action_recommender` fields on `PredictiveEngagementScorer` **are used**: `feature_extractor` is called in `predict_engagement` (lines 206–214), and `action_recommender` is called at lines 220 and 223. These fields should not be removed.

## Implementation Steps
1. Open `src/adaptive/predictive_scorer.rs`.
2. Remove the `nonlinear_params` field from `ModelWeights` (line 40).
3. Delete the `NonlinearParams` struct definition (lines 44–47).
4. Delete the `ActivationFunction` enum definition (lines 50–55).
5. Delete the `LayerConfig` struct definition (lines 58–62).
6. Run `cargo check` and confirm no compilation errors.
7. Run `cargo test` to confirm all tests pass.
8. Run `cargo clippy` and address any new warnings (there should be none from this change).

## API Changes
No API changes. All removed types are private to the module.

## Validation
- `cargo check --lib` compiles without errors.
- `cargo test` passes all existing tests.
- `cargo clippy` shows no new warnings related to this change.
- Visual inspection confirms `NonlinearParams`, `ActivationFunction`, and `LayerConfig` are gone and `ModelWeights` has only `coefficients` and `bias`.

## Design Decisions and Risks
- **Risk: Low**. The field and its type chain are purely private and unreferenced. No serialization format depends on them.
- The `#![allow(dead_code)]` attribute at line 4 has been masking this dead code; this cleanup removes the dead code but leaves the attribute in place since other stubbed types may still need it.
- The `feature_extractor` and `action_recommender` fields were examined and confirmed in use—no changes needed there.
- If future ML integration plans to use nonlinear parameters, the types can be re-added when the implementation is ready.

Confidence: High