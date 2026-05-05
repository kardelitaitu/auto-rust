# Pre-flight Validation Guide

The validation module provides comprehensive static analysis for task definitions, catching errors before execution begins.

## Overview

Pre-flight validation analyzes task files for:
- **Structural errors**: Invalid task names, missing required fields
- **Action validation**: Unknown action types, invalid parameters
- **Selector syntax**: CSS selector balance (brackets, quotes, parentheses)
- **Variable references**: Detects `${variable}` usage
- **Circular dependencies**: Detects task call cycles
- **Nesting limits**: Validates action nesting depth

## Quick Start

### Basic Validation

```rust
use auto::task::validation::validate_task;
use auto::task::dsl::TaskDefinition;

let task_def = /* load your task */;
let report = validate_task(&task_def);

if report.is_valid() {
    println!("Task is ready for execution!");
} else {
    for issue in &report.issues {
        println!("{}", issue.message());
    }
}
```

### With Known Tasks (for Call Validation)

```rust
use auto::task::validation::validate_task_with_known_tasks;

let known_tasks = vec!["login", "fetch_data", "process_results"];
let report = validate_task_with_known_tasks(&task_def, known_tasks);
```

## Validation Report

The `ValidationReport` provides detailed analysis results:

```rust
pub struct ValidationReport {
    pub task_name: String,           // Name of validated task
    pub issues: Vec<ValidationIssue>, // All found issues
    pub action_count: usize,          // Total action nodes
    pub variables_referenced: HashSet<String>, // Variables used
    pub tasks_called: HashSet<String>, // Tasks called via `call`
}
```

### Issue Types

```rust
pub enum ValidationIssue {
    Error(String),    // Prevents execution
    Warning(String),  // Suggests improvement
}
```

**Errors** must be fixed before execution.
**Warnings** indicate potential issues but don't block execution.

## TaskValidator Configuration

### Basic Validator

```rust
use auto::task::validation::TaskValidator;

let validator = TaskValidator::new();
let report = validator.validate(&task_def);
```

### With Known Tasks

```rust
let validator = TaskValidator::new()
    .with_known_tasks(vec!["login", "logout", "check_status"]);
```

### With Custom Nesting Limit

```rust
let validator = TaskValidator::new()
    .with_max_nesting_depth(5); // Default is 10
```

## Validation Rules

### Task Structure

| Check | Severity | Description |
|-------|----------|-------------|
| Empty name | Error | Task must have a name |
| Spaces in name | Error | Use underscores or hyphens |
| Special characters | Warning | Recommend alphanumeric, `_`, `-` |
| No actions/includes | Error | Task must have content |
| Duplicate parameters | Error | Parameter names must be unique |

### Action Types

All 23 action types are validated:

- **Navigation**: `navigate` - URL format validation
- **Interaction**: `click`, `type`, `clear`, `hover`, `select`, `right_click`, `double_click`
- **Wait**: `wait`, `wait_for` - Duration sanity checks
- **Extraction**: `extract` - Variable naming
- **Execution**: `execute` - Script presence
- **Control Flow**: `if`, `loop`, `call`, `parallel`, `retry`, `foreach`, `while`, `try`
- **Screenshot**: `screenshot` - Path validation
- **Logging**: `log` - Message and level validation

### Selector Validation

CSS selectors are checked for:

```yaml
# Valid selector
- click:
    selector: "div[class='content'] > button#submit"

# Invalid - triggers errors
- click:
    selector: ""                          # Error: Empty selector
- click:
    selector: "div[class='test'"          # Error: Unbalanced brackets
- click:
    selector: "input[type='text"          # Error: Unbalanced quotes
```

### Variable Detection

Variables referenced via `${variable}` syntax are extracted:

```yaml
actions:
  - type:
      selector: "#name"
      text: "${user_name}"        # Detected: user_name
  - navigate:
      url: "https://${domain}/api"  # Detected: domain
```

Report shows:
```
variables_referenced: {"user_name", "domain"}
```

### Call Action Validation

When using known tasks:

```yaml
actions:
  - call:
      task: "unknown_task"        # Warning: Not in known task list
```

## Control Flow Validation

### If/Then/Else

```yaml
- if:
    condition:
      element_exists: "#modal"
    then: []                        # Warning: Empty then block
    else:
      - wait: { duration_ms: 100 }
```

### Enhanced Conditions (12 Types)

| Condition | Validation Checks |
|-----------|-------------------|
| `element_visible` / `element_exists` | Valid selector format |
| `variable_equals` | Variable name non-empty |
| `text_matches` / `variable_matches` | Valid regex pattern, non-empty selector/variable |
| `numeric_greater_than` / `numeric_less_than` / `numeric_range` | Variable name non-empty, numeric value |
| `date_before` / `date_after` | Variable name non-empty, valid date string |
| `array_contains` / `array_length` | Variable name non-empty |

**Regex Validation:**
```yaml
- if:
    condition:
      text_matches:
        selector: "#status"
        pattern: "[invalid("          # Error: Invalid regex
```

**Numeric Range Validation:**
```yaml
- if:
    condition:
      numeric_range:
        name: "score"
        min: 100
        max: 50                      # Error: min > max
```

### Loop

```yaml
- loop:
    count: 0                        # Warning: No iterations
    actions:
      - click: { selector: "#button" }

- loop:
    # Error: Must have count or condition
    actions:
      - wait: { duration_ms: 100 }
```

### Retry

```yaml
- retry:
    actions:
      - click: { selector: "#flaky" }
    max_attempts: 0                 # Error: Cannot be 0
    initial_delay_ms: 5000
    max_delay_ms: 1000              # Error: Initial > Max
```

### Foreach

```yaml
- foreach:
    variable: "item"
    collection:
      type: range
      start: 10
      end: 5                         # Error: Start >= End
    actions:
      - log: { message: "{{item}}" }
```

## Integration with DslExecutor

### Optional Pre-flight Check

```rust
use auto::task::dsl_executor::DslExecutor;
use auto::task::validation::validate_task;

pub async fn execute_with_validation(
    api: &TaskContext,
    task_def: &TaskDefinition,
) -> Result<()> {
    // Pre-flight validation
    let report = validate_task(task_def);
    
    if !report.is_valid() {
        let errors: Vec<String> = report.issues
            .iter()
            .filter(|i| i.is_error())
            .map(|i| i.message().to_string())
            .collect();
        
        return Err(anyhow::anyhow!(
            "Task validation failed: {}",
            errors.join(", ")
        ));
    }
    
    // Execute if valid
    let mut executor = DslExecutor::new(api, task_def);
    executor.execute().await
}
```

### CLI Integration

```bash
# Validate all tasks
cargo run -- --validate-tasks

# Validate specific task
cargo run -- --validate my_task

# Dry run (validation + simulation)
cargo run -- --dry-run my_task
```

## Error Examples

### Common Validation Errors

```
❌ Task name cannot be empty
❌ Task 'my task' name cannot contain spaces
❌ Task must have at least one action or include
❌ Parameter 'timeout' is required but has no default

❌ actions[0]: Selector cannot be empty
❌ actions[1].then[0]: Selector has unbalanced brackets: 'div[class='test''
❌ actions[2]: Loop must have either 'count' or 'condition'
❌ actions[3]: max_attempts cannot be 0
❌ actions[4]: initial_delay_ms (5000) > max_delay_ms (1000)
```

### Common Warnings

```
⚠️ Task 'complex_task' has 42 actions - consider breaking into smaller tasks
⚠️ Parameter 'retries' is required but has a default value (redundant)
⚠️ actions[5]: Wait duration is 0ms (no-op)
⚠️ actions[6]: 'then' block has no actions
⚠️ actions[7]: Task 'helper_task' is not in the known task list
```

## Testing

### Unit Tests

```rust
#[test]
fn test_valid_task() {
    let task = create_basic_task();
    let report = validate_task(&task);
    assert!(report.is_valid());
}

#[test]
fn test_empty_selector() {
    let mut task = create_basic_task();
    task.actions = vec![Action::Click { 
        selector: "".to_string() 
    }];
    
    let report = validate_task(&task);
    assert!(!report.is_valid());
    assert!(report.issues.iter().any(|i| 
        i.message().contains("Selector cannot be empty")
    ));
}
```

### Integration with Test Suite

```rust
#[tokio::test]
async fn test_validation_before_execution() {
    let task_def = load_task("invalid_task.yaml");
    let report = validate_task(&task_def);
    
    assert!(!report.is_valid());
    
    // Verify execution would fail
    let result = execute_dsl_task(&api, &task_def).await;
    assert!(result.is_err());
}
```

## Performance

Validation is fast and suitable for CI pipelines:

- **Small tasks** (< 10 actions): < 1ms
- **Medium tasks** (10-50 actions): < 5ms
- **Large tasks** (50+ actions): < 10ms

The validator performs a single pass through the task structure without executing any browser operations.

## Best Practices

1. **Always validate in CI**: Add `--validate-tasks` to your CI pipeline
2. **Use known tasks**: Provide task list for better Call validation
3. **Fix warnings**: They often indicate future bugs
4. **Validate after editing**: Run validation after any task file changes
5. **Check selectors**: Validation catches syntax errors early

## API Reference

### Functions

```rust
/// Quick validation without configuration
pub fn validate_task(def: &TaskDefinition) -> ValidationReport;

/// Validation with known task list
pub fn validate_task_with_known_tasks(
    def: &TaskDefinition,
    known_tasks: impl IntoIterator<Item = impl Into<String>>,
) -> ValidationReport;
```

### TaskValidator Methods

```rust
impl TaskValidator {
    pub fn new() -> Self;
    pub fn with_max_nesting_depth(self, depth: usize) -> Self;
    pub fn with_known_tasks(self, tasks: impl IntoIterator<Item = impl Into<String>>) -> Self;
    pub fn with_parameter(self, name: impl Into<String>, param: ParameterDef) -> Self;
    pub fn validate(&self, def: &TaskDefinition) -> ValidationReport;
}
```

### ValidationReport Methods

```rust
impl ValidationReport {
    pub fn is_valid(&self) -> bool;      // No errors
    pub fn has_errors(&self) -> bool;     // Has errors
    pub fn error_count(&self) -> usize;  // Number of errors
    pub fn warning_count(&self) -> usize;// Number of warnings
    pub fn summary(&self) -> String;     // Human-readable summary
}
```

## See Also

- [DSL Task Syntax](API_REFERENCE.md#dsl-task-syntax)
- [Task Composition](README.md#task-composition-call-action)
- [Testing Guide](TEST_SUMMARY.md)
