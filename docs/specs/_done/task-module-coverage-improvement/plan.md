# Plan

## Step 1: Baseline and inventory

- Re-read the TODO coverage section and confirm the three target modules.
- Inspect the current helper functions in each module and list the exact branch points to cover.
- Keep the scope limited to helper coverage unless a behavior path truly needs a smoke test.

## Step 2: Pure unit coverage

- Add deterministic tests for `twitteractivity.rs` payload/config helpers and entry-point selection.
- Add deterministic tests for `twitterfollow.rs` candidate ordering and URL or username extraction.
- Add deterministic tests for `twitterintent.rs` intent parsing and payload URL extraction.

## Step 3: Narrow regression checks

- Add a browser-backed or harness-backed smoke test only if a branch cannot be proven with unit tests.
- Keep any smoke test small and tied to one concrete behavior path.

## Step 4: Verification

- Run `.
check-fast.ps1` during iteration.
- Run `.
check.ps1` before handoff.
- Keep the implementation notes focused on what changed and what coverage gap was closed.
