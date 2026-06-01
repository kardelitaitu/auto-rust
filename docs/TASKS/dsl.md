last audited 13-05-26 by Buffy
# DSL Task Guide

Use this guide when you change task parsing, validation, execution, or DSL task authoring.

## What It Covers

- Task definition loading
- YAML and TOML parsing
- Action execution
- Variables and condition handling
- Control flow like `if`, `loop`, `foreach`, `while`, `retry`, and `parallel`
- Validation before execution

## Core Rule

Keep DSL behavior predictable.

- Parse once.
- Validate before execution.
- Execute with the smallest safe scope.
- Prefer shared helpers over task-specific ad hoc logic.

## Recommended Reading Order

1. [docs/TASKS/overview.md](overview.md)
2. [docs/API_REFERENCE.md](../API_REFERENCE.md)
3. `src/task/dsl/parser.rs`
4. `src/task/dsl/executor.rs`
5. `src/task/dsl/control_flow.rs`
6. `src/task/dsl/evaluator.rs`

## When to Use This Doc

- Adding a new DSL action
- Changing how task payloads are resolved
- Editing validation rules
- Touching control flow or retry behavior
- Modifying task execution reports or debug flow

## Action Reference

### Execute

Runs JavaScript in the browser page and logs the result. Does not return the result to subsequent actions — use `Extract` for reading DOM text.

**YAML:**
```yaml
- action: execute
  script: "document.title"

- action: execute
  script: "document.querySelector('#status').textContent"
```

**TOML:**
```toml
[[actions]]
action = "execute"
script = "document.title"
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `script` | string | yes | JavaScript expression or statements to execute |

**Behavior:**
- Supports `${variable}` substitution — variables are resolved before execution
- Clears the selector cache after execution (the DOM may have changed)
- Propagates API errors (failed JS evaluation fails the action)
- Result is logged at `info` level but not stored or returned

**Example with variable substitution:**
```yaml
- action: execute
  script: "document.getElementById('${element_id}').click()"
```

> **Calls API:** Uses `api.execute_js(script)` internally.

---

### Screenshot

Captures a screenshot of the current page or a specific element. The screenshot file is saved to an auto-generated path (or optionally a custom path).

**YAML:**
```yaml
# Full page screenshot
- action: screenshot

# Screenshot with custom path
- action: screenshot
  path: "results/page1.png"

# Screenshot a specific element (scrolls to it first)
- action: screenshot
  selector: "#chart-container"

# Screenshot element with custom path
- action: screenshot
  path: "screenshots/chart.png"
  selector: "#chart"
```

**TOML:**
```toml
[[actions]]
action = "screenshot"

[[actions]]
action = "screenshot"
selector = "#chart"
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string (optional) | no | Custom save path; if omitted, path is auto-generated |
| `selector` | string (optional) | no | CSS selector for element screenshot; full page if omitted |

**Behavior:**
- If a `selector` is provided, the element is scrolled into view before capturing
- If no `selector`, captures the full viewport
- Clears the selector cache after execution (the page visual state changed)
- Supports `${variable}` substitution in both `selector` and `path`
- The API return path is logged; custom paths are noted in logs but rely on external file management

> **Calls API:** Uses the browser screenshot API internally.

---

### Select

Selects an option from a `<select>` dropdown by visible text or by the `value` attribute. Uses JavaScript injection to set the dropdown value.

**YAML:**
```yaml
# Select by visible text (default)
- action: select
  selector: "#country"
  value: "United States"

# Select by value attribute
- action: select
  selector: "#country"
  value: "US"
  by_value: true

# Select with variable substitution
- action: select
  selector: "${dropdown_id}"
  value: "${target_country}"
  by_value: false
```

**TOML:**
```toml
[[actions]]
action = "select"
selector = "#country"
value = "United States"

[[actions]]
action = "select"
selector = "#country"
value = "US"
by_value = true
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `selector` | string | yes | CSS selector for the `<select>` element |
| `value` | string | yes | Value to select — either the visible text or the `value` attribute |
| `by_value` | bool (optional) | no | `true` = select by `value` attribute; `false` (default) = select by visible text |

**Behavior:**
- By default (`by_value` omitted or `false`), finds the `<option>` whose `text.trim()` matches `value`
- With `by_value: true`, sets `element.value` directly to the provided `value` string
- Supports `${variable}` substitution in both `selector` and `value` fields
- Clears the selector cache after execution (the form state changed)
- Propagates errors from `execute_js` (invalid selector or JS failure)

**Variable substitution example:**
```yaml
- action: extract
  selector: "#selected-country"
  variable: target_country

- action: select
  selector: "#country-dropdown"
  value: "${target_country}"
```

> **Calls API:** Uses `api.execute_js()` internally to set the dropdown value.

---

### Press

Simulates a key press, optionally with modifier keys (e.g., `Control`, `Shift`, `Alt`).
Useful for keyboard shortcuts, form submission (Enter), or dismissing dialogs (Escape).

**YAML:**
```yaml
# Press a single key
- action: press
  key: "Enter"

# Press with modifiers (e.g., Control+C)
- action: press
  key: "c"
  modifiers:
    - "Control"

# Multiple modifiers
- action: press
  key: "Delete"
  modifiers:
    - "Control"
    - "Shift"

# With variable substitution
- action: press
  key: "${hotkey}"
  modifiers:
    - "${mod_key}"
```

**TOML:**
```toml
[[actions]]
action = "press"
key = "Enter"

[[actions]]
action = "press"
key = "c"
modifiers = ["Control"]

[[actions]]
action = "press"
key = "Delete"
modifiers = ["Control", "Shift"]
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `key` | string | yes | Key to press (e.g., `"Enter"`, `"Escape"`, `"Tab"`, `"c"`, `"Delete"`) |
| `modifiers` | array of strings (optional) | no | Modifier keys held during press (e.g., `["Control"]`, `["Shift", "Alt"]`). Omitted or empty for no modifiers |

**Behavior:**
- Presses the specified key in the focused element (or globally for system keys like Escape)
- Common key names: `"Enter"`, `"Escape"`, `"Tab"`, `"Delete"`, `"Backspace"`, `"ArrowUp"`, `"ArrowDown"`, letter keys, number keys
- Common modifiers: `"Control"`, `"Shift"`, `"Alt"`, `"Meta"` (Windows/Command key)
- Supports `${variable}` substitution in both `key` and individual `modifiers` entries
- Clears the selector cache after execution (keyboard input may change DOM state)
- Propagates API errors

> **Calls API:** Uses `api.press(key, modifiers)` internally.

---

### Clear

Clears the contents of an input field, textarea, or other editable element.

**YAML:**
```yaml
- action: clear
  selector: "#search-input"

- action: clear
  selector: "${field_id}"
```

**TOML:**
```toml
[[actions]]
action = "clear"
selector = "#search-input"
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `selector` | string | yes | CSS selector for the input field to clear |

**Behavior:**
- Clears the text content of the matched input element
- Supports `${variable}` substitution in the selector
- Clears the selector cache after execution (DOM content changed)
- Propagates API errors (invalid selector or non-editable element)

> **Calls API:** Uses `api.clear(selector)` internally.

---

### Hover

Moves the cursor over an element, triggering `:hover` CSS styles and any
JavaScript `mouseenter`/`mouseover` event handlers.

**YAML:**
```yaml
- action: hover
  selector: "#dropdown-menu"

- action: hover
  selector: "${menu_id}"
```

**TOML:**
```toml
[[actions]]
action = "hover"
selector = "#dropdown-menu"
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `selector` | string | yes | CSS selector for the element to hover over |

**Behavior:**
- Simulates moving the mouse cursor over the element
- Useful for opening dropdown menus, revealing tooltips, or triggering hover effects
- Supports `${variable}` substitution in the selector
- Clears the selector cache after execution (hover state may change the DOM)
- Propagates API errors (invalid selector or element not visible)

> **Calls API:** Uses `api.hover(selector)` internally.

---

### RightClick

Simulates a right-click (context menu click) on an element.

**YAML:**
```yaml
- action: right_click
  selector: "#context-target"

- action: right_click
  selector: "${target_id}"
```

**TOML:**
```toml
[[actions]]
action = "right_click"
selector = "#context-target"
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `selector` | string | yes | CSS selector for the element to right-click |

**Behavior:**
- Simulates a right-click on the target element, typically opening a context menu
- Supports `${variable}` substitution in the selector
- Clears the selector cache after execution (context menu may change the DOM)
- Propagates API errors (invalid selector or element not found)

> **Calls API:** Uses `api.right_click(selector)` internally.

---

### DoubleClick

Simulates a double-click on an element.

**YAML:**
```yaml
- action: double_click
  selector: "#editable-text"

- action: double_click
  selector: "${target}"
```

**TOML:**
```toml
[[actions]]
action = "double_click"
selector = "#editable-text"
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `selector` | string | yes | CSS selector for the element to double-click |

**Behavior:**
- Simulates a double-click, useful for selecting text, opening files, or triggering `ondblclick` handlers
- Supports `${variable}` substitution in the selector
- Clears the selector cache after execution (double-click may change DOM state)
- Propagates API errors (invalid selector or element not found)

> **Calls API:** Uses `api.double_click(selector)` internally.

---

### Extract

Reads the text content of an element and optionally stores it in a variable
for use in subsequent actions (e.g., for assertion or as input to another action).

**YAML:**
```yaml
# Extract text without storing (for logging/assertion)
- action: extract
  selector: "#page-title"

# Extract text and store in a variable
- action: extract
  selector: "#result-text"
  variable: result

# Extract with variable substitution in selector
- action: extract
  selector: "#${dynamic_id}"
  variable: dynamic_content

# Use the extracted variable in a later action
- action: type
  selector: "#output-field"
  text: "${result}"
```

**TOML:**
```toml
[[actions]]
action = "extract"
selector = "#page-title"

[[actions]]
action = "extract"
selector = "#result-text"
variable = "result"
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `selector` | string | yes | CSS selector for the element to read text from |
| `variable` | string (optional) | no | Name of the variable to store the extracted text in. If omitted, the text is still fetched (and logged) but not stored |

**Behavior:**
- Reads the visible text content of the matched element
- If the element is not found or has no text, stores an empty string
- Supports `${variable}` substitution in the selector
- **Does NOT clear the selector cache** — Extract is a read-only operation and preserves cached DOM state
- Propagates API errors
- The stored variable can be referenced in later actions using `${variable_name}` syntax

> **Calls API:** Uses `api.text(selector)` internally to read element text.

---

### Navigate

Navigates the browser to a specified URL. This is typically the first action in a task.

**YAML:**
```yaml
- action: navigate
  url: "https://example.com"

- action: navigate
  url: "${base_url}/login"
```

**TOML:**
```toml
[[actions]]
action = "navigate"
url = "https://example.com"
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | yes | Full URL to navigate to (http/https) |

**Behavior:**
- Navigates to the specified URL with a 30-second timeout
- Supports `${variable}` substitution in the URL
- Clears the selector cache after navigation (entire page DOM changes)
- Propagates API errors (timeout, unreachable host, invalid URL)

> **Calls API:** Uses `api.navigate(url, 30000)` internally with a 30-second timeout.

---

### Click

Simulates a mouse click on an element identified by a CSS selector.

**YAML:**
```yaml
- action: click
  selector: "#submit-button"

- action: click
  selector: "${btn_id}"
```

**TOML:**
```toml
[[actions]]
action = "click"
selector = "#submit-button"
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `selector` | string | yes | CSS selector for the element to click |

**Behavior:**
- Clicks the center of the matched element
- The element must be visible and within the viewport
- Supports `${variable}` substitution in the selector
- Clears the selector cache after execution (click may change DOM state)
- Propagates API errors (element not found, not visible, or not interactive)

> **Calls API:** Uses `api.click(selector)` internally.

---

### Type

Types text into an input field, textarea, or other editable element.

**YAML:**
```yaml
- action: type
  selector: "#username"
  text: "admin"

- action: type
  selector: "#search-box"
  text: "${search_query}"

- action: type
  selector: "${input_id}"
  text: "Hello ${name}"
```

**TOML:**
```toml
[[actions]]
action = "type"
selector = "#username"
text = "admin"
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `selector` | string | yes | CSS selector for the target input element |
| `text` | string | yes | Text to type into the element |

**Behavior:**
- Types the specified text into the focused/matched input element
- Supports `${variable}` substitution in both `selector` and `text` fields
- Clears the selector cache after execution (input content changed)
- Propagates API errors (element not found, not editable, or not visible)

> **Calls API:** Uses `api.r#type(selector, text)` internally (the `r#` prefix is Rust's keyword-escaped name for the `type` method).

---

### ScrollTo

Scrolls the page until the specified element is visible within the viewport.

**YAML:**
```yaml
- action: scroll_to
  selector: "#footer"

- action: scroll_to
  selector: "${target_id}"
```

**TOML:**
```toml
[[actions]]
action = "scroll_to"
selector = "#footer"
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `selector` | string | yes | CSS selector for the element to scroll to |

**Behavior:**
- Scrolls the page so the target element enters the visible viewport
- Useful before clicking elements that may be below the fold
- Supports `${variable}` substitution in the selector
- Clears the selector cache after execution (scroll position changes visible DOM)
- Propagates API errors (element not found)

> **Calls API:** Uses `api.scroll_to(selector)` internally.

---

### WaitFor

Waits for an element matching a CSS selector to appear in the DOM. Polls at regular
intervals until the element is found or a timeout is reached. Preferred over a fixed
`wait` duration when you need to wait for dynamic content.

**YAML:**
```yaml
# Wait for an element with default timeout (5000ms)
- action: wait_for
  selector: "#content"

# Wait with a custom timeout
- action: wait_for
  selector: ".search-result"
  timeout_ms: 10000

# With variable substitution
- action: wait_for
  selector: "${target_selector}"
  timeout_ms: ${timeout_ms}
```

**TOML:**
```toml
[[actions]]
action = "wait_for"
selector = "#content"

[[actions]]
action = "wait_for"
selector = ".search-result"
timeout_ms = 10000
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `selector` | string | yes | CSS selector of the element to wait for |
| `timeout_ms` | integer (optional) | no | Maximum wait time in milliseconds (default: 5000ms) |

**Behavior:**
- Polls the DOM every 100ms checking if the element exists
- Returns `Ok(())` as soon as the element is found — no additional delay
- Supports `${variable}` substitution in the selector
- Does **not** clear the selector cache — WaitFor is a read-only operation
- Propagates errors with a descriptive timeout message if the element doesn't appear
- The default timeout of 5000ms can be overridden per-action with `timeout_ms`
- Prefer WaitFor over a fixed `wait` when waiting for dynamic content — it's faster
  on success (element appears early) and more reliable (adapts to network conditions)

> **Calls API:** Uses `api.exists(selector)` internally (via the cached selector cache).

---

### Wait

Pauses execution for a specified duration (in milliseconds). Does not make any API calls.
Useful for waiting for page transitions, animations, or network requests to complete.

**YAML:**
```yaml
- action: wait
  duration_ms: 2000

- action: wait
  duration_ms: 500
```

**TOML:**
```toml
[[actions]]
action = "wait"
duration_ms = 2000
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `duration_ms` | integer | yes | Duration to wait in milliseconds |

**Behavior:**
- Pauses execution for the specified number of milliseconds
- Does **not** make any API calls (works entirely client-side)
- Does **not** clear the selector cache — Wait is a timing primitive that does not touch the DOM
- Always succeeds — no error can be returned
- Prefer `wait_for` with a selector when waiting for a specific element to appear

> **Calls API:** No API calls — pure client-side timing primitive.

---

### Log

Logs a message at the specified severity level. Useful for debugging,
diagnostic tracing, or annotating task execution progress.

**YAML:**
```yaml
- action: log
  message: "Starting login flow"

- action: log
  message: "User ${username} logged in successfully"
  level: info

- action: log
  message: "Rate limit approaching"
  level: warn

- action: log
  message: "Failed to find element: ${last_error}"
  level: error
```

**TOML:**
```toml
[[actions]]
action = "log"
message = "Starting login flow"

[[actions]]
action = "log"
message = "Processing complete"
level = "info"
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `message` | string | yes | Message to log |
| `level` | string (optional) | no | Log level: `"debug"`, `"info"` (default), `"warn"`, or `"error"` |

**Behavior:**
- Logs the message at the specified level via the standard log framework
- Supports `${variable}` substitution in the message
- Does **not** make any API calls (purely diagnostic)
- Does **not** clear the selector cache — Log is a pure diagnostic action
- Default level is `"info"` if omitted
- Always succeeds — no error can be returned

> **Calls API:** No API calls — purely diagnostic, uses the standard log framework.

---

### Common action patterns

All actions follow these conventions:

| Pattern | Navigate | Click | Type | ScrollTo | WaitFor | Wait | Log | Execute | Screenshot | Select | Press | Clear | Hover | RightClick | DoubleClick | Extract |
|---------|----------|-------|------|----------|---------|------|-----|---------|------------|--------|-------|-------|-------|------------|-------------|---------|
| Clears selector cache | ✓ | ✓ | ✓ | ✓ | ✗ (read-only) | ✗ | ✗ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ (read-only) |
| Variable substitution in fields | ✓ (`url`) | ✓ (`selector`) | ✓ (`selector`, `text`) | ✓ (`selector`) | ✓ (`selector`) | — | ✓ (`message`) | ✓ (`script`) | ✓ (`selector`, `path`) | ✓ (`selector`, `value`) | ✓ (`key`, `modifiers`) | ✓ (`selector`) | ✓ (`selector`) | ✓ (`selector`) | ✓ (`selector`) | ✓ (`selector`) |
| Propagates API errors | ✓ | ✓ | ✓ | ✓ | ✓ (timeout) | — | — | ✓ | ✓ | ✓ (via `execute_js`) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| Makes API calls | ✓ | ✓ | ✓ | ✓ | ✓ | ✗ | ✗ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

## Control Flow

Control flow actions nest other actions and manage execution order, error handling, and iteration. They use the same `action` field tag as simple actions.

### If/Else

Conditionally executes one of two branches based on a [condition](#conditions). If the condition evaluates to true, the `then` branch runs; otherwise, the optional `else` branch runs.

**YAML:**
```yaml
- action: if
  condition:
    element_exists:
      selector: ".success-message"
  then:
    - action: extract
      selector: ".success-message"
      variable: result
  else:
    - action: log
      message: "Element not found"
      level: warn
```

**TOML:**
```toml
[[actions]]
action = "if"
condition = { element_exists = { selector = ".success-message" } }
then = [
  { action = "extract", selector = ".success-message", variable = "result" }
]
else = [
  { action = "log", message = "Element not found", level = "warn" }
]
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `condition` | condition | yes | A [Condition](#conditions) to evaluate |
| `then` | array of actions | yes | Actions to execute if the condition is true |
| `else` | array of actions (optional) | no | Actions to execute if the condition is false |

**Behavior:**
- Evaluates the condition; executes `then` or `else` accordingly
- Supports variable substitution inside condition selectors and values via `${variable}`
- If `else` is omitted and the condition is false, no actions execute (no error)
- Propagates errors from condition evaluation (e.g., invalid selector) and child actions
- Does **not** clear the selector cache (read-only evaluation)

---

### Retry

Retries a block of actions on failure with exponential backoff. Useful for resilient
interactions where transient failures (network, animation, overlay) are expected.

**YAML:**
```yaml
- action: retry
  max_attempts: 3
  initial_delay_ms: 1000
  backoff_multiplier: 2.0
  max_delay_ms: 30000
  jitter: true
  actions:
    - action: click
      selector: "#submit-button"
    - action: wait
      duration_ms: 2000
```

**TOML:**
```toml
[[actions]]
action = "retry"
max_attempts = 3
initial_delay_ms = 1000
backoff_multiplier = 2.0
jitter = true
actions = [
  { action = "click", selector = "#submit-button" },
  { action = "wait", duration_ms = 2000 }
]
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `actions` | array of actions | yes | Actions to retry on failure |
| `max_attempts` | integer (optional) | no | Maximum retry attempts (default: 3) |
| `initial_delay_ms` | integer (optional) | no | Initial delay before first retry in ms (default: 1000) |
| `backoff_multiplier` | float (optional) | no | Exponential backoff multiplier (default: 2.0) |
| `max_delay_ms` | integer (optional) | no | Maximum delay between retries in ms (default: 30000) |
| `jitter` | bool (optional) | no | Add random jitter to prevent thundering herd (default: true) |
| `retry_on` | array of strings (optional) | no | Only retry on specific error substrings (default: retry all) |

**Behavior:**
- Retries the block on any error (or errors matching `retry_on` patterns)
- Uses exponential backoff: first retry waits `initial_delay_ms`, then multiplies by `backoff_multiplier` each attempt
- Jitter randomizes the delay to prevent synchronized retries in parallel tasks
- All actions in the block must succeed for the retry to succeed
- If all attempts fail, the last error is propagated

---

### Loop

Repeats a block of actions a fixed number of times, or while a condition is true, or both.

**YAML:**
```yaml
# Loop a fixed number of times
- action: loop
  count: 5
  actions:
    - action: click
      selector: ".next-page"

# Loop while a condition is true
- action: loop
  condition:
    element_exists:
      selector: ".load-more"
  actions:
    - action: click
      selector: ".load-more"
    - action: wait
      duration_ms: 1000
```

**TOML:**
```toml
[[actions]]
action = "loop"
count = 5
actions = [
  { action = "click", selector = ".next-page" }
]
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `actions` | array of actions | yes | Actions to repeat |
| `count` | integer (optional) | no | Fixed number of iterations (mutually exclusive with `condition`) |
| `condition` | condition (optional) | no | Condition evaluated before each iteration (mutually exclusive with `count`) |

**Behavior:**
- With `count`: iterates exactly that many times
- With `condition`: evaluates before each iteration, stops when false
- If both `count` and `condition` are omitted, loops forever (use with caution!)
- Errors in child actions propagate and stop the loop

---

### Foreach

Iterates over a collection, binding each value to a variable for use within the loop body.
Supports arrays, numeric ranges, DOM element selectors, and existing variables.

**YAML:**
```yaml
# Iterate over an array of values
- action: foreach
  variable: item
  collection:
    array:
      values: ["apple", "banana", "cherry"]
  actions:
    - action: type
      selector: "#search"
      text: "${item}"

# Iterate over a numeric range (0 to 4)
- action: foreach
  variable: index
  collection:
    range:
      start: 0
      end: 5
  actions:
    - action: log
      message: "Processing item ${index}"

# Iterate over DOM elements matching a selector
- action: foreach
  variable: link_id
  collection:
    elements:
      selector: "a.link"
  actions:
    - action: extract
      selector: "#${link_id}"
      variable: link_text
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `variable` | string | yes | Variable name to bind each iteration value |
| `collection` | collection | yes | Source of values: `array`, `range`, `elements`, or `variable` |
| `actions` | array of actions | yes | Actions to execute for each iteration |
| `max_iterations` | integer (optional) | no | Safety limit on iterations (default: 100) |

**Collection types:**
- `array`: `{ values: [...] }` — iterates over a static array
- `range`: `{ start: N, end: N }` — iterates over an integer range (start inclusive, end exclusive)
- `elements`: `{ selector: "..." }` — iterates over DOM elements, binding each element's ID or index
- `variable`: `{ name: "..." }` — iterates over a variable containing an array

---

### While

Repeats a block of actions while a condition is true. The condition is evaluated before
each iteration. Includes a safety limit on total iterations.

**YAML:**
```yaml
# Wait for a spinner to disappear
- action: while
  condition:
    element_visible:
      selector: "#loading-spinner"
  actions:
    - action: wait
      duration_ms: 500
  max_iterations: 60
```

**TOML:**
```toml
[[actions]]
action = "while"
condition = { element_visible = { selector = "#loading-spinner" } }
actions = [
  { action = "wait", duration_ms = 500 }
]
max_iterations = 60
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `condition` | condition | yes | Condition evaluated before each iteration |
| `actions` | array of actions | yes | Actions to execute while the condition is true |
| `max_iterations` | integer (optional) | no | Maximum iterations before forced exit (default: 1000, safety limit) |

**Behavior:**
- Evaluates the condition before each iteration; stops when false
- If the condition is initially false, no actions execute (zero iterations)
- `max_iterations` prevents infinite loops (default 1000)
- Errors in child actions propagate and stop the loop

---

### Parallel

Executes multiple child actions concurrently. Useful for independent operations that
can run simultaneously (e.g., clicking multiple buttons, filling multiple fields).

**YAML:**
```yaml
- action: parallel
  max_concurrency: 2
  actions:
    - action: click
      selector: "#tab1"
    - action: click
      selector: "#tab2"
    - action: click
      selector: "#tab3"
```

**TOML:**
```toml
[[actions]]
action = "parallel"
max_concurrency = 2
actions = [
  { action = "click", selector = "#tab1" },
  { action = "click", selector = "#tab2" },
  { action = "click", selector = "#tab3" }
]
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `actions` | array of actions | yes | Actions to execute concurrently |
| `max_concurrency` | integer (optional) | no | Maximum concurrent executions (default: all at once) |

**Behavior:**
- All child actions run concurrently using tokio tasks
- If any child action fails, the remaining are cancelled and the error is propagated
- `max_concurrency` limits how many run simultaneously (helps with rate limiting)
- Each child runs in its own variable scope — variables set in one child are not visible to others

---

### Call

Invokes another named DSL task, passing optional parameters. Returned variables from
the called task are merged back into the calling task's variable scope.

**YAML:**
```yaml
- action: call
  task: "login"
  parameters:
    username: "admin"
    password: "secret123"

# Call with variable substitution
- action: call
  task: "${subtask}"
  parameters:
    url: "${base_url}/api"
```

**TOML:**
```toml
[[actions]]
action = "call"
task = "login"
parameters = { username = "admin", password = "secret123" }
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `task` | string | yes | Name of the task to invoke (must be registered) |
| `parameters` | map (optional) | no | Parameters to pass to the called task |

**Behavior:**
- Parameters support `${variable}` substitution from the calling scope
- Variables created or modified in the called task are copied back to the caller
- Recursion depth is limited to prevent infinite cycles (max 10 levels)
- The called task inherits the caller's selector cache TTL
- Supports task composition : build complex workflows from reusable subtasks

---

### Try

Executes a block of actions and catches errors gracefully, similar to try/catch/finally
in programming languages. Optionally stores the error message in a variable.

**YAML:**
```yaml
- action: try
  try_actions:
    - action: click
      selector: "#submit"
    - action: wait_for
      selector: ".success"
      timeout_ms: 5000
  catch_actions:
    - action: log
      message: "Submission failed: ${error}"
      level: error
    - action: screenshot
      path: "error.png"
  error_variable: error
  finally_actions:
    - action: log
      message: "Submission attempt completed"
```

**TOML:**
```toml
[[actions]]
action = "try"
error_variable = "error"
try_actions = [
  { action = "click", selector = "#submit" },
  { action = "wait_for", selector = ".success", timeout_ms = 5000 }
]
catch_actions = [
  { action = "log", message = "Submission failed: ${error}", level = "error" },
  { action = "screenshot", path = "error.png" }
]
finally_actions = [
  { action = "log", message = "Submission attempt completed" }
]
```

**Fields:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `try_actions` | array of actions | yes | Actions to attempt (the `try` block) |
| `catch_actions` | array of actions (optional) | no | Actions to execute if an error occurs (the `catch` block) |
| `error_variable` | string (optional) | no | Variable name to store the error message |
| `finally_actions` | array of actions (optional) | no | Actions that always execute (the `finally` block) |

**Behavior:**
- If `try_actions` succeed, `catch_actions` are skipped (but `finally_actions` always run)
- If `try_actions` fail, `catch_actions` run with the error stored in `error_variable`
- `finally_actions` always execute, regardless of success or failure
- If `catch_actions` are omitted and `try_actions` fail, the error propagates to the parent
- Supports variable substitution in all fields

---

## Conditions

Conditions are used by the [If/Else](#ifelse), [Loop](#loop), and [While](#while) actions to make decisions. Each condition is defined as a map with a single key identifying the condition type.

### Boolean conditions

Simple `true` or `false` values. Useful for debugging or forcing a branch.

**YAML:**
```yaml
condition: true   # Always true
condition: false  # Always false
```

### Element conditions

Check the state of a DOM element.

| Condition | Fields | Description |
|-----------|--------|-------------|
| `element_exists` | `{ selector: string }` | True if the element exists in the DOM |
| `element_visible` | `{ selector: string }` | True if the element is visible (exists and not hidden) |

**YAML:**
```yaml
condition:
  element_exists:
    selector: ".search-result"
condition:
  element_visible:
    selector: "#loading-spinner"
```

### Text conditions

Check the text content of a DOM element.

| Condition | Fields | Description |
|-----------|--------|-------------|
| `text_equals` | `{ selector: string, value: string }` | True if the element's text exactly matches `value` |
| `text_matches` | `{ selector: string, pattern: string }` | True if the element's text matches the regex `pattern` |

**YAML:**
```yaml
condition:
  text_equals:
    selector: "#status"
    value: "Complete"
condition:
  text_matches:
    selector: ".price"
    pattern: "^\\$[0-9]+\\."
```

### Variable conditions

Check the state of task variables.

| Condition | Fields | Description |
|-----------|--------|-------------|
| `variable_equals` | `{ name: string, value: any }` | True if the variable equals the given value |
| `variable_matches` | `{ name: string, pattern: string }` | True if the variable matches the regex `pattern` |
| `variable_defined` | `{ name: string }` | True if the variable is defined |
| `variable_not_defined` | `{ name: string }` | True if the variable is not defined |

**YAML:**
```yaml
condition:
  variable_equals:
    name: "status"
    value: "ready"
condition:
  variable_defined:
    name: "error_text"
```

### Numeric conditions

Compare numeric variable values.

| Condition | Fields | Description |
|-----------|--------|-------------|
| `numeric_greater_than` | `{ name: string, value: float }` | True if the variable's numeric value > `value` |
| `numeric_less_than` | `{ name: string, value: float }` | True if the variable's numeric value < `value` |
| `numeric_range` | `{ name: string, min: float, max: float }` | True if the variable is within [min, max] (inclusive) |

**YAML:**
```yaml
condition:
  numeric_greater_than:
    name: "count"
    value: 10
condition:
  numeric_range:
    name: "score"
    min: 0
    max: 100
```

### Compound conditions

Combine or negate other conditions.

| Condition | Fields | Description |
|-----------|--------|-------------|
| `and` | `{ conditions: [condition, ...] }` | True if all sub-conditions are true |
| `or` | `{ conditions: [condition, ...] }` | True if any sub-condition is true |
| `not` | `{ condition: condition }` | Inverts the sub-condition (true becomes false) |

**YAML:**
```yaml
condition:
  and:
    conditions:
      - element_exists:
          selector: ".result"
      - text_equals:
          selector: ".status"
          value: "Complete"

condition:
  not:
    condition:
      element_exists:
        selector: ".error"
```

## End-to-end example: Login flow

The following task demonstrates a realistic login scenario combining navigation, form filling,
conditional logic, error resilience, and result extraction.

**YAML:**
```yaml
name: login_flow
description: "Log in, verify success, handle errors gracefully"
policy: default

parameters:
  base_url:
    type: string
    default: "https://example.com"
  username:
    type: string
    default: "admin"
  password:
    type: string
    default: "secret123"

actions:
  # ── Step 1: Navigate to login page ──────────────────────────────────────
  - action: navigate
    url: "${base_url}/login"

  - action: log
    message: "Navigated to login page: ${base_url}/login"

  # ── Step 2: Wait for the login form to appear ────────────────────────────
  - action: wait_for
    selector: "#login-form"
    timeout_ms: 10000

  # ── Step 3: Fill in credentials ──────────────────────────────────────────
  - action: clear
    selector: "#username"

  - action: type
    selector: "#username"
    text: "${username}"

  - action: clear
    selector: "#password"

  - action: type
    selector: "#password"
    text: "${password}"

  - action: log
    message: "Credentials entered for user: ${username}"
    level: debug

  # ── Step 4: Click login (with retry on failure) ───────────────────────────
  - action: retry
    max_attempts: 3
    initial_delay_ms: 1000
    backoff_multiplier: 2.0
    actions:
      - action: click
        selector: "#login-button"

      - action: wait
        duration_ms: 2000

  # ── Step 5: Check login result ───────────────────────────────────────────
  - action: if
    condition:
      element_exists:
        selector: ".welcome-message"
    then:
      # Success path: extract welcome text
      - action: extract
        selector: ".welcome-message"
        variable: welcome_text

      - action: log
        message: "Login successful — ${welcome_text}"
        level: info

      - action: screenshot
        path: "results/welcome.png"
        selector: ".welcome-message"
    else:
      # Failure path: capture error and take diagnostic screenshot
      - action: extract
        selector: ".error-message"
        variable: error_text

      - action: log
        message: "Login failed — ${error_text}"
        level: error

      - action: screenshot
        path: "results/error.png"

      - action: execute
        script: "document.title"
```

**Same task in TOML:**
```toml
name = "login_flow"
description = "Log in, verify success, handle errors gracefully"
policy = "default"

[parameters]
[parameters.base_url]
type = "string"
default = "https://example.com"

[parameters.username]
type = "string"
default = "admin"

[parameters.password]
type = "string"
default = "secret123"

[[actions]]
action = "navigate"
url = "${base_url}/login"

[[actions]]
action = "log"
message = "Navigated to login page: ${base_url}/login"

[[actions]]
action = "wait_for"
selector = "#login-form"
timeout_ms = 10000

[[actions]]
action = "clear"
selector = "#username"

[[actions]]
action = "type"
selector = "#username"
text = "${username}"

[[actions]]
action = "clear"
selector = "#password"

[[actions]]
action = "type"
selector = "#password"
text = "${password}"

[[actions]]
action = "log"
message = "Credentials entered for user: ${username}"
level = "debug"

[[actions]]
action = "retry"
max_attempts = 3
initial_delay_ms = 1000
backoff_multiplier = 2.0
actions = [
  { action = "click", selector = "#login-button" },
  { action = "wait", duration_ms = 2000 }
]

[[actions]]
action = "if"
condition = { element_exists = { selector = ".welcome-message" } }
then = [
  { action = "extract", selector = ".welcome-message", variable = "welcome_text" },
  { action = "log", message = "Login successful — ${welcome_text}" },
  { action = "screenshot", path = "results/welcome.png", selector = ".welcome-message" }
]
else = [
  { action = "extract", selector = ".error-message", variable = "error_text" },
  { action = "log", message = "Login failed — ${error_text}", level = "error" },
  { action = "screenshot", path = "results/error.png" },
  { action = "execute", script = "document.title" }
]
```

> **Note:** TOML inline tables work well for simple nested actions. For deeply nested or
> multi-branch control flow (e.g., `retry` containing `if/else`), prefer YAML which
> supports hierarchical nesting more naturally.

**What this task demonstrates:**

| Concept | How it's shown |
|---------|----------------|
| **Parameters** | `base_url`, `username`, `password` with defaults |
| **Variable substitution** | `${base_url}`, `${username}`, `${password}` used across all fields |
| **Task composition** | `wait_for`, `retry`, `if/else` nesting actions inside control flow |
| **Form interaction** | `clear` before `type` to ensure clean fields |
| **Error resilience** | `retry` around the login click with exponential backoff |
| **Conditional branching** | `if element_exists` to choose success or failure path |
| **Data extraction** | `extract` stores welcome text or error message into variables |
| **Diagnostic logging** | `log` at debug/info/error levels throughout the flow |
| **Screenshots** | On success captures the welcome element; on failure captures the full error page |
| **Fallback diagnostics** | `execute` reads `document.title` as a last-resort diagnostic |

## Tutorial: Building a custom task from scratch

This tutorial walks through creating a DSL task step by step. We'll build a task that
searches a website and captures the first result. Each step introduces one new concept.

### Step 1: Start with the skeleton

Every task needs a name, description, and policy. The actions array starts empty:

```yaml
name: search_tutorial
description: "Search example.com and capture the first result"
policy: default

actions: []
```

### Step 2: Add navigation

A task that does nothing isn't useful. Add a `navigate` action to go to the search page:

```yaml
actions:
  - action: navigate
    url: "https://example.com/search"
```

> **Reference:** See the [Navigate](#navigate) action docs for field details.

### Step 3: Wait for the page to load

After navigating, wait for the search form to appear. Using `wait_for` with a selector
is more reliable than a fixed `wait`:

```yaml
actions:
  - action: navigate
    url: "https://example.com/search"

  - action: wait_for
    selector: "#search-form"
    timeout_ms: 10000
```

> **Reference:** The `timeout_ms` field defaults to 5000ms if omitted. See the [Wait](#wait) section for field details, and the [WaitFor](#waitfor) action for selector-based waiting.

### Step 4: Interact with the page

Clear any pre-filled text, then type a search query. Use `clear` before `type` to
ensure the field starts empty:

```yaml
actions:
  - action: navigate
    url: "https://example.com/search"

  - action: wait_for
    selector: "#search-form"
    timeout_ms: 10000

  - action: clear
    selector: "#search-input"

  - action: type
    selector: "#search-input"
    text: "Rust programming"

  - action: click
    selector: "#search-button"
```

> **Reference:** See [Clear](#clear), [Type](#type), and [Click](#click) for field details and behavior.

### Step 5: Wait for results and extract data

After clicking search, wait for results to appear, then extract the first result's text
into a variable. Use `wait` to give the page time to respond:

```yaml
actions:
  - action: navigate
    url: "https://example.com/search"

  - action: wait_for
    selector: "#search-form"
    timeout_ms: 10000

  - action: clear
    selector: "#search-input"

  - action: type
    selector: "#search-input"
    text: "Rust programming"

  - action: click
    selector: "#search-button"

  - action: wait
    duration_ms: 3000

  - action: wait_for
    selector: ".search-result"
    timeout_ms: 15000

  - action: extract
    selector: ".search-result:first-child"
    variable: first_result
```

Now you can use `${first_result}` in later actions — for example, to log it or type it
into another field.

> **Reference:** See [Extract](#extract) for how variables are stored and reused.

### Step 6: Add diagnostic logging

Log key milestones so you can trace execution in the logs:

```yaml
actions:
  - action: log
    message: "Starting search for: Rust programming"
    level: info

  - action: navigate
    url: "https://example.com/search"

  - action: log
    message: "Navigated to search page"
    level: debug

  - action: wait_for
    selector: "#search-form"
    timeout_ms: 10000

  - action: clear
    selector: "#search-input"

  - action: type
    selector: "#search-input"
    text: "Rust programming"

  - action: click
    selector: "#search-button"

  - action: wait
    duration_ms: 3000

  - action: wait_for
    selector: ".search-result"
    timeout_ms: 15000

  - action: extract
    selector: ".search-result:first-child"
    variable: first_result

  - action: log
    message: "First result: ${first_result}"
    level: info
```

> **Reference:** Log levels: `debug`, `info` (default), `warn`, `error`. See [Log](#log).

### Step 7: Add error resilience

Use `retry` around the search button click, and use `if/else` to handle the case where
no results are found:

```yaml
actions:
  - action: log
    message: "Starting search for: Rust programming"
    level: info

  - action: navigate
    url: "https://example.com/search"

  - action: wait_for
    selector: "#search-form"
    timeout_ms: 10000

  - action: clear
    selector: "#search-input"

  - action: type
    selector: "#search-input"
    text: "Rust programming"

  # Retry the click up to 3 times if it fails
  - action: retry
    max_attempts: 3
    initial_delay_ms: 1000
    backoff_multiplier: 2.0
    actions:
      - action: click
        selector: "#search-button"

      - action: wait
        duration_ms: 2000

  - action: if
    condition:
      element_exists:
        selector: ".search-result"
    then:
      - action: extract
        selector: ".search-result:first-child"
        variable: first_result

      - action: log
        message: "Found result: ${first_result}"
        level: info

      - action: screenshot
        selector: ".search-result:first-child"
    else:
      - action: log
        message: "No search results found"
        level: warn

      - action: screenshot
        path: "results/no_results.png"
```

> **Reference:** See the [Retry](#retry) action for backoff options and the [If/Else](#ifelse) action for condition types.

### Step 8: Extract parameters for reuse

Replace hardcoded values with task parameters so the task can be reused with different
search queries:

```yaml
name: search_tutorial
description: "Search example.com and capture the first result"
policy: default

parameters:
  search_url:
    type: string
    default: "https://example.com/search"
  query:
    type: string
    default: "Rust programming"
  timeout_ms:
    type: integer
    default: 10000

actions:
  - action: log
    message: "Starting search for: ${query}"
    level: info

  - action: navigate
    url: "${search_url}"

  - action: wait_for
    selector: "#search-form"
    timeout_ms: ${timeout_ms}

  - action: clear
    selector: "#search-input"

  - action: type
    selector: "#search-input"
    text: "${query}"

  - action: retry
    max_attempts: 3
    initial_delay_ms: 1000
    backoff_multiplier: 2.0
    actions:
      - action: click
        selector: "#search-button"

      - action: wait
        duration_ms: 2000

  - action: if
    condition:
      element_exists:
        selector: ".search-result"
    then:
      - action: extract
        selector: ".search-result:first-child"
        variable: first_result

      - action: log
        message: "Found result: ${first_result}"
        level: info

      - action: screenshot
        selector: ".search-result:first-child"
    else:
      - action: log
        message: "No search results found for: ${query}"
        level: warn

      - action: screenshot
        path: "results/no_results.png"
```

> **Reference:** Parameters are defined in the `parameters` section. Each parameter has
> a `type` (`string`, `integer`, `boolean`) and an optional `default`. See the [full login example](#end-to-end-example-login-flow) for more parameter patterns.

### Next steps

You now have a parameterized, resilient search task. From here you could:

- **Chain tasks** using the [`call`](#call) action to compose smaller tasks into larger workflows
- **Loop over results** using [`foreach`](#foreach) to process multiple search results
- **Add more conditions** — `element_exists` is one of several [condition types](#conditions) available
- **Run in parallel** using the [`parallel`](#parallel) action to perform independent actions concurrently

Refer back to the [Action Reference](#action-reference) for the full list of available actions
and their fields.

## Glossary

| Term | Definition |
|------|------------|
| **Action** | A single operation in a DSL task (e.g., `click`, `navigate`, `type`). Each action is identified by an `action` field and has its own set of fields. Some actions (control flow) nest other actions inside them. |
| **Backoff multiplier** | The exponential growth factor for retry delays. Each retry multiplies the previous delay by this value. Default is 2.0. See [Retry](#retry). |
| **Cache TTL** | Time-to-live for [selector cache](#selector-cache) entries. Controls how long a cached element state is considered valid before it must be rechecked. Default varies by configuration. |
| **Call depth** | How deeply nested [`call`](#call) invocations are. The maximum call depth is 10, enforced to prevent infinite recursion. Each nested call increments the depth by 1. |
| **Condition** | An expression evaluated to true or false, used in [If/Else](#ifelse), [Loop](#loop), and [While](#while) actions. Conditions can check DOM state (element exists, visible), text content, variable values, or be combined with `and`/`or`/`not`. |
| **Control flow** | Actions that manage execution order by nesting other actions inside them: [If/Else](#ifelse), [Retry](#retry), [Loop](#loop), [Foreach](#foreach), [While](#while), [Parallel](#parallel), [Call](#call), [Try](#try). |
| **DSL** | Declarative Scripting Language — a YAML/TOML format for defining browser automation tasks without writing Rust code. Task files declare a sequence of actions instead of imperative logic. |
| **Exponential backoff** | A retry strategy where the delay between attempts increases exponentially (e.g., 1s, 2s, 4s, 8s). Used by [Retry](#retry) to avoid hammering the server on transient failures. |
| **Jitter** | A small random variation added to retry delays to prevent multiple parallel tasks from retrying in sync (the "thundering herd" problem). Enabled by default in [Retry](#retry). |
| **Parameters** | Configurable values defined at the task level with a `type` (`string`, `integer`, `boolean`) and optional `default`. Overridable when a task is called via [`call`](#call) with a `parameters` map. |
| **Policy** | A named security and behavior profile applied to a task, controlling which browser APIs and capabilities the task can access (e.g., `default`, `restricted`). |
| **Selector cache** | An in-memory cache that stores whether DOM elements exist and are visible. Avoids redundant browser API calls when the same selector is checked multiple times. Cleared by mutating actions (click, type, etc.) but preserved by read-only actions (extract, wait_for). |
| **Serde tag** | The `action` field used to identify which Rust `Action` enum variant a YAML/TOML entry corresponds to (e.g., `action: click` maps to `Action::Click`). All action types use `#[serde(tag = "action", rename_all = "snake_case")]` for deserialization. |
| **Task definition** | The top-level YAML/TOML structure containing `name`, `description`, `policy`, optional `parameters`, optional `include`, and `actions`. Defines a complete, reusable automation workflow. |
| **Variable scope** | The visibility and lifetime of variables within nested control flow and [`call`](#call) tasks. Variables set in a parent scope are visible to children. Variables created in a child block are copied back to the parent on return. Parallel actions have isolated scopes. |
| **Variable substitution** | The `${variable_name}` syntax for referencing task [parameters](#parameters), extracted values, and loop variables within action field values. Resolved at execution time before the API call is made. Supports interpolation (e.g., `${base_url}/login`). |

## Common Risks

- Parser and executor drift
- Validation accepting an action the executor cannot run
- Scope leaks across nested task calls
- Confusing defaults for control flow or retries

## Best Practice

Update the docs and tests together when DSL behavior changes.

