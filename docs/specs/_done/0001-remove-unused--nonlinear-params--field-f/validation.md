## Validation
- `cargo check --lib` compiles without errors.
- `cargo test` passes all existing tests.
- `cargo clippy` shows no new warnings related to this change.
- Visual inspection confirms `NonlinearParams`, `ActivationFunction`, and `LayerConfig` are gone and `ModelWeights` has only `coefficients` and `bias`.
