# Proposal: Accessibility Locator Support (Evidence-Based Revision)

Companion implementation spec: `docs/ACCESSIBILITY_LOCATOR_SPEC.md`

## Implementation Safety Checklist (Major Change Gate)

- [ ] Approve locator grammar, ambiguity policy, and error taxonomy in `docs/ACCESSIBILITY_LOCATOR_SPEC.md`.
- [ ] Confirm non-goals (no task API expansion, no hidden fallback from malformed locator syntax).
- [ ] Baseline current behavior before coding:
  - [ ] run `cargo check`
  - [ ] run `cargo test`
  - [ ] record current pass/fail totals and flaky tests (if any)
- [ ] Add implementation behind a feature flag (default off for first landing).
- [ ] Implement parser first, with full unit coverage, before resolver wiring.
- [ ] Implement thin CDP Accessibility resolver in shared navigation path only (single source of truth).
- [ ] Keep `TaskContext` task-facing `api.*` signatures unchanged.
- [ ] Add deterministic error mapping (`locator_parse_error`, `locator_not_found`, `locator_ambiguous`, `locator_scope_invalid`).
- [ ] Add compatibility tests proving CSS selector behavior is unchanged.
- [ ] Add ambiguity and missing-target tests proving deterministic failure semantics.
- [ ] Add observability fields (`selector_mode`, `locator_role`, `locator_result`) and validate log output.
- [ ] Run full verification suite again:
  - [ ] run `cargo check`
  - [ ] run `cargo test`
- [ ] Roll out in phases:
  - [ ] migrate one high-value task
  - [ ] monitor telemetry/error rates
  - [ ] expand migration only when stable
- [ ] Define rollback trigger and rollback action before enabling feature flag by default.
- [ ] Update `src/task/SELECTOR.md` and cross-reference this proposal + spec.


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
| Add internal locator parser | 80% | Contained implementation, clear entry points |
| Add role+name resolver semantics | 65% | Ambiguity/visibility semantics require careful design |
| Keep CSS fallback intact | 95% | Existing path already mature |
| Add role/name test matrix | 90% | Straightforward once grammar is fixed |
| Update `src/task/SELECTOR.md` and docs | 95% | Documentation-only change |
| Migrate high-value tasks incrementally | 78% | Depends on real target DOM stability |

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
