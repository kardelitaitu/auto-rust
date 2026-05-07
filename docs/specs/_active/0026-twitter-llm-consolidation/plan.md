# Plan

## What Is the Solution

Consolidate small twitter modules by merging related functionality.

### Step 1: Audit All 27 Files
List all files with line counts to identify consolidation candidates.

### Step 2: Merge Persona + Constants
Merge `twitteractivity_persona.rs` (74 lines) into `twitteractivity_constants.rs` (444 lines).

### Step 3: Merge Errors + Retry
Merge `twitteractivity_errors.rs` into `twitteractivity_retry.rs`.

### Step 4: Consolidate Sentiment Sub-Modules
The sentiment directory has 5 files. Consider consolidating into a single module.

### Step 5: Update All Imports
Update all `use` statements across the codebase that reference the merged modules.

## Internal API Outline

Implementation details to be defined during active development.

## Decisions

1. **Only merge small modules**: Keep modules >500 lines as-is.
2. **Preserve public API**: Ensure all existing imports continue to work.
3. **Update docs**: Reflect module changes in rustdoc.

## Expected Outcome

After consolidation:
- File count reduced from 27 to ~20-22 files
- No functionality changed
- All existing tests pass
