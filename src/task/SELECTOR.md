# Selector Standard

This file defines the selector approach for task code.

## Goal

- Reliable
- Scalable
- Easy to use

## Main Rule

Use only two selector types:

1. Accessibility locators
2. DOM selectors

No third selector style should be added in normal task code.

## Selector Order

1. Accessibility locator
2. DOM selector

## 1. Accessibility Locators

Primary choice. Use when the UI exposes a real role or label.

Examples:

```rust
api.click("button[aria-label='Like']")
api.click("[role='button'][aria-label='Follow @user']")
```

Why this is good:

- Closer to how a user or assistive tech sees the page
- Usually more stable than raw CSS
- Easier to read in code

## 2. DOM Selectors

Use when accessibility locators are not available or not reliable.

Examples:

```rust
api.click("[data-testid='tweetButton']")
api.click("[data-testid='like']")
```

Why this is good:

- Fast
- Direct
- Simple to debug

Why this can fail:

- A site rename or markup change can break the selector
- The selector may match the wrong node if structure shifts

## Task Author Rules

- Prefer accessibility locators first.
- Use DOM selectors only when needed.
- Keep selectors scoped to the target container when possible.
- Do not use page-wide scans if a single element can be targeted.
- Verify the same element you clicked, if possible.

## Twitter-Specific Guidance

- Use accessibility locators when X exposes a good label or role.
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

## Best Next Move

- Use this standard when adding new task selectors.
- If a selector is repeated across tasks, move it into a shared utility.
