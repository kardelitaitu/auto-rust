# Implementation Notes

## Progress Tracking

### Current Status
- Created: 2026-05-06
- Status: **Phase 1 - Analysis Complete**
- Implementer: cascade

### Phase 1 Complete ✅
- [x] Analyzed all 5 decision engine files
- [x] Mapped shared types and traits
- [x] Identified unique capabilities per engine
- [x] Documented duplication patterns
- [x] Defined consolidation strategy

### Pending Phases
- [x] Phase 2: Create unified foundation (types.rs, engine.rs) - COMPLETE
- [ ] Phase 3: Migrate legacy strategy
- [ ] Phase 4: Migrate persona strategy
- [ ] Phase 5: Migrate LLM strategy
- [ ] Phase 6: Migrate hybrid strategy
- [ ] Phase 7: Migrate unified strategy
- [ ] Phase 8: Integration and cleanup
- [ ] Phase 9: Validation

## Phase 1 Analysis Results

### File Statistics
| File | Lines | Trait Impl | Key Feature |
|------|-------|------------|-------------|
| decision.rs | 690 | ✅ | Keyword matching, blocklists |
| decision_persona.rs | 278 | ✅ | Persona weight application |
| decision_llm.rs | 405 | ✅ | OpenAI API integration |
| decision_hybrid.rs | 260 | ✅ | Strategy combination |
| decision_unified.rs | 468 | ✅ | Single-call decision+content |

### Consolidation Strategy

1. **Create `decision/types.rs`** - Extract all shared types
2. **Create `decision/engine.rs`** - UnifiedEngine with strategy dispatch
3. **Create `decision/strategies/`** - One file per strategy
4. **Preserve exact logic** - Copy-paste initially, no refactoring
5. **Delete old files** - After migration complete

### Risk Areas Identified

1. **Safety check duplication** - All engines have similar safety checks
   - Mitigation: Extract to shared `safety.rs` module

2. **EngagementLevel scoring** - Slight variations in score thresholds
   - Mitigation: Document exact thresholds, preserve per-strategy

3. **LLM response parsing** - Different JSON schemas
   - Mitigation: Keep separate parsers per strategy

## Migration Order

Recommended order based on dependencies:
1. **legacy** (base, no dependencies)
2. **persona** (extends legacy logic)
3. **llm** (standalone)
4. **hybrid** (combines multiple)
5. **unified** (most complex, uses LLM)

## Blockers
None currently.

## Next Action
Start Phase 2: Create `decision/types.rs` with shared types extracted from `decision.rs`
