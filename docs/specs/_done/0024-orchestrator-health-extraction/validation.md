# Validation Checklist

## Pre-Implementation
- [ ] Run `cargo check` to establish baseline
- [ ] Run `cargo test` to ensure all tests pass
- [ ] Count lines in `orchestrator.rs` (target: <1000 after refactoring)
- [ ] Document current `execute_task_with_retry` line count
- [ ] Review `src/health_monitor.rs` usage in codebase

## During Implementation
- [ ] Create new module (`task_runner.rs` or health integration)
- [ ] Move code using `edit_file` to preserve git history
- [ ] Update module declarations in `lib.rs` or `mod.rs`
- [ ] Ensure all imports are correct
- [ ] Run `cargo check` after each major change

## Post-Implementation
- [ ] Run `cargo test` - all tests must pass
- [ ] Run `cargo clippy` - no new warnings
- [ ] Verify orchestrator.rs line count reduced
- [ ] Check `should_mark_session_unhealthy` behavior unchanged
- [ ] Verify `execute_task_with_retry` logic preserved (if moved)
- [ ] Test with sample task execution (if possible)

## Behavioral Verification
- [ ] Session health transitions work correctly
- [ ] Task retry logic unchanged
- [ ] Cancellation token propagated correctly
- [ ] Timeout handling preserved
- [ ] Metrics collection still works

# CI Commands

```bash
# Full validation gate
cd "C:\My Script\auto-rust" && .\check.ps1

# Individual checks
cd "C:\My Script\auto-rust" && cargo check
cd "C:\My Script\auto-rust" && cargo test
cd "C:\My Script\auto-rust" && cargo clippy
```

# Quality Rules

1. **No logic changes**: Refactoring only, behavior must remain identical
2. **Preserve tests**: All existing tests must pass without modification
3. **Document new modules**: Add rustdoc for extracted modules
4. **Keep public API stable**: No breaking changes to orchestrator API
5. **Line count target**: orchestrator.rs should be under 1000 lines
6. **Use existing code**: Integrate with `health_monitor.rs` if possible
