# Selector Rules

Use this guide when you edit task selectors, selector helpers, or task author docs.

## Goal

- Reliable
- Scalable
- Easy to use

## Main Rule

Use only two selector styles in normal task code:

1. Accessibility locators
2. DOM selectors

Do not introduce a third selector style unless the runtime contract changes.

## Selection Order

1. Accessibility locator
2. DOM selector

## Accessibility Locators

Use accessibility locators first when the UI exposes a real role and accessible name.

Examples:

```rust
api.click("role=button[name='Like']")
api.click("role=button[name='Follow @user'][match=contains]")
```

Why:

- Closer to the user model
- Usually more stable than raw CSS
- Easier to read in code

Note:

- `button[aria-label='Like']` is CSS, not semantic locator grammar.
- `role=...` syntax is resolved through accessibility logic when the feature is enabled.

## DOM Selectors

Use DOM selectors when accessibility locators are missing or weak.

Examples:

```rust
api.click("[data-testid='tweetButton']")
api.click("[data-testid='like']")
```

Why:

- Fast
- Direct
- Simple to debug

Risk:

- Markup changes can break the selector
- Broad selectors can match the wrong node

## Task Author Rules

- Prefer accessibility locators first.
- Use DOM selectors only when needed.
- Keep selectors scoped to the target container when possible.
- Avoid page-wide scans if a single element is enough.
- Verify the same element you clicked when possible.

## Twitter-Specific Rules

- Prefer accessibility locators when X exposes a good label or role.
- Use `data-testid` only when the accessibility locator is weak or missing.
- Avoid broad `document.querySelectorAll('button')` scans unless needed.
- Keep retries small and local.

## Good Pattern

1. Try accessibility locator.
2. Fall back to a stable DOM selector.
3. Verify the action with a second read.

## Bad Pattern

- Hardcoding many page-wide selectors
- Repeating the same selector logic in many task files
- Depending on exact markup shape when a label is available
- Introducing a third selector style

## Related Docs

- [docs/TASKS/overview.md](overview.md)
- [src/task/SELECTOR.md](../../src/task/SELECTOR.md)

