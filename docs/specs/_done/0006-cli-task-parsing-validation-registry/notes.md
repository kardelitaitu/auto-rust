# Implementation Notes: CLI Task Parsing + Validation + Registry

## Completed Work

### Parser Boundary Established

Created `src/cli/parser.rs` module with dedicated parsing logic:

- **`parse_task_groups()`** - Parses CLI task arguments with "then" separators
- **`parse_browser_filters()`** - Normalizes browser filter arguments  
- **`format_task_groups()`** - Formats task groups for display
- **`TaskDefinition`** struct - Represents parsed task with name and payload
- Comprehensive unit tests (22 tests) covering all parsing scenarios

### Registry-Backed Validation

Added `TaskValidationInfo` struct and `get_task_validation_info()` function in `src/validation/task.rs`:

- Structured payload guidance for each known task
- Examples, required fields, optional fields
- Used by CLI help system (no hardcoding in CLI layer)

### --help-task Command

Added `--help-task <TASK>` CLI flag:

- Added to `Args` struct in `src/cli/mod.rs`
- `get_task_help()` function retrieves help from registry
- `render_task_help()` formats output for display
- Shows task name, source, policy, and payload guidance

### Startup Mode Handling

Updated `src/main.rs`:

- Added `HelpTask` variant to `StartupMode` enum
- Updated `select_startup_mode()` to check `help_task` first
- Added handler in `run_async()` to print help and exit
- Added tests for HelpTask startup mode

### Files Modified

1. **New:** `src/cli/parser.rs` - Parser module (22 tests)
2. **New:** `src/cli/mod.rs` - CLI module with --help-task support
3. **Deleted:** `src/cli.rs` - Moved to cli/mod.rs
4. **Modified:** `src/validation/task.rs` - Added TaskValidationInfo
5. **Modified:** `src/validation/mod.rs` - Export new types
6. **Modified:** `src/main.rs` - HelpTask startup mode and tests

## Test Results

- **22 new parser tests** added in `cli/parser.rs`
- **4 new CLI tests** added in `cli/mod.rs`
- **3 new startup mode tests** added in `main.rs`
- All 2265 tests pass (was 2243)
- All CI checks pass

## Validation Checklist Status

- [x] Task-group parsing behavior stays stable after parser extraction
- [x] Browser filter parsing behavior stays stable after parser extraction
- [x] Task validation uses task names and registry metadata
- [x] `--help-task` returns task-specific payload guidance
- [x] Existing startup modes continue to work
- [x] `./check-fast.ps1` passes
- [x] `./check.ps1` passes

