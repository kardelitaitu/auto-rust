# Twitter Decision Engine Consolidation

Status: `done`

Owner: `spec-agent`
Implementer: `cascade`

## Summary

The Twitter activity module currently maintains 5 separate decision engines (`decision`, `decision_hybrid`, `decision_llm`, `decision_persona`, `decision_unified`) with overlapping functionality and duplicated code (~2,100 lines total). This creates maintenance burden, confusing API surface, and inconsistent behavior. This initiative consolidates all engines into a single unified system with pluggable strategies, reducing code duplication while preserving all existing capabilities.

## Scope

- **In scope**:
  - Analyzing all 5 decision engines to extract unique capabilities
  - Designing unified `DecisionEngine` with strategy plugin system
  - Migrating legacy decision logic as a strategy plugin
  - Migrating hybrid decision logic as a strategy plugin  
  - Migrating LLM decision logic as a strategy plugin
  - Migrating persona decision logic as a strategy plugin
  - Updating `mod.rs` to expose clean unified API
  - Comprehensive testing to ensure no behavioral regression
  
- **Out of scope**:
  - Modifying actual decision algorithms (preserve existing logic)
  - Changes to `twitteractivity_engagement.rs` (consumer of decision engines)
  - Adding new decision capabilities beyond existing ones
  - Changes to sentiment analysis modules

## Files

- `spec.yaml`
- `baseline.md`
- `internal-api-outline.md`
- `plan.md`
- `validation-checklist.md`
- `ci-commands.md`
- `decisions.md`
- `quality-rules.md`
- `implementation-notes.md`

## Rules

- Preserve exact decision logic - no behavioral changes
- Plugin system should be simple (enum-based strategies, not trait objects)
- Maintain backward compatibility for config-driven strategy selection
- All existing tests must pass without modification
- Delete individual engine files after migration complete
- Run `spec-lint.ps1` before handoff

## Next Step

Phase 1: Create baseline analysis of all 5 decision engines (IN PROGRESS)

# Baseline

## Current State (Phase 1 Analysis Complete)

### Decision Engine Files

| File | Lines | Functions | Public API | Purpose | Key Features |
|------|-------|-----------|------------|---------|--------------|
| `twitteractivity_decision.rs` | 690 | 32 fn, 2 pub | `DecisionEngine` trait, `EngagementDecision`, `TweetContext`, `DecisionEngineFactory` | Legacy rule-based engine + shared types | Keyword matching, blocklists, quality scoring |
| `twitteractivity_decision_hybrid.rs` | 260 | 11 fn, 3 pub | `HybridEngine` | Combines multiple strategies | Strategy combination with weights |
| `twitteractivity_decision_llm.rs` | 405 | 17 fn, 2 pub | `LLMEngine`, `LLMEngineBuilder` | LLM-powered decisions | OpenAI API integration |
| `twitteractivity_decision_persona.rs` | 278 | 7 fn, 1 pub | `PersonaEngine` | Persona-weighted decisions | Persona weight application |
| `twitteractivity_decision_unified.rs` | 468 | 24 fn, 11 pub | `UnifiedEngine`, `UnifiedEngineBuilder` | Most comprehensive engine | Single LLM call for decision + content |

**Total: ~2,101 lines across 5 files**

### All Engines Implement DecisionEngine Trait

```rust
#[async_trait]
pub trait DecisionEngine: Send + Sync {
    fn name(&self) -> &'static str;
    async fn decide(&self, ctx: &TweetContext) -> EngagementDecision;
    fn is_available(&self) -> bool { true }
}
```

### Shared Types (defined in decision.rs)

- `TweetContext` - Input context for decisions
- `EngagementDecision` - Output with level, score, reason
- `EngagementLevel` - Full/Medium/Minimal/None
- `DecisionStrategy` - Enum for strategy selection
- `DecisionEngineFactory` - Factory for creating engines

### Unique Capabilities per Engine

| Engine | Unique Capability |
|--------|-------------------|
| `decision.rs` (legacy) | Keyword blocklists (controversial, spam, negative), quality scoring algorithm |
| `decision_persona.rs` | PersonaWeights application to decisions |
| `decision_llm.rs` | Pure LLM-based decision with OpenAI API |
| `decision_hybrid.rs` | Weighted combination of multiple strategies |
| `decision_unified.rs` | Single LLM call returns both decision AND generated content |

### Duplication Analysis

1. **All engines implement same trait** - 5 identical `impl DecisionEngine` blocks
2. **Safety checks duplicated** - Each engine has tragedy/crypto scam detection
3. **Persona integration similar** - Multiple engines apply PersonaWeights
4. **EngagementLevel mapping** - Same score→level logic in multiple places

## Target State

### Unified Structure

```
src/utils/twitter/
  decision/
    mod.rs              # Re-export unified API
    types.rs            # Shared types (TweetContext, EngagementDecision, etc.)
    engine.rs           # UnifiedEngine with strategy dispatch
    strategies/         # Strategy implementations
      mod.rs            # Strategy exports
      legacy.rs         # Rule-based (from decision.rs)
      hybrid.rs         # Strategy combination (from hybrid.rs)
      llm.rs            # LLM-based (from llm.rs)
      persona.rs        # Persona-based (from persona.rs)
      unified.rs        # Unified LLM (from unified.rs)
```

### Reduced API Surface

```rust
// New clean API from decision/mod.rs
pub use types::{
    TweetContext,
    EngagementDecision,
    EngagementLevel,
    DecisionStrategy,
};
pub use engine::{DecisionEngine, UnifiedEngine};
```

### Size Reduction

- Before: ~2,100 lines across 5 files
- After: ~1,200 lines in unified structure
- Reduction: ~43% code reduction

