# Plan

## What Is the Solution

1. **Modularize the directory**: Create a new directory `src/utils/twitter/engagement/`.
2. **Component Breakdown**:
   - `engagement/pipeline.rs`: The new home for a much leaner `process_candidate` function that acts purely as an orchestrator.
   - `engagement/actions.rs`: Functions to dispatch and retry specific actions (like, retweet, quote, follow).
   - `engagement/helpers.rs`: Shared pure functions like `extract_tweet_text` or `calc_rate`.
3. **Refactor `process_candidate`**: Extract the deep action execution loops and the newly added thread reply scanning loops into dedicated helper functions within `engagement/actions.rs`.
4. **Testing**: Extract the extensive integration and statistical tests currently at the bottom of the file into the `tests/` directory.