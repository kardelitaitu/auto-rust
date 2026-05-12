## Validation

- All 17 dead code items removed or annotated with explicit reason.
- HOME_LOGO_SELECTOR uses correct unescaped quotes or removed.
- simulation.rs delegates to select_persona_weights() instead of duplicating.
- Llm::new() called once per session instead of per reply/quote.
- Regex compiled via static Lazy.
- All existing tests pass; no compilation warnings about unused items.
- `spec-lint.ps1`, `./check-fast.ps1`, and `./check.ps1` pass.
