# Orchestrator Health Monitor Extraction

Status: `done`

Owner: `spec-agent`
Implementer: `pending`

## Summary
**REALITY CHECK**: Original spec claims were inaccurate. Code review reveals:

- `should_mark_session_unhealthy`: **8 lines** (NOT 638 as originally claimed)
- `execute_task_with_retry`: **287 lines** (substantial, at lines 535-821)
- `orchestrator.rs`: **1328 lines** (large but not extreme)
- `health_monitor.rs`: **374+ lines of dead code** (declared in lib.rs, NEVER used)

## Scope
- **In scope**: 
  - Decide: Extract `execute_task_with_retry`, integrate `health_monitor.rs`, or close spec
  - If proceeding: Actually reduce `orchestrator.rs` line count
  - Resolve dead code issue (`health_monitor.rs`)
- **Out of scope**: Changing core logic or health determination rules

## Next Step
**CRITICAL DECISION REQUIRED**: This spec was based on false premises. Choose path forward.

# Baseline

## What I Find (VERIFIED MEASUREMENTS)

**orchestrator.rs** (1328 lines total):
- Lines 1-100: Imports, helpers (`format_duration`, `broadcast_execution_count`)
- Lines 101-180: `GlobalExecutionSlot`, `SessionExecutionGuard`, `TaskAttemptFailure`
- Lines 181-473: `Orchestrator` struct + `execute_group` / `execute_group_with_cancel`
- Lines 474-533: `execute_task_on_session` (~60 lines)
- Lines 535-821: `execute_task_with_retry` (**287 lines**)
- Lines 850-857: `should_mark_session_unhealthy` (**8 lines** - already minimal!)
- Lines 860-1328: Tests (~468 lines)

**health_monitor.rs** (374+ lines):
- Declared in `lib.rs` line 23
- **ZERO usage in entire codebase**
- Contains: `HealthState`, `HealthStats`, `HealthMonitor` structs
- Has comprehensive tests but no integration

## What I Claim
**Original spec was WRONG:**
1. ~~"638-line health function"~~ → Actually 8 lines
2. ~~"315-line execute_task_with_retry"~~ → Actually 287 lines (close but not exact)
3. ~~"orchestrator.rs is 1400+ lines"~~ → Actually 1328 lines

**Actual issues:**
1. `health_monitor.rs` is dead code (374 lines wasted)
2. `execute_task_with_retry` at 287 lines could be extracted
3. `orchestrator.rs` is large but not unmanageable

## What Is the Proof
1. **Line count verified**: `Get-Content "src/orchestrator.rs" | Measure-Object -Line` = 1328
2. **Function boundaries verified**: `Select-String -Pattern "fn execute_task_with_retry|fn should_mark_session_unhealthy"` with context
3. **Usage verified**: `Select-String -Path "src/*.rs" -Pattern "health_monitor"` → ONLY in lib.rs line 23
4. **Code review**: Read actual function implementations in orchestrator.rs

## Brutal Truth
This spec was created based on **incorrect assumptions**. The "problem" it describes (638-line function) **does not exist**. 

**Options:**
1. **Close spec** - Original justification is false; code is reasonably organized
2. **Pivot spec** - Focus on dead code cleanup (`health_monitor.rs`)  
3. **Proceed anyway** - Extract 287-line function despite original premise being wrong
