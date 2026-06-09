# Baseline

## What I Find

`src/config/mod.rs` is the **largest file in the codebase** at 3631 lines — 21% larger than the previous largest file (`orchestrator.rs` at 1623 before spec 0016 extraction). It bundles 13 struct definitions, 2 enums, all Default implementations, environment override logic, and ~2400 lines of inline tests.

**Current contents:**

| Section | Lines (est.) | Description |
|---|---|---|
| Imports + enums | 1–120 | NativeClickCalibrationMode, NativeInputBackend |
| Struct definitions | 19–530 | Config, BrowserConfig, NativeInteractionConfig, CircuitBreakerConfig, BrowserProfile, RoxybrowserConfig, OrchestratorConfig, TwitterActivityConfig, TwitterProbabilitiesConfig, TwitterLLMConfig, TracingConfig, TaskDiscoveryConfig, EngagementLimitsConfig |
| Default impls + helpers | 160–530 | Default blocks + default_*() functions for all structs |
| Config load + env overrides | 2691–3092 | load_config(), load_dotenv_defaults(), load_code_config(), apply_env_overrides() |
| ConfigValidationReport | 3093–3631 | Validation report struct + impl methods |
| Tests | 624–2690 | ~2066 lines of config parsing/validation tests |

**Code smells:**
- 6 `#[allow(dead_code)]` annotations
- Mixed concerns: type definitions, defaults, env parsing, and tests in one file
- BrowserConfig references CircuitBreakerConfig and NativeInteractionConfig inline

## What I Claim

Extracting struct definitions into `types.rs`, Default impls into `defaults.rs`, and env override logic into `env.rs` will:
- Reduce `config/mod.rs` from 3631 to ≤200 non-test lines (re-exports + load_config + ConfigValidationReport)
- Make each config struct independently readable and maintainable
- Follow the established submodule pattern (3 prior extractions: DSL executor, orchestrator, twitter state)
- Zero behavioral changes — identical test suite passes

## What Is the Proof

**Proof 1 — Monolithic size:** 3631 lines makes this the largest file in the project. Every prior extraction (orchestrator 1623→submodules, twitter state 1226→submodules) produced immediate readability and maintainability gains. This is the natural next target.

**Proof 2 — Clean structural boundaries:** The config file has 3 clear concerns: type definitions (structs/enums), default implementations (impl Default + helper fns), and env override logic. These map directly to `types.rs`, `defaults.rs`, and `env.rs`.

**Proof 3 — Successful precedent pattern:** Three prior extractions (specs 0014, 0016, 0017) followed the same mechanical approach: create submodules, move types verbatim, wire re-exports, verify tests pass. All three compiled and tested cleanly with zero behavioral changes.

**Proof 4 — High dead_code count:** 6 `#[allow(dead_code)]` annotations — more than most files. Extraction makes it easier to identify and clean up genuinely unused items by isolating them in smaller files.

**Proof 5 — No existing spec coverage:** Zero of the 13 completed specs (0005–0017) target config/mod.rs. This proposal fills a clear gap.
