# Validation Checklist

## Pre-Implementation Checks
- [ ] Run `cargo test` - establish baseline (ALL tests must pass)
- [ ] Run `cargo check` - verify clean compilation
- [ ] Count lines: `mouse.rs` = 2,877 lines
- [ ] Verify submodules exist: `native.rs` (680 lines), `trajectory.rs` (500 lines), `types.rs` (74 lines)

## During Implementation (After Each Extraction)

### After Extracting `check_point_collision` (Step 1a)
- [ ] `cargo check` passes
- [ ] `mouse.rs` reduced by ~30 lines
- [ ] Collision detection still works (if integration tests available)

### After Extracting `generate_avoidance_path` (Step 1b)
- [ ] `cargo check` passes
- [ ] `mouse.rs` reduced by ~50 lines
- [ ] Avoidance path generation works correctly

### After Simplifying `dispatch_mouse_action` (Step 2)
- [ ] `cargo check` passes
- [ ] `mouse.rs` reduced by ~40 lines
- [ ] All mouse actions work: click, hover, double-click, right-click, drag

### After Simplifying `detect_ui_collisions_along_path` (Step 3)
- [ ] `cargo check` passes
- [ ] `mouse.rs` reduced by ~20 lines

## Post-Implementation Verification

### Line Count Verification
- [ ] `mouse.rs` reduced from 2,877 to ~2,400-2,500 lines
- [ ] `native.rs` unchanged (680 lines)
- [ ] `trajectory.rs` unchanged (500 lines)
- [ ] `types.rs` unchanged (74 lines)
- [ ] No new files created

### Functional Verification
- [ ] Run `cargo test` - ALL tests pass (especially mouse-related tests)
- [ ] Run `cargo test --lib mouse` - mouse tests pass
- [ ] Run `.\check.ps1` - FULL CI GATE PASSES

### Code Quality
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` applied
- [ ] Each helper function has single responsibility
- [ ] Function signatures are clean (no 10-parameter functions)

## Behavioral Verification

### Mouse Movement
- [ ] Bezier curve generation works
- [ ] Arc curve generation works
- [ ] Zigzag path generation works
- [ ] Collision avoidance works
- [ ] Cursor overlay syncs correctly

### Mouse Actions
- [ ] Click works (left, middle, right)
- [ ] Hover works
- [ ] Double-click works
- [ ] Drag works
- [ ] Native click calibration works

# CI Commands

```bash
# Full validation gate (MUST PASS before commit)
cd "C:\My Script\auto-rust"
.\check.ps1

# Individual checks
cd "C:\My Script\auto-rust"
cargo check
cargo test --lib mouse
cargo clippy -- -D warnings
cargo fmt --all -- --check

# Line count verification (mouse.rs should be ~2,400-2,500 lines)
Get-Content "src/utils/mouse.rs" | Measure-Object -Line | Select-Object Lines
```

# Quality Rules

1. **No logic changes**: Refactoring ONLY - behavior must be identical
2. **Keep in same file**: Do NOT create new files or subdirectories
3. **Preserve tests**: All 41 existing tests must pass without modification
4. **Document new functions**: Add rustdoc for extracted helper functions
5. **Incremental verification**: Run `cargo check` after EACH extraction
6. **No new dependencies**: Use existing types and functions


