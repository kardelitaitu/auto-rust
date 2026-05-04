# Proposal: Accessibility Locator Support (Evidence-Based Revision)

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
  - [ ] monitor telemetry/error rates
  - [ ] expand migration only when stable
- [ ] Define rollback trigger and rollback action before enabling feature flag by default.
- [x] Update `src/task/SELECTOR.md` and cross-reference this proposal + spec.

### Baseline Snapshot (2026-04-29)

- `cargo check`: pass
- `cargo test`: pass (unit + integration + doc tests completed in current run)
- Ignored tests observed (expected in this repo): soak/orchestrator subsets and one overlay-state test marked as ignored
- Baseline blocker resolved by fixing example imports:
  - `src/task/demo-interaction-keyboard.rs` now imports `auto::utils::timing::DEFAULT_DEMO_DURATION_MS`
  - `src/task/demo-interaction-mouse.rs` now imports `auto::utils::timing::DEFAULT_DEMO_DURATION_MS`
- Gate 2 parser milestone completed:
  - feature flag added: `accessibility-locator`
  - parser module added: `src/utils/accessibility_locator.rs`
  - parser tests under feature flag: 9 passed, 0 failed
- Feature-on verification rerun:
  - `cargo check --features accessibility-locator`: pass
  - `cargo test --features accessibility-locator`: pass
  - doc-test summary (captured): `test result: ok. 63 passed; 0 failed; 9 ignored; 0 measured; 0 filtered out; finished in 3.52s`

## Implementation Status (Code-Proven, 2026-04-29)

What is already landed (feature-gated):

| Item | Proof in code | Status |
|---|---|---|
| Feature flag exists | `Cargo.toml` contains `accessibility-locator = []` | Done |
| Parser module exists | `src/utils/accessibility_locator.rs` with `parse_selector_input(...)` | Done |
| Parser rejects malformed `role=` syntax | `parse_selector_for_navigation(...)` maps to `locator_parse_error: ...` in `src/utils/navigation.rs` | Done |
| Resolver uses CDP Accessibility domain | `QueryAxTreeParams`, `EnableParams`, `GetDocumentParams` imported/used in `src/utils/navigation.rs` | Done (for current covered helpers) |
| Resolver wired to shared selector path | `selector_exists`, `selector_is_visible`, `selector_text`, `selector_value` dispatch on `ParsedSelector`; `selector_html`/`selector_attr` parser-aware with explicit unsupported mapping in `src/utils/navigation.rs` | Done |
| Action-path resolver wired through TaskContext | `selector_action_point(...)` + `selector_uses_accessibility_locator(...)` + `focus_at_point(...)` used by `focus`, `hover`, `click`, `double_click`, `right_click`, `middle_click`, `drag`, typing/select-all verification branches in `src/runtime/task_context.rs` | Done |
| Ambiguity error mapping exists | error string `locator_ambiguous: role='{}' name='{}' ...` in `src/utils/navigation.rs`; runtime assertions in `tests/task_api_behavior.rs` | Done |
| Scope error mapping exists | error string `locator_scope_invalid: ...` in `src/utils/navigation.rs`; runtime assertions in `tests/task_api_behavior.rs` | Done |
| CSS fallback preserved | `ParsedSelector::Css(css) => ...` branches call existing CSS helpers | Done |
| Task API signatures unchanged | no `TaskContext` selector method signature changes required | Done |

Still pending to reach v1 done:
- [done] broader browser-runtime compatibility tests now exist in `tests/task_api_behavior.rs`:
  - `browser_runtime_css_compatibility_matrix_under_feature_flag`
  - `browser_runtime_accessibility_locator_integration_semantics`
- [done] selector observability assertions now validate emitted telemetry fields:
  - unit: `src/utils/navigation.rs::tests::test_selector_observation_logs_css_mode_result_fields`
  - unit: `src/utils/navigation.rs::tests::test_selector_observation_logs_locator_metadata_fields`
  - integration: `tests/task_api_behavior.rs::browser_runtime_locator_action_emits_selector_telemetry_fields`
- [done] pilot task migration completed: `src/task/twitterfollow.rs` now uses locator-first strategy (`Follow @...` / `Following @...`) with safe CSS/JS fallback
- phased rollout execution remains:
  - telemetry/error-rate monitoring
  - rollback trigger/action definition before default-on
  - expansion gate for additional tasks


## Goal

Add real accessibility-locator support to the current task framework so tasks can target UI elements by role and accessible name, while keeping the current task API stable.

Target qualities:
- reliable
- scalable
- easy to use

## Verified Baseline (Real Code Evidence)

Current runtime behavior is CSS-selector driven.

| Claim | Evidence | Result |
|---|---|---|
| `TaskContext::click` documents CSS selector input | `src/runtime/task_context.rs` line 4707: `* selector - CSS selector for the element to click` | Confirmed |
| `TaskContext` query APIs delegate raw selector strings | `src/runtime/task_context.rs` lines 5485-5521 (`exists`, `visible`, `wait_for`, `wait_for_visible`) | Confirmed |
| Selector checks use `document.querySelector(...)` directly | `src/utils/navigation.rs` lines 99-115 (`selector_exists`, `selector_is_visible`) | Confirmed |
| No native `getByRole`/`getByLabel`-style task API exists | Codebase search found no runtime API by those names | Confirmed |
| `role=button[name='...']` is not currently parsed | No parser found in `src/runtime/task_context.rs` or `src/utils/navigation.rs` | Confirmed |

Important correction:
- Accessibility-shaped CSS like `button[aria-label='Like']` already works today because it is valid CSS, not because runtime supports a dedicated accessibility locator grammar.

## Problem Statement

We currently mix two concepts:
1. CSS selectors that reference accessibility attributes (works today)
2. True accessibility locators (role/name semantic query model; not implemented yet)

This gap can cause false confidence and runtime breakage if we start using non-CSS locator strings before parser/resolver support exists.

## Desired End State

1. Keep existing task-facing API unchanged (`api.click`, `api.exists`, `api.wait_for`, etc.)
2. Add internal dual resolution:
   - accessibility locator path (new)
   - CSS selector path (existing fallback)
3. Preserve backward compatibility for all current CSS-based tasks

## API Shape (No Task API Churn)

No new task-facing methods are required.

The existing task API should accept:
- CSS selectors (existing behavior)
- accessibility locator strings (new behavior after implementation)

Example (after implementation):

```rust
// Existing CSS selector path
api.click("button[aria-label='Like']").await?;

// New accessibility locator path (requires parser + resolver)
api.click("role=button[name='Save changes']").await?;
```

## Locator Grammar Decision (Must Be Frozen First)

Recommended initial grammar (v1):
- `role=<role>[name='<accessible name>']`
- optional scope: `scope='<css selector>'`
- optional match mode: `match=exact|contains` (default `exact`)

Example:
- `role=button[name='Save changes']`
- `role=link[name='Profile'][match=contains]`
- `role=button[name='Follow'][scope='main']`

If parsing fails, return a clear error and do not silently downgrade malformed locator syntax into CSS.

## Resolution Semantics (v1)

1. Parse selector string
2. If syntax matches locator grammar:
   - resolve by role + accessible name
   - if exactly 1 visible match: success
   - if 0 match: error `locator_not_found`
   - if >1 match: error `locator_ambiguous` (unless scope narrows to one)
3. If syntax does not match locator grammar:
   - treat as CSS selector (current behavior)

## Implementation Options

| Option | Pros | Cons |
|---|---|---|
| Add explicit role-based task helpers (`get_by_role`, etc.) | Clear semantics, simple onboarding | Expands public API surface; contradicts thin TaskContext direction |
| Add internal generic locator resolver behind current API | No task API churn, scalable, aligns with architecture | More plumbing and test surface |
| Keep CSS-only model | Lowest short-term effort | No true accessibility-locator capability; lower long-term robustness |

## Cargo Dependency Reality Check (2026-04-29)

Live checks performed:
- `cargo search accessibility --limit 20`
- `cargo search accesskit --limit 20`
- `cargo search atspi --limit 20`
- `cargo search chromiumoxide --limit 20`
- `cargo search playwright --limit 10`
- `cargo info accesskit`
- `cargo info atspi`

What this means for this codebase:

| Candidate | Pros | Cons |
|---|---|---|
| Keep `chromiumoxide` + CDP Accessibility domain (`QueryAxTree`) | Already in repo; no runtime migration; direct browser AX query path; keeps `api.*` unchanged | We own thin resolver semantics and tests |
| `accesskit` ecosystem | Strong for app/UI accessibility infrastructure | Not a drop-in browser CDP AX-tree query engine for remote Chromium pages |
| `atspi` ecosystem | Mature Linux accessibility protocol tooling | OS accessibility bus focus; not a cross-platform substitute for CDP browser-page AX queries |
| Rust Playwright crates (`playwright`, `playwright-rs`) | Built-in locator semantics possible after migration | Major framework migration risk, API drift, and larger change surface |

Conclusion:
- There is no clear Cargo drop-in package that replaces the needed browser-page AX locator resolution while preserving current `chromiumoxide` architecture and `TaskContext` API.
- Best path remains: thin internal resolver on top of existing CDP Accessibility domain.

## Recommended Approach

Use internal generic locator resolution behind existing `TaskContext` methods, implemented as a thin adapter over Chromium CDP Accessibility APIs.

Approved direction for this codebase:
- Keep `chromiumoxide` (no framework migration)
- Implement thin resolver on top of CDP Accessibility domain
- Keep current `api.*` task-facing surface unchanged

Reason:
- preserves current task code
- centralizes semantics in one resolver
- avoids high-risk automation framework migration
- keeps migration incremental and measurable

## Confidence Map (Recalibrated)

| Change | Confidence | Why |
|---|---:|---|
| Keep existing selector methods unchanged | 100% | Already stable, required for compatibility |
| Add internal locator parser | 95% | Implemented behind feature flag with passing parser tests |
| Add role+name resolver semantics | 90% | Read helpers + action-path resolver are wired with deterministic mapping; html/attr remain explicit `locator_unsupported` by v1 design |
| Keep CSS fallback intact | 95% | Existing path already mature |
| Add role/name test matrix | 96% | Parser matrix + navigation routing/error tests + browser-runtime compatibility/action-path assertions are landed |
| Update `src/task/SELECTOR.md` and docs | 95% | Documentation-only change |
| Migrate high-value tasks incrementally | 80% | Depends on real target DOM stability and telemetry confidence |

## Rollout Plan (Phased)

1. Freeze grammar + ambiguity policy
   - Output: documented grammar and deterministic error semantics
2. Implement parser utility
   - Suggested location: shared selector utility near navigation layer
3. Implement thin CDP Accessibility resolver
   - Use Chromium Accessibility domain queries for role/name matching
   - Keep resolver internal to shared navigation path
4. Integrate resolver into navigation helpers
   - Touch points: `selector_exists`, `selector_is_visible`, `selector_text`, `wait_for_selector`, `wait_for_visible_selector`
5. Keep TaskContext method signatures unchanged
   - `api.click`, `api.exists`, `api.visible`, `api.wait_for` unchanged
6. Add test matrix before migration
7. Migrate a small high-value task subset
8. Measure success/failure telemetry, then widen migration

## Validation Plan (Proof of Effectiveness)

Add automated tests for:
- role+name exact lookup success (`button`, `link`)
- visibility checks on role-based target
- ambiguous matches -> deterministic error
- missing role/name -> deterministic error
- malformed locator syntax -> deterministic parse error
- CSS selectors still work unchanged
- mixed workload (CSS + locator syntax) in same task flow

Success criteria:
- 0 regressions in existing CSS-based tests
- deterministic behavior for ambiguous locator queries
- clear error classification for parser/resolver failures

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Ambiguous accessible names | wrong element action | exact match default + explicit scope support |
| Nested/duplicated roles | unstable resolution | prefer container-scoped lookup; fail on ambiguity |
| Hidden/offscreen matches | flaky behavior | visibility-aware resolution by default for action APIs |
| API drift across modules | inconsistent semantics | centralize parser+resolver in shared layer |
| False confidence from docs | misuse by task authors | explicitly separate CSS-attribute selectors vs semantic locators |

## What To Review First

1. Locator grammar and parse error policy
2. Ambiguous result handling policy
3. Visibility semantics for action vs read APIs
4. Exact integration points in `src/utils/navigation.rs`

## Best Next Moves

1. Approve grammar and ambiguity policy before coding
2. Implement thin CDP Accessibility-backed parser+resolver behind feature flag
3. Land tests first, then migrate selected tasks
4. Update `src/task/SELECTOR.md` to distinguish:
   - CSS selectors using ARIA attributes (current)
   - true locator grammar (new)

## Preferred Direction

Keep `TaskContext` as the only task-facing entry point.
Add accessibility locator capability as internal behavior, not as a separate task API family.
