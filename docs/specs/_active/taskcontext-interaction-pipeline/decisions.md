# Decisions

- Keep the public TaskContext API stable and move only internal orchestration behind the pipeline.
- Use shared interaction types to carry action-specific verification and fallback data.
- Keep click and type as separate verbs, but let them share the same preflight and postflight stages.
- Prefer deterministic tests over timing-heavy interaction checks.
