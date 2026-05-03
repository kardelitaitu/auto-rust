# DemoQA Task

Demonstrates the task-api on the DemoQA text box page: focus, keyboard input, random cursor movement, submit, and output verification.

## Quick Start

```bash
cargo run demoqa
```

## What It Does

Fills the DemoQA text box form with fixed sample data:

| Field | Value |
|-------|-------|
| Full Name | `Demo QA` |
| Email | `demoqa@example.com` |
| Current Address | `123 Demo Street, Demo City` |
| Permanent Address | `456 Demo Avenue, Demo Town` |

## Actions Demonstrated

1. Navigate to [demoqa.com/text-box](https://demoqa.com/text-box)
2. `api.focus()` - Focus input fields
3. `api.keyboard()` - Type text with human-like timing
4. `api.randomcursor()` - Move cursor randomly
5. Submit form
6. Verify output appears

## Purpose

This is a **learning task** for understanding:
- Task structure and context API
- Element interaction patterns
- Form filling workflows
- Verification techniques

See [Tutorial: Building First Task](../TUTORIAL_BUILDING_FIRST_TASK.md) for building your own tasks.
