# Plan

1. Define the shared candidate plan types and the planner input contract.
2. Extract only pure candidate decisions from `process_candidate()` into the planner.
3. Keep browser actions, retries, and pauses inside a small executor.
4. Reuse the same planner from `twitteractivity_simulation.rs`.
5. Add deterministic tests for planner output, live wiring, and simulation reuse.
