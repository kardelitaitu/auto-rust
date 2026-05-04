# DSL Task Templates

A collection of reusable task templates for the auto-rust DSL task system.

## Quick Start

```bash
# Run a basic template
auto-rust --task simple-navigation --url "https://example.com"

# Run with custom parameters
auto-rust --task form-login \
  --url "https://example.com/login" \
  --username "user@example.com" \
  --password "secret123"

# Validate a template before running
auto-rust --validate-tasks --task simple-navigation
```

## Template Categories

### Basic Templates
Simple, single-purpose tasks perfect for learning and common operations.

| Template | Purpose | Key Actions |
|----------|---------|-------------|
| `simple-navigation` | Navigate to URL and wait | navigate, wait, log |
| `form-login` | Standard login form handling | navigate, type, click, if/else |

### Intermediate Templates
More complex workflows with conditions and data extraction.

| Template | Purpose | Key Actions |
|----------|---------|-------------|
| `search-and-extract` | Search and extract results | navigate, type, extract, loop pattern |
| `conditional-click` | Handle optional UI elements | if/else, element_exists, element_visible |

### Advanced Templates
Complex compositions demonstrating parallel execution and task calling.

| Template | Purpose | Key Actions |
|----------|---------|-------------|
| `parallel-scrape` | Scrape multiple sections concurrently | parallel, extract |
| `composed-workflow` | Chain multiple tasks together | call, if/else |

## Template Structure

Each template follows this structure:

```yaml
name: template-name
description: "What this template does"
policy: default

parameters:
  param_name:
    type: string|integer|boolean|url|selector
    required: true|false
    default: "optional default value"
    description: "What this parameter does"

actions:
  - action_name:
      # action-specific parameters
```

## Parameter Types

| Type | Description | Example |
|------|-------------|---------|
| `string` | Text value | `"Hello World"` |
| `integer` | Whole number | `5000` |
| `boolean` | true/false | `true` |
| `url` | Valid URL | `https://example.com` |
| `selector` | CSS selector | `#button`, `.class`, `input[name='email']` |

## Common Patterns

### Navigation + Wait
```yaml
- navigate:
    url: "{{url}}"
- wait:
    duration_ms: 2000
```

### Conditional Action
```yaml
- if:
    condition:
      element_visible: "#button"
    then:
      - click:
          selector: "#button"
    else:
      - log:
          message: "Button not found"
          level: warn
```

### Parallel Execution
```yaml
- parallel:
    max_concurrency: 3
    actions:
      - extract:
          selector: ".section-1"
          variable: "section1"
      - extract:
          selector: ".section-2"
          variable: "section2"
```

### Task Composition
```yaml
- call:
    task: simple-navigation
    parameters:
      url: "{{target_url}}"
      wait_ms: 3000
```

## Creating Your Own Templates

1. Copy an existing template as a starting point
2. Modify the `name`, `description`, and `parameters`
3. Define your action sequence
4. Test with `--validate-tasks` first
5. Run with `--dry-run` to preview without execution

Example:
```bash
# Copy and modify
cp examples/tasks/basic/simple-navigation.task my-task.task

# Edit my-task.task with your requirements

# Validate
auto-rust --validate-tasks --task my-task

# Dry run
auto-rust --task my-task --url "https://example.com" --dry-run

# Execute
auto-rust --task my-task --url "https://example.com"
```

## Tips

- Use descriptive parameter names with clear descriptions
- Provide sensible defaults for optional parameters
- Include usage examples in template comments
- Use `log` actions for debugging and monitoring
- Add `screenshot` actions at key checkpoints
- Test with `--dry-run` before production use
