# Task Migration Guide: Accessibility Locator

## Purpose

This guide explains how to migrate task code from CSS-first selectors to accessibility locator-first selectors using the current runtime behavior.

Goals:
- reliable
- scalable
- easy to use

Companion references:
- `docs/ACCESSIBILITY_LOCATOR_SPEC.md`
- `PROPOSAL_ACCESSIBILITY_LOCATOR.md`
- `src/task/SELECTOR.md`

## Scope and Constraints

In scope:
- Task-level selector migration using existing `TaskContext` APIs (`api.click`, `api.visible`, `api.wait_for`, etc.)
- Accessibility locator grammar (`role=...`) with CSS fallback
- Deterministic error handling and telemetry alignment

Out of scope:
- New task-facing APIs (`get_by_role`, etc.)
- Changing `TaskContext` method signatures
- Silent fallback from malformed locator grammar

## Runtime Contract (Must Follow)

### 1) Locator Grammar

Supported v1 grammar:

```text
role=<role>[name='<accessible name>'][scope='<css selector>'][match=exact|contains]
```

Rules:
- `role` is required
- `name` is required (v1)
- single quotes only for `name` and `scope`
- `match` defaults to `exact` if omitted

### 2) Selector Resolution Order

For migrated tasks:
1. Accessibility locator candidates (preferred)
2. Stable CSS fallback candidates
3. Legacy JS fallback (only where still needed)

### 3) Deterministic Errors

Handle these as first-class outcomes:
- `locator_parse_error`
- `locator_not_found`
- `locator_ambiguous`
- `locator_scope_invalid`
- `locator_unsupported`

Policy:
- Never reinterpret malformed locator strings as CSS.
- Only non-locator strings should flow through CSS path.

## Migration Workflow (Task Author Checklist)

### Step 0: Baseline
- Run `cargo check`
- Run task-specific tests before edits

### Step 1: Inventory selectors in the task
- List all action selectors (`click`, `hover`, `drag`, `focus`)
- List all read selectors (`exists`, `visible`, `text`, `value`)
- Mark each selector as:
  - semantic-ready (role + accessible name exists)
  - CSS-only (no stable semantic label)

### Step 2: Build locator candidates per action
For each critical action, define ordered candidates:
- strict exact locator scoped to target container
- contains-based locator fallback
- stable CSS fallback

Example:
```rust
let candidates = [
    "role=button[name='Follow @target'][scope='main header']",
    "role=button[name='Follow @'][match=contains][scope='main header']",
    "role=button[name='Follow'][scope='main header']",
    "button[data-testid$='-follow']",
];
```

### Step 3: Implement locator-first action flow
Use `TaskContext` APIs only.

Pattern:
```rust
for selector in candidates {
    if api.visible(selector).await.unwrap_or(false) {
        if api.click(selector).await.is_ok() {
            // success
            break;
        }
    }
}
```

### Step 4: Implement deterministic verification
Verify post-action state with semantic-first checks.

Pattern:
```rust
let verified = api
    .visible("role=button[name='Following @'][match=contains]")
    .await
    .unwrap_or(false)
    || api.visible("button[data-testid$='-unfollow']").await.unwrap_or(false);
```

### Step 5: Keep fallback, but local and minimal
- Keep CSS fallback close to the primary action
- Avoid broad page scans unless unavoidable
- Avoid duplicating selector logic across task files

### Step 6: Add tests
At minimum:
- selector candidate builder tests
- behavior tests for successful candidate selection order
- compatibility tests proving fallback still works

### Step 7: Verify and document
- Run feature-on checks:
  - `cargo check --features accessibility-locator`
  - `cargo test --features accessibility-locator <task-or-suite>`
- Update proposal/spec status if this is a rollout milestone

## Recommended Patterns

### Pattern A: Candidate builders (preferred)

Create small helper builders for repeated semantic selectors.

```rust
fn follow_locator_candidates(username: &str) -> Vec<String> {
    vec![
        format!("role=button[name='Follow @{}'][scope='main header']", username),
        "role=button[name='Follow @'][match=contains][scope='main header']".to_string(),
        "role=button[name='Follow'][scope='main header']".to_string(),
        "button[data-testid$='-follow']".to_string(),
    ]
}
```

### Pattern B: Scoped first, global second

Always prefer scoped locators before unscoped ones to reduce ambiguity.

### Pattern C: Verify the same intent

If you clicked Follow, verify Following/Unfollow state using semantic + CSS fallback.

## Anti-Patterns (Do Not Introduce)

- Adding a third selector style outside locator + CSS
- Silent downgrade of malformed locator syntax into CSS
- Reintroducing broad `document.querySelectorAll('button')` scans in task code when locator candidates can do the job
- Splitting follow-up verification from action context in a way that loses determinism
- Task-local parser logic (parser belongs to shared resolver path)

## Telemetry and Observability Expectations

Shared navigation emits selector telemetry fields:
- `selector_mode`
- `locator_role`
- `locator_result`
- `locator_match_mode`
- `locator_scope_used`

Task migration should preserve semantic path usage so these fields are meaningful for rollout monitoring.

Recommended rollout metric checks:
- drop in ambiguous/not_found rates for migrated actions
- stable success rate vs CSS baseline
- no spike in `locator_parse_error`

## Rollout and Rollback Gates

### Rollout gates
1. One pilot task migrated and validated
2. Telemetry/error-rate monitoring window completed
3. Expansion only after stability threshold is met

### Rollback trigger examples
- sustained increase in follow/action failure rate above baseline
- sustained increase in `locator_ambiguous` or `locator_not_found` for migrated actions
- browser/runtime incompatibility in target environment

### Rollback action
- revert task-local locator candidate usage to prior CSS path
- keep shared resolver untouched
- keep telemetry assertions and compatibility tests

## Concrete Example: twitterfollow Pilot

Implemented pilot characteristics:
- locator-first for not-followed and already-following states
- confirmed live patterns:
  - `Follow @...` + `...-follow`
  - `Following @...` + `...-unfollow`
- retained CSS/JS fallback for resilience

Use this pilot as template for next tasks.

## Definition of Done (Per Task)

A task migration is done when all are true:
- locator-first candidates implemented for critical actions
- fallback remains safe and minimal
- deterministic verification included
- task tests pass with `--features accessibility-locator`
- no API surface change required
- rollout notes updated in proposal/spec when applicable
