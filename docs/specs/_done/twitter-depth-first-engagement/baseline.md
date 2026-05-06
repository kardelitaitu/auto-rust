# Baseline

## Current Behavior

1.  **Strict 1:1 Engagement**: For every thread "dive", exactly one root tweet is engaged with.
2.  **Immediate Retreat**: Even if a thread has hundreds of high-quality replies, the bot navigates back to the "Home" feed immediately after the root action.
3.  **Navigation Overhead**: Transitioning from Home -> Status -> Home for every single action consumes significant time and increases the footprint of automation.

## Why It Needs Work

- **Efficiency Gap**: We are wasting the "cost" of the page load. Once we are in a thread, the most efficient thing is to engage with 2-3 high-quality items rather than just one.
- **Realism Gap**: Real users don't exclusively engage with the first tweet of every thread they open. They scroll down, read some comments, and like the ones they agree with.
- **Social Impact Gap**: Engaging with replies helps build community presence beyond just the main "influencer" tweets.

## Relevant Files

- `src/utils/twitter/twitteractivity_engagement.rs`
- `src/utils/twitter/twitteractivity_selectors.rs`

## Known Failure Modes

- Bottlenecked by navigation speed: the bot spends ~40% of its time just waiting for the home feed or status pages to load.
- Repetitive navigation patterns (Home -> Status -> Home) are easier for bot detection systems to fingerprint.

## Evidence

- Analysis of `process_candidate` shows an unconditional `goto_home(api)` call immediately after the root action.
- Log analysis shows "Dive Success" followed immediately by "Navigating back to home".
