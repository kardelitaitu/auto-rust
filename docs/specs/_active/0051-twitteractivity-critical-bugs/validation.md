## Validation

- LLM API key is threaded from config through to DecisionEngineFactory::create() so LLM-powered decisions actually use LLM.
- extract_tweet_context() JS correctly extracts each reply's own author.
- Popup dismissal runs BEFORE login verification in phase1_navigation().
- All existing tests pass; no behavioral regressions.
- `spec-lint.ps1`, `./check-fast.ps1`, and `./check.ps1` pass.
