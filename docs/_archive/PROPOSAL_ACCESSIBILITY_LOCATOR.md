# Proposal: Accessibility Locator Support (Evidence-Based Revision)

last audited 08-05-26 by Kilo

Companion implementation spec: `docs/ACCESSIBILITY_LOCATOR_SPEC.md`

## Implementation Safety Checklist (Major Change Gate)

- [x] Approve locator grammar, ambiguity policy, and error taxonomy in `docs/ACCESSIBILITY_LOCATOR_SPEC.md`.
- [x] Confirm non-goals (no task API expansion, no hidden fallback from malformed locator syntax).
- [x] Baseline current behavior before coding:
  - [x] run `cargo check` (pass)
  - [x] run `cargo test` (pass; integration + doc tests completed, some suites intentionally ignored)
  - [x] record current pass/fail totals and flaky tests (if any)
- [x] Add implementation behind a feature flag (default off for first landing).
- [x] Implement parser first, with full unit coverage, before resolver wiring.
- [x] Implement thin CDP Accessibility resolver in shared navigation path only (single source of truth).
  - Current status: done for shared read helpers + action-point resolution (`selector_action_point`) consumed by action APIs; `nativeclick` remains explicitly CSS-only with deterministic `locator_unsupported`.
- [x] Keep `TaskContext` task-facing `api.*` signatures unchanged.
- [x] Add deterministic error mapping (`locator_parse_error`, `locator_not_found`, `locator_ambiguous`, `locator_scope_invalid`).
- [x] Add explicit unsupported-operation mapping for non-semantic helpers (`locator_unsupported` for `html`/`attr` with a11y locator input).
- [x] Add compatibility tests proving CSS selector behavior is unchanged.
  - added CSS-compat regression matrix in parser + navigation routing tests
- [x] Add ambiguity and missing-target tests proving deterministic failure semantics.
  - implemented as unit-level resolver classification tests in `src/utils/navigation.rs`
- [x] Add observability fields (`selector_mode`, `locator_role`, `locator_result`) and validate log output.
  - implemented with `tracing::debug!` fields in `src/utils/navigation.rs`
- [x] Run full verification suite again:
  - [x] run `cargo check`
  - [x] run `cargo test`
- [ ] Roll out in phases:
  - [x] migrate one high-value task (`twitterfollow` pilot: locator-first follow/following detection with CSS/JS fallback)
