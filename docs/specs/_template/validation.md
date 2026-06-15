last audited 16-06-26 by opencode

## Acceptance Criteria
<!-- Measurable conditions that prove the spec is done.
     Each criterion must be specific to THIS initiative, not generic.
     Bad:  "Generated spec package is complete and validated."
     Good: "The `nonlinear_params` field is removed from `ModelWeights` and
            `cargo check --lib` compiles without errors." -->

## Test Commands
<!-- Exact commands to run. Adapt to the change scope. -->
- `cargo check --lib`
- `cargo test`
- `cargo clippy`

## Visual Inspection
<!-- What to look for in the diff or file state after implementation.
     Example: "Confirm `NonlinearParams`, `ActivationFunction`, and `LayerConfig`
     are gone and `ModelWeights` has only `coefficients` and `bias`." -->
