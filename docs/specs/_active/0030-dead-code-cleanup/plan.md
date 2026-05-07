# Plan#

## What Is the Solution#

Systematic dead code cleanup.

### Step 1: Run Clippy with Fix
```bash
cargo clippy --fix --allow-no-vcs -- -D warnings
```

### Step 2: Review Unused Imports
Check for unused `use` statements across all modules.

### Step 3: Find Unreachable Code
Look for functions that are never called, deprecated markers.

### Step 4: Remove Dead Test Utilities
Remove test helpers that are no longer used.

## Internal API Outline#

No API changes - removal only.

## Decisions#

1. **Conservative removal**: Only remove code confirmed dead.
2. **Keep public API**: Don't remove public functions unless confirmed unused.
3. **Test after each**: Run `cargo test` after each removal.

## Expected Outcome#

After cleanup:
- No clippy warnings for dead code
- Cleaner module files
- Reduced maintenance burden
