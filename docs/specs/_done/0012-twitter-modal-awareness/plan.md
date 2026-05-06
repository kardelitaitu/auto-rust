# Plan

## Step 1: Enhance Context Validation

Modify `is_on_tweet_page` in `src/utils/twitter/twitteractivity_interact.rs`.
- Keep existing URL check.
- Add a JS evaluation to check for the presence and visibility of `div[role="dialog"]` or `div[data-testid="tweetDetail"]`.
- Return `true` if either condition is met.

## Step 2: Prioritize Modal Selectors

Modify `js_root_tweet_button_center` in `src/utils/twitter/twitteractivity_selectors.rs`.
- Update the JS logic to look for a modal container (`[role="dialog"]`) first.
- If found, scope the `article[data-testid="tweet"]` search to within that modal.
- If no modal is found, fallback to the existing URL-matching and background-article logic.

## Step 3: Verification

- Run existing tests to ensure no regressions.
- Add integration tests (or manual verification steps) that simulate a modal environment if possible.
- Verify that engagement buttons are correctly identified when a modal is present.

## Rollback

- Revert the changes to the pure URL-based logic and the simple article selection in the JS generator.

# Internal API Outline

## Contract

`is_on_tweet_page` must return `true` if the user is looking at a specific tweet thread, whether by URL or by modal presence.
`js_root_tweet_button_center` must target the "focused" tweet, preferring a modal if one exists.

## Inputs

- `api: &TaskContext`: Used to check URL and evaluate JS in the page.
- `selector: &str`: CSS selector for the button to find (like, retweet, etc.).

## Outputs

- `is_on_tweet_page`: `Result<bool>`
- `js_root_tweet_button_center`: `String` (JavaScript code)

## State Changes

- No persistent state changes, these are read-only checks of the current DOM/URL.

## Error Paths

- `get_current_url` failure.
- `page().evaluate()` failure.

## Invariants

- If a modal is open, the automation must NOT interact with the background feed.

# Decisions

## Decision Log

- **Scope scoping to Modals**: Decided to prioritize anything inside `[role="dialog"]` if it contains an article. This is because Twitter's modal implementation is consistent in using the dialog role for thread overlays.
- **Combined Spec**: Decided to combine context validation and selector priority into one spec because they both address the same root problem (modal invisibility to automation).

## Open Questions

- Should we check for `aria-modal="true"` specifically? (Currently sticking to `role="dialog"` as it's more common).
- How to handle multiple nested dialogs? (Twitter usually only has one main thread modal).

