# Plan

## REALITY CHECK (Read Before Proceeding)

**Original spec claims (FALSE):**
- ❌ "638-line `should_mark_session_unhealthy` function" → **Actually 8 lines**
- ❌ "315-line `execute_task_with_retry`" → **Actually 287 lines**  
- ❌ "orchestrator.rs is 1400+ lines" → **Actually 1328 lines**

**Actual situation:**
- ✅ `should_mark_session_unhealthy` is already minimal (8 lines)
- ✅ `execute_task_with_retry` is 287 lines (manageable)
- ✅ `health_monitor.rs` is **374 lines of dead code** (NEVER USED)

---

## Decision Point: What Now?

### Option A: Close Spec (RECOMMENDED)
**Why:**
- Original problem doesn't exist
- Code is already reasonably organized
- Don't fix what isn't broken

**Action:**
```bash
# Move to done with explanation
mv "docs/specs/_active/0024-orchestrator-health-extraction" "docs/specs/_done/"
# Add note: "Closed - original claims were inaccurate"
```

### Option B: Clean Up Dead Code
**Focus:** `health_monitor.rs` (374 lines, never used)

**Steps:**
1. Decide if `health_monitor.rs` should be integrated
2. If yes: Wire it up in orchestrator or session modules
3. If no: Delete it and remove from `lib.rs`

**Pros:** Removes waste, improves codebase
**Cons:** May not be worth the effort

### Option C: Extract execute_task_with_retry
**Focus:** Move 287-line function to `task_runner.rs`

**Steps:**
1. Create `src/task_runner.rs`
2. Move function + helpers (`TaskAttemptFailure`, etc.)
3. Update imports, mod declarations
4. orchestrator.rs: 1328 → ~1041 lines

**Pros:** Better separation of concerns
**Cons:** 287 lines isn't "too long"; may add unnecessary abstraction

---

## Recommended Approach: Option A or B

**Why not Option C?**
- Original spec was wrong about the problem
- 287 lines isn't excessive for a complex retry function
- Don't create work just to feel productive

**If choosing Option B (dead code cleanup):**

### Phase 1: Evaluate health_monitor.rs (1 hour)
```bash
cd "C:\My Script\auto-rust"
cargo test health_monitor  # Run existing tests
# Review health_monitor.rs - is it useful?
```

### Phase 2: Decide (30 mins)
- **Integrate**: Use it in `orchestrator.rs` or `session/mod.rs`
- **Delete**: Remove file + `lib.rs` declaration

### Phase 3: Implement (1-2 hours)
- Wire up OR delete
- Run `cargo test` to verify
- Update docs

---

## My Recommendation

**Close this spec** and create a new one if needed:
- "Remove dead health_monitor.rs code" (if that's the real issue)
- "Extract execute_task_with_retry" (if you genuinely want that refactoring)

Don't proceed with this spec as-is - it's based on false premises.

# Internal API Outline (Option C Only)

### TaskRunner (if you insist on extracting)
```rust
// src/task_runner.rs
pub struct TaskRunner {
    config: Config,
    metrics: Arc<MetricsCollector>,
}

impl TaskRunner {
    pub async fn execute_with_retry(
        &self,
        task_def: &TaskDefinition,
        session: &Session,
        cancel_token: CancellationToken,
    ) -> TaskResult {
        // 287 lines moved from orchestrator::execute_task_with_retry
    }
}
```

# Decisions

## Decision: What Should We Actually Do?
**Status**: **NOT YET DECIDED**

**Options:**
- **A) Close spec** (RECOMMENDED): Original claims false; code is fine
- **B) Clean up dead code**: Integrate or delete `health_monitor.rs`
- **C) Extract anyway**: Move 287-line function despite original premise being wrong

**My vote**: Option A or B. Don't do Option C just to save face on a bad spec.

**Next Action**: You decide. Don't let sunk cost fallacy drive this.
