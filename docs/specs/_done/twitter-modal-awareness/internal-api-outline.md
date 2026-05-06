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
