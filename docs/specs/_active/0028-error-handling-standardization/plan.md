# Plan

## What Is the Solution

Standardize error handling across the codebase.

### Step 1: Audit Error Types
List all error types and handling patterns.

### Step 2: Choose Standard
Decide on `anyhow` vs custom errors for each module.

### Step 3: Add Context
Ensure all errors have proper context using `.context()` or `#[from]`.

### Step 4: Update Documentation
Add rustdoc examples for error handling patterns.

## Internal API Outline

Implementation details to be defined during active development.

## Decisions

1. **Prefer anyhow**: For most cases, use `anyhow::Result` with context.
2. **Custom errors only where needed**: When error types must be pattern-matched.
3. **Document patterns**: Add examples in rustdoc.

## Expected Outcome

After standardization:
- Consistent error handling across all modules
- All errors have proper context
- Error messages are actionable
