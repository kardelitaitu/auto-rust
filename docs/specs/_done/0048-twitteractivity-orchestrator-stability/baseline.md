## Baseline

- `src/task/twitteractivity.rs` is already a thin orchestrator over `src/utils/twitter/*`.
- The file currently owns timeout wrapping, session setup, scan loop control, and summary logging.
- Existing unit tests cover payload parsing, entry-point selection, and summary formatting.
- `tests/twitteractivity_integration.rs` already links the task module with persona, sentiment, tracker, and limit helpers.
- A filtered `cargo test --quiet twitteractivity_integration` run passed during baseline capture.
- The remaining risk is contract drift between the task shell and helper modules, not missing core logic in the task file.
