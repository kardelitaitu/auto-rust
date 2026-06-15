# Validation Checklist

- [ ] `cargo check` — 0 errors
- [ ] `cargo test --lib predictive_scorer` — all tests pass
- [ ] `cargo test --lib` — full test suite passes
- [ ] `cargo clippy --all-targets --all-features` — 0 dead_code warnings in predictive_scorer.rs
- [ ] `predictive_scorer.rs` ≤ 530 lines (down from 825)
- [ ] `#[allow(dead_code)]` count reduced from 10→0 in this file
- [ ] `ActionRecommender` still functional (tests pass)
- [ ] `PredictiveEngagementScorer::new()` still works
