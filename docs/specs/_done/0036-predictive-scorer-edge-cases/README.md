# Predictive Scorer Edge Cases

Status: `done`

Owner: `spec-agent`
## Summary

Add property-based regression coverage around `src/adaptive/predictive_scorer.rs` so scorer outputs stay finite and bounded across arbitrary inputs without changing the model layout or persistence story.

## Scope

- In scope:
  - property tests for score bounds and finite outputs
  - feature extractor regressions for empty or extreme inputs
  - one or two small fixed regressions for edge-case examples
- Out of scope:
  - persistence or save/load behavior
  - model architecture changes
  - new learning-engine APIs
  - external dependency additions

## Baseline

- `src/adaptive/predictive_scorer.rs` already contains the scorer, feature extractor, model weights, and broad fixed-case unit tests.
- The file already checks confidence and expected-engagement bounds on a normal input.
- The remaining work is proving those invariants hold for arbitrary or extreme inputs, not redesigning the scorer.
- Persistence is not implemented here, so it should not be pulled into this spec.

## Why This Was Needed

- The existing tests were mostly fixed examples.
- The archived spec added property-style coverage proving outputs stay finite under varied inputs.
- The package pinned the scorer invariants that mattered most for reliability.

## Files

- `spec.yaml`
- `plan.md`
- `validation.md`
- `notes.md`

## Archive Notes

This package is complete and retained as a reference record for scorer edge-case coverage.
