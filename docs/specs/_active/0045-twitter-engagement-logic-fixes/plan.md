# Plan

## Step 1: Fix Dry-Run Safety Bypass

- In `src/utils/twitter/twitteractivity_engagement.rs`, locate the `engage_replies` function.
- Add a check at the beginning of the reply iteration loop: `if task_config.dry_run_actions { info!("Dry-run: would like reply..."); continue; }`.

## Step 2: Fix Action Starvation

- In `process_candidate`, locate the `select_candidate_action` call.
- Currently, it selects exactly ONE action. Remove `select_candidate_action` entirely.
- Determine `need_dive` by checking if `actions_to_do` contains *any* action other than "like".
- If `need_dive` is true, perform the thread dive once.
- Iterate over the entire `actions_to_do` array and execute each one sequentially.

## Step 3: Remove Dead Code

- In the execution loop (where `action == "like"`), remove the `if did_dive` check.
- Since we are now allowing multi-actions, if a dive occurred (because we also wanted to reply), the "like" action should be executed using the generic `like_tweet(api)` method rather than the positional one, as the UI layout changes when in the thread view. Therefore, the logic should be: `if did_dive { like_tweet(api) } else { like_at_position(...) }`. (Note: The previous review stated this was dead code because the bot *only* did one action. With multi-actions, `did_dive` CAN be true when executing a like, so the branch is no longer dead, but its condition is now correct and required).

## Step 4: Verification

- Run `cargo clippy` and `cargo test`.
- Run the integration tests. Ensure limits are properly decremented for multiple actions per tweet.

# Internal API Outline

- Modify `process_candidate` to loop over `actions_to_do`.
- Ensure `engage_replies` respects `task_config.dry_run_actions`.

# Decisions

- Multi-action execution: A human realistically likes and replies to the same tweet. The bot should do the same. This will consume action budgets faster but results in a much higher quality automation footprint.
