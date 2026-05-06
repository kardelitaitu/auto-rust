# Twitter Decision Engine Consolidation

Status: `implementing`

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
