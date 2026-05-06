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
