## Internal API Outline

### `src/task/twitteractivity.rs`

- `run(api, payload, config)`
  - parses `TaskConfig`
  - enforces timeout
  - forwards into `run_inner()`
- `run_inner(api, payload, config, task_config)`
  - builds persona weights
  - initializes session limits and state
  - runs phase 1 navigation
  - loops feed scan and candidate processing
  - logs the final summary
- `log_summary(session, task_config, api)`
  - emits summary lines only
- `build_summary_lines(session, task_config)`
  - returns summary and remaining-limit strings

### Dependencies

- `twitteractivity_navigation::phase1_navigation`
- `twitteractivity_feed::identify_engagement_candidates`
- `twitteractivity_engagement::process_candidate`
- `twitteractivity_limits::EngagementLimits`
- `twitteractivity_persona::{select_persona_weights, apply_behavior_profile}`
- `twitteractivity_state::{TaskConfig, SessionState, CandidateContext, CandidateResult}`
