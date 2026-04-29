# Accessibility Locator Spec (v1)

Status: Draft for approval
Owner: Runtime/Task API maintainers
Depends on: `PROPOSAL_ACCESSIBILITY_LOCATOR.md`

## 1. Purpose

Define the exact runtime contract for accessibility locator support behind existing `TaskContext` selector-based APIs.

This spec is implementation-facing and testable.

## 2. Scope

In scope:
- Parse selector input into either CSS selector or accessibility locator.
- Resolve accessibility locator by role + accessible name (+ optional scope).
- Keep existing `TaskContext` method signatures unchanged.
- Provide deterministic error semantics.

Out of scope (v1):
- New public task-facing methods (`get_by_role`, `get_by_label`, etc.).
- Non-role locator families.
- Heuristic auto-correction of invalid locator strings.

## 3. Existing Baseline (Verified)

Current runtime treats selector inputs as CSS selectors and uses DOM query operations (`document.querySelector(...)`) in shared helpers.

Implication:
- ARIA attribute selectors work as CSS.
- `role=button[name='Save']` grammar is not supported until parser/resolver is added.

## 4. Supported Locator Grammar (v1)

### 4.1 Grammar

```
role=<role>[name='<accessible name>'][scope='<css selector>'][match=exact|contains]
```

### 4.2 Field Rules

- `role` (required)
  - lowercase token, e.g. `button`, `link`, `textbox`, `menuitem`
- `name` (required for v1)
  - single-quoted string
- `scope` (optional)
  - single-quoted CSS selector used to narrow search root
- `match` (optional)
  - `exact` (default)
  - `contains`

### 4.3 Valid Examples

- `role=button[name='Save changes']`
- `role=link[name='Profile'][match=contains]`
- `role=button[name='Follow'][scope='main']`

### 4.4 Invalid Examples

- `role=button` (missing `name`)
- `role=button[name="Save"]` (double-quote name is invalid in v1)
- `role=[name='Save']` (missing role value)
- `role=button[name='Save'][match=prefix]` (unsupported match value)

## 5. Resolution Semantics

1. Parse incoming selector string.
2. If parse succeeds as accessibility locator:
   - Build candidate set by role within scope root (`document` when no scope).
   - Filter by accessible name using `match` mode.
   - For action APIs, require visible target.
3. Result rules:
   - 0 matches -> `locator_not_found`
   - 1 match -> success
   - >1 matches -> `locator_ambiguous`
4. If parse does not match locator grammar:
   - Treat as CSS selector (existing behavior).

## 6. Error Taxonomy

Standardize typed errors/messages so logs and tests are deterministic.

- `locator_parse_error`
  - malformed locator grammar
- `locator_not_found`
  - valid locator syntax, no match
- `locator_ambiguous`
  - valid locator syntax, multiple matches
- `locator_scope_invalid`
  - `scope` CSS is invalid/unresolvable

Policy:
- Do not silently reinterpret malformed locator syntax as CSS.
- Only non-locator strings should enter CSS fallback path.

## 7. API Compatibility Contract

Public task API remains unchanged:
- `api.click(selector)`
- `api.exists(selector)`
- `api.visible(selector)`
- `api.text(selector)`
- `api.wait_for(selector, timeout_ms)`
- `api.wait_for_visible(selector, timeout_ms)`

No new task-facing API required for v1.

## 8. Integration Points (Implementation Plan)

Primary integration layer:
- shared selector/navigation utility path (single source of truth)

Suggested phased touch points:
1. Add parser + locator model module
2. Integrate resolver into shared selector helpers
3. Keep `TaskContext` methods as pass-through caller surface

Do not duplicate parser/resolver logic across task modules.

## 9. Observability Requirements

Emit structured metadata for selector resolution attempts:
- `selector_mode`: `css` | `a11y`
- `locator_role`
- `locator_match_mode`
- `locator_scope_used`
- `locator_result`: `ok` | `not_found` | `ambiguous` | `parse_error`

Goal: measure migration quality and flake sources.

## 10. Test Matrix (Required Before Migration)

### Parser tests
- valid grammar accepted
- invalid grammar rejected with `locator_parse_error`

### Resolver tests
- button by exact name
- link by contains name
- scope-narrowed resolution
- ambiguous match -> `locator_ambiguous`
- no match -> `locator_not_found`

### Compatibility tests
- existing CSS selectors keep behavior
- mixed CSS and locator usage in same flow
- visibility checks on locator targets

## 11. Rollout Gates

Gate 1: Spec approval
- grammar + ambiguity policy approved

Gate 2: Parser + unit tests
- parse and error semantics green

Gate 3: Resolver + compatibility tests
- no regression on existing CSS tests

Gate 4: Targeted migration
- migrate small high-value task subset
- monitor failure taxonomy and pass rates

Gate 5: Broader adoption
- continue migration only when telemetry is stable

## 12. Non-Goals and Constraints

- No broad API expansion in v1
- No hidden fallback from malformed locator syntax
- No task-specific one-off locator semantics

## 13. Open Decisions (Approve Before Code)

1. Should `name` remain required in v1?
2. Should hidden elements ever be allowed for non-action APIs?
3. Should `match=contains` be allowed by default or require explicit opt-in?
4. Should scope failures return hard errors or soft not-found?

## 14. Acceptance Criteria

Spec is accepted when:
- grammar is frozen
- ambiguity policy is frozen
- error taxonomy is frozen
- test matrix is agreed
- rollout gates are agreed
