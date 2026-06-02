# Bug-Hunting Implementation Plan

*Scope: all of `src/` except `src/task/` and `src/utils/twitter/`.*
*Ordered by ROI — start with the highest-impact, lowest-risk changes.*

---

## Phase 1: Newtypes (prevent bug classes entirely)

Add thin wrapper types where `String`, `f64`, and `u64` are used interchangeably.
Mechanical changes, almost zero behavioral risk.

### 1a. LLM Module (`src/llm/`)

| Current | Newtype | Reason |
|---|---|---|
| `ChatMessage.role: String` | `Role(User\|System\|Assistant)` | Typos like `"systemm"` become compile errors |
| `LlmConfig.temperature: f64` | `Temperature(f64)` | Range `0.0..=2.0`, validated at construction |
| `LlmConfig.max_tokens: Option<u32>` | `MaxTokens(NonZeroU32)` | Zero-tokens = nonsense |

### 1b. Config & Profile (`src/config/`, `src/utils/profile.rs`)

| Current | Newtype | Reason |
|---|---|---|
| `ProfileParam.base: f64` | `ParamValue(f64)` | Rejects NaN, -inf, +inf |
| `ProfileParam.deviation_pct: f64` | `DeviationPct(f64)` | Clamped `0.0..=100.0` |
| `action_delay_min`, `scroll_pause`, etc: `f64` | typed wrappers | Prevents mixing ms/pixels/% |
| `Probability` fields throughout config | `Probability(f64)` | Clamped `0.0..=1.0` |

### 1c. Bacon Pipeline (`src/bacon_core/`)

| Current | Newtype | Reason |
|---|---|---|
| `PipelineConfig.spec_path: String` | `SpecPath(PathBuf)` | Validated absolute path |
| `PipelineConfig.agent_name: String` | `AgentName(String)` | Validated alphanumeric |
| `StageDelay: u64` | `DelayMs(NonZeroU64)` | Zero-delay is always wrong |
| `WorkerOutput.status: String` | `WorkerStatus(Success\|Failure\|Skipped)` | Typo-proof |

### 1d. Session & Circuit Breaker (`src/session/`, `src/api/`)

| Current | Newtype | Reason |
|---|---|---|
| `Session.session_id: String` | `SessionId(String)` | Non-empty validation |
| `CircuitBreaker.failure_count: u32` | `FailureCount(u32)` | Threshold-aware |
| `timeout_secs: u64` | `TimeoutSecs(NonZeroU64)` | Zero timeout = busy loop |

---

## Phase 2: Property-Based Testing (`proptest`)

Find edge cases the type system can't express.

### Priority 1 — LLM text parsers (highest impact)

- **`clean_llm_json_response()`** — inject strings with: unmatched braces, markdown fenced blocks, Unicode, emoji, control chars, deeply nested JSON, extremely long prefixes
- **`parse_batch_response_static()`** — empty arrays, non-array JSON, truncated lines, mixed formats, stray closing brackets
- **`extract_json_object()`** (`bacon_core`) — random byte noise, bracket soup, null bytes

Strategy: feed random-ish strings and assert:
  - Never panics
  - If it returns `Some(json)`, `serde_json::from_str(json)` succeeds
  - If it returns `None`, the input was genuinely malformed

### Priority 2 — Numeric invariants

- **`with_profile_variance()`** — property: output weights are always in `[0.0, 1.0]`
- **Temperature in prompts** — property: formatted string always shows `0.0..=2.0`
- **Circuit breaker `is_circuit_breaker_open_pure()`** — property: never panics on any `usize` value (overflow, wraparound, `usize::MAX`)
- **Delay calculations** — property: computed delay is always `>= SOME_MIN_MS`

### Priority 3 — State transitions

- **Circuit breaker** — property: valid transition graph (`Closed→Open→HalfOpen→Closed`), no illegal transitions
- **Learning engine `adaptation_for()`** — property: outputs stay within defined hardware limits (speed, delay, verification strictness)

---

## Phase 3: Fuzzing (`cargo fuzz`)

For code paths that consume untrusted input (LLM output, config files, API responses).

### Target 1: LLM JSON Response Parser

The path: NVIDIA/Ollama/OpenRouter returns raw text → `clean_llm_json_response()` → `serde_json::from_str`

Fuzz with: random bytes, valid JSON mixed with markdown, HTML, binary data,
JSON that is valid but structurally wrong (string where array expected).

### Target 2: Coder SEARCH/REPLACE Block Parser

`src/bacon_agent_nvidia/coder.rs` — hand-written parser for:

```
SEARCH:
```
...
```
REPLACE:
```
...
```
```

Fuzz with: missing markers, reversed order, multiple SEARCH blocks, very long lines,
empty replace blocks, trailing whitespace variations.

### Target 3: Config TOML + Env Var Override Parser

`src/config/` — `toml::from_str` + manual env var override logic.

Fuzz with: malformed TOML (skip sections, duplicate keys, wrong types),
invalid env var values (negative numbers, overflow, typos in boolean strings).

---

## Phase 4: Mutation Testing (`cargo mutants`)

Verifies existing tests actually catch bugs.

### Baseline targets

| Module | Lines | Mutants expected | Risk |
|---|---|---|---|
| `src/llm/unified_processor.rs` | ~340 | ~40 | Medium (parsing logic) |
| `src/session/mod.rs` (circuit breaker) | ~50 | ~15 | Low (well-tested) |
| `src/bacon_core/mod.rs` (extract_confidence) | ~30 | ~8 | Low |
| `src/api/mod.rs` (circuit breaker) | ~40 | ~12 | Low |
| `src/config/mod.rs` (env var parsing) | ~60 | ~20 | Medium |

Threshold: < 15% surviving mutants.

### Interpreting results

- If mutants survive in comparison operators (`>=` → `>`): add boundary-value assertions
- If mutants survive in boolean logic (`&&` → `||`): add property-based tests
- If mutants survive in arithmetic (`+` → `-`): add roundtrip/invariant tests

---

## Phase 5: State Machines via Types

Encode the state machines that are currently implicit booleans + runtime checks.

### Pipeline stage progression

Current: `fn next_stage(stage: &Stage) -> Stage` + discriminant comparison

Replace with:

```rust
struct ObserverStage;
struct StrategistStage(ObserverStage);
struct CoderStage(StrategistStage);
struct AuditorStage(CoderStage);

trait PipelineStage { /* shared interface */ }
impl PipelineStage for ObserverStage { /* ... */ }
```

Then `fn execute<P: PipelineStage>(self, stage: P) -> Result<()>` guarantees the sequence at compile time.

### Circuit breaker

Current: `is_circuit_breaker_open_pure(failure_count, last_failure, now, threshold, timeout)`

Replace with:

```rust
enum CircuitState { Closed, Open(Instant), HalfOpen }
```

Illegal transitions (`Open(HalfOpen) → Open` while timer hasn't expired) become match exhaustiveness.

### Pipeline outcome booleans

Current: `scope_reduction_needed: bool, coder_refused: bool, needs_human_approval: bool` (3 independent flags, 8 states, many invalid)

Replace with:

```rust
enum PipelineOutcome {
    Passed,
    CoderRefused,
    NeedsHumanApproval,
    ScopeReductionRequired,
}
```

Invalid flag combinations (`coder_refused + needs_human_approval = true` both) become impossible.

---

## Phase 6: Dynamic Analysis (`cargo miri`)

Lowest priority — confirmed zero `unsafe` blocks in scope. Run monthly as regression check.

- `cargo miri test` on the full non-task test suite
- Any failure is a transitive dependency issue (unsafe in `reqwest`, `chromiumoxide`, `enigo`)
- Log and pin affected dependency versions

---

## Execution Order

```
Phase 1: Newtypes         — high impact, mechanical, ~2 hours
Phase 2: Proptest         — moderate impact, finds real bugs, ~4 hours
Phase 3: Fuzzing           — high impact on LLM paths, ~3 hours
Phase 4: Mutation testing  — validates Phase 1-3, ~1 hour
Phase 5: State machines    — larger refactor, ~4 hours
Phase 6: Miri              — ongoing, 5 min per run
```

Each phase is independent — any phase can be done in isolation.
Start with Phase 1 (newtypes) — it's mechanical, safe, and prevents bugs permanently.
