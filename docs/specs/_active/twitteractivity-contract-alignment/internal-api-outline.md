# Internal Boundary Outline

- `TaskConfig::from_payload`
  - owns payload parsing, defaults, and validation for TwitterActivity
  - should expose explicit runtime fields for duration and scroll budget
- `twitteractivity.rs::run_inner`
  - owns the task execution contract
  - should use `TaskConfig` values only and avoid hidden fallback logic
- `twitteractivity_feed::scroll_feed`
  - owns the lower-level feed scrolling primitive
  - should stay reusable and not absorb contract decisions
- `read_u64` / `read_u32`
  - should reject malformed numeric payloads instead of silently substituting defaults
- docs/config files
  - should mirror runtime behavior, not define it independently
