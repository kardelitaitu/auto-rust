# Baseline

## What I Find

`src/utils/twitter/twitteractivity_state.rs` is the **second-largest file in the codebase** at 1226 lines. It bundles 8 distinct type definitions with their impls in a single file, despite the adjacent `src/utils/twitter/` directory already having 23 submodules demonstrating the submodule pattern.

**Current structure:**

| Struct/Enum | Description |
|---|---|
| `TaskValidationError` (line 21) | Error type for task validation |
| `SentimentTemplates` (line 55) | LLM prompt templates for sentiment analysis |
| `TaskConfig` (line 114) | Task configuration with `from_payload()` constructor |
| `TweetActionTracker` (line 218) | Per-tweet action deduplication |
| `CandidateContext` (line 262) | Decision candidate metadata |
| `CandidateResult` (line 275) | Decision outcome |
| `SessionState` (line 286) | Per-session engagement state + limits |
| `RateLimitBackoff` (line 387) | Exponential backoff for rate limits |

## What I Claim

Extracting these 8 types into `src/utils/twitter/state/` submodules will:
- Make each type independently readable and maintainable
- Reduce `twitteractivity_state.rs` from 1226 to ≤50 lines (re-export shim)
- Follow the established submodule pattern in the codebase (23 existing files under `src/utils/twitter/`)
- Zero behavioral changes — identical test suite passes

## What Is the Proof

**Proof 1 — Monolithic size:** 1226 lines makes this the second-largest file in the project. The only larger file was `orchestrator.rs` (1623 lines, extracted in spec 0016). This is the natural next extraction target.

**Proof 2 — Distinct types with zero coupling between groups:** `TaskConfig` (line 114) has no dependency on `SessionState` (line 286). `TweetActionTracker` (line 218) is self-contained. `SentimentTemplates` (line 55) is a standalone struct. Each can be extracted independently.

**Proof 3 — Successful precedent:** Spec 0014 extracted DSL executor action handlers into 4 submodules. Spec 0016 extracted orchestrator concerns into 6 submodules. Both patterns compiled and tested cleanly with zero behavioral changes. The same mechanical extraction approach applies here.

**Proof 4 — No dead code:** Unlike past extractions that dealt with dead_code methods, this file has zero `#[allow(dead_code)]` annotations. Every type is actively used by the Twitter automation pipeline.

**Proof 5 — Existing submodule pattern:** The `src/utils/twitter/` directory already has 23 files organized into subdirectories (`decision/`, `js/`, `persona_keywords/`, `sentiment/`). Adding a `state/` subdirectory with 4 files follows the established codebase convention.
