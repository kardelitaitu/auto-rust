# Plan

## What Is the Solution

Extract command handlers from `cli.rs` into a `src/commands/` directory.

### Step 1: Analyze `cli.rs`
Count lines and identify command handler functions.

### Step 2: Create `src/commands/mod.rs`
Define module structure and re-exports.

### Step 3: Extract Command Handlers
Move each command handler to its own file:
- `commands/twitter.rs` - Twitter commands
- `commands/task.rs` - Task commands
- `commands/config.rs` - Config commands

### Step 4: Update `cli.rs`
Simplify to argument parsing only, delegating to command modules.

## Internal API Outline

Implementation details to be defined during active development.

## Decisions

1. **Keep cli.rs for parsing**: Don't move argument definitions.
2. **Extract only handlers**: Command execution logic goes to `commands/`.
3. **Preserve error handling**: Ensure all error paths work after refactoring.

## Expected Outcome

After refactoring:
- `cli.rs` reduced from ~X lines to ~200-300 lines
- Command handlers in dedicated `commands/` directory
- All CLI tests pass
