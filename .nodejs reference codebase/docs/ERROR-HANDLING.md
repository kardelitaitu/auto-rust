# Error Handling Guide

Complete guide to understanding and handling errors in Auto-AI.

---

## Table of Contents

1. [Error Types](#error-types)
2. [Error Structure](#error-structure)
3. [Basic Error Handling](#basic-error-handling)
4. [Error Suggestions](#error-suggestions)
5. [Common Errors](#common-errors)
6. [Enhanced Errors](#enhanced-errors)
7. [Best Practices](#best-practices)

---

## Error Types

Auto-AI uses a hierarchical error system:

```
AutomationError (base)
├── SessionError
│   ├── SessionDisconnectedError
│   ├── SessionClosedError
│   ├── SessionNotFoundError
│   └── SessionTimeoutError
├── ContextError
│   ├── ContextNotInitializedError
│   └── PageClosedError
├── ElementError
│   ├── ElementNotFoundError
│   ├── ElementDetachedError
│   ├── ElementObscuredError
│   └── ElementTimeoutError
├── ActionError
│   ├── ActionFailedError
│   ├── NavigationError
│   └── TaskTimeoutError
├── ConfigError
│   └── ConfigNotFoundError
├── LLMError
│   ├── LLMTimeoutError
│   ├── LLMRateLimitError
│   └── LLMCircuitOpenError
└── ValidationError
```

---

## Error Structure

All Auto-AI errors have this structure:

```javascript
{
    name: 'ElementNotFoundError',
    code: 'ELEMENT_NOT_FOUND',
    message: 'Element not found: .button',
    metadata: { selector: '.button' },
    timestamp: '2026-03-31T10:00:00.000Z',
    stack: '...',
    cause: null  // Original error if wrapped
}
```

### Enhanced Errors

Enhanced errors include suggestions:

```javascript
{
    ...baseError,
    suggestions: [
        'Verify selector is correct',
        'Wait for element first',
        'Check for iframe context'
    ],
    docs: 'docs/RECIPES.md#error-handling',
    severity: 'medium'
}
```

---

## Basic Error Handling

### Try-Catch Pattern

```javascript
import { api } from './api/index.js';
import { ElementNotFoundError } from './api/core/errors.js';

await api.withPage(page, async () => {
    await api.init(page);
    
    try {
        await api.click('.button');
    } catch (error) {
        if (error instanceof ElementNotFoundError) {
            console.log('Button not found, using alternative...');
            await api.click('.alt-button');
        } else {
            throw error;  // Re-throw unknown errors
        }
    }
});
```

### Error Code Checking

```javascript
import { isErrorCode } from './api/core/errors.js';

try {
    await api.click('.button');
} catch (error) {
    if (isErrorCode(error, 'ELEMENT_NOT_FOUND')) {
        // Handle missing element
    } else if (isErrorCode(error, 'TIMEOUT')) {
        // Handle timeout
    }
}
```

### Error with Metadata

```javascript
try {
    await api.click('.button');
} catch (error) {
    console.error('Error:', error.message);
    console.error('Code:', error.code);
    console.error('Metadata:', error.metadata);
    console.error('Stack:', error.stack);
}
```

---

## Error Suggestions

### Using Error Suggestions Module

```javascript
import { getSuggestionsForError, formatSuggestions } 
    from './api/utils/error-suggestions.js';

try {
    await api.click('.button');
} catch (error) {
    const suggestions = getSuggestionsForError(error);
    
    if (suggestions) {
        console.log(formatSuggestions(suggestions));
        // Output:
        // Issue: Element not found
        // Suggestions:
        //   1. Verify selector is correct
        //   2. Wait for element first
        //   ...
    }
}
```

### Auto-Suggestions on Errors

```javascript
import { enhanceError } from './api/core/errors-enhanced.js';

try {
    await api.click('.button');
} catch (error) {
    const enhanced = enhanceError(error);
    enhanced.print();  // Prints error with suggestions
}
```

---

## Common Errors

### Element Not Found

```javascript
// Error: ElementNotFoundError
// Code: ELEMENT_NOT_FOUND

// Solution 1: Wait for element
await api.wait.forElement('.button');
await api.click('.button');

// Solution 2: Check existence first
if (await api.exists('.button')) {
    await api.click('.button');
}

// Solution 3: Use retry
await api.click('.button', { retries: 3, delay: 500 });
```

### Context Not Initialized

```javascript
// Error: ContextNotInitializedError
// Code: CONTEXT_NOT_INITIALIZED

// Wrong - no context
await api.click('.button');  // ❌

// Correct - with context
await api.withPage(page, async () => {  // ✅
    await api.init(page);
    await api.click('.button');
});
```

### Browser Not Found

```javascript
// Error: BrowserNotFoundError
// Code: BROWSER_NOT_FOUND

// Solution: Start browser with remote debugging
// Chrome/Brave:
chrome.exe --remote-debugging-port=9222

// ixBrowser:
// Settings → Remote Debugging → Port 53200
```

### LLM Timeout

```javascript
// Error: LLMTimeoutError
// Code: LLM_TIMEOUT

// Solution 1: Check Ollama is running
ollama serve

// Solution 2: Use smaller model
// config/settings.json:
{ "llm": { "local": { "model": "hermes3:8b" } } }

// Solution 3: Enable fallback
{ "llm": { "fallback": { "enabled": true } } }
```

### Navigation Error

```javascript
// Error: NavigationError
// Code: NAVIGATION_ERROR

// Solution: Handle navigation failures
try {
    await api.goto('https://example.com');
} catch (error) {
    if (error.code === 'NAVIGATION_ERROR') {
        console.log('Navigation failed, retrying...');
        await api.wait(2000);
        await api.goto('https://example.com');
    }
}
```

---

## Enhanced Errors

### Using Enhanced Error Classes

```javascript
import { 
    EnhancedElementNotFoundError,
    EnhancedBrowserNotFoundError,
    enhanceError
} from './api/core/errors-enhanced.js';

// Throw enhanced error with suggestions
throw new EnhancedElementNotFoundError('.button', {
    metadata: { page: 'home' }
});

// Enhance existing error
try {
    someOperation();
} catch (error) {
    throw enhanceError(error, {
        context: 'user registration',
        severity: 'high'
    });
}
```

### Enhanced Error Output

```javascript
// Enhanced error prints with suggestions
const error = new EnhancedElementNotFoundError('.submit-btn');
error.print();

// Output:
// [ELEMENT_NOT_FOUND] Element not found: .submit-btn
//
// Suggestions:
//   1. Verify selector is correct: check for typos
//   2. Wait for element: await api.wait.forElement('.submit-btn')
//   3. Check if element is inside an iframe
//   4. Verify page has fully loaded
//   5. Try alternative selectors (CSS, XPath, text)
//
// Documentation: docs/RECIPES.md#error-handling
// Severity: medium
```

---

## Best Practices

### 1. Always Use Try-Catch for External Operations

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    
    try {
        await api.goto(url);
        await api.click('.action');
    } catch (error) {
        logger.error('Automation failed', error);
        await api.screenshot.save('error-state.png');
        throw error;
    }
});
```

### 2. Use Specific Error Handling

```javascript
// Good - specific handling
try {
    await api.click('.submit');
} catch (error) {
    if (error.code === 'ELEMENT_NOT_FOUND') {
        // Handle missing element
    } else if (error.code === 'TIMEOUT') {
        // Handle timeout
    } else {
        throw error;
    }
}

// Bad - catch all silently
try {
    await api.click('.submit');
} catch (error) {
    // Swallowing errors hides problems
}
```

### 3. Add Context to Errors

```javascript
try {
    await api.click('.submit');
} catch (error) {
    throw new ActionError(
        'FORM_SUBMIT_FAILED',
        `Failed to submit form: ${error.message}`,
        { form: 'registration', step: 'final' },
        error
    );
}
```

### 4. Use Enhanced Errors for User-Facing Messages

```javascript
import { withEnhancedError } from './api/core/errors-enhanced.js';

// Wrap operations with enhanced error handling
const result = await withEnhancedError(
    () => api.click('.submit'),
    'form submission',
    { severity: 'high' }
);
```

### 5. Log Errors with Full Context

```javascript
try {
    await automationTask();
} catch (error) {
    logger.error({
        message: 'Automation failed',
        error: error.message,
        code: error.code,
        stack: error.stack,
        metadata: error.metadata,
        timestamp: error.timestamp
    });
}
```

### 6. Implement Retry Logic

```javascript
async function clickWithRetry(selector, maxRetries = 3) {
    for (let i = 0; i < maxRetries; i++) {
        try {
            return await api.click(selector);
        } catch (error) {
            if (i === maxRetries - 1) throw error;
            
            // Exponential backoff
            await api.wait(1000 * Math.pow(2, i));
        }
    }
}
```

### 7. Graceful Degradation

```javascript
async function engageWithContent() {
    try {
        await api.click('.primary-action');
    } catch (error) {
        if (error.code === 'ELEMENT_NOT_FOUND') {
            // Fallback to alternative action
            await api.click('.secondary-action');
        } else {
            throw error;
        }
    }
}
```

---

## Error Reference

| Error Code | Severity | Common Causes | Quick Fix |
|------------|----------|---------------|-----------|
| `BROWSER_NOT_FOUND` | High | Browser not running | Start with --remote-debugging-port |
| `CONNECTION_TIMEOUT` | High | Firewall, wrong port | Check port config |
| `CONTEXT_NOT_INITIALIZED` | High | Missing withPage() | Wrap in api.withPage() |
| `ELEMENT_NOT_FOUND` | Medium | Wrong selector, timing | Wait for element |
| `ELEMENT_TIMEOUT` | Medium | Slow page load | Increase timeout |
| `ACTION_FAILED` | Medium | Element state issue | Retry with backoff |
| `NAVIGATION_ERROR` | High | Invalid URL, network | Verify URL |
| `LLM_TIMEOUT` | High | LLM not running | Start Ollama |
| `LLM_RATE_LIMIT` | Medium | Too many requests | Add delays |
| `SESSION_DISCONNECTED` | High | Browser crashed | Restart browser |

---

## Related Documentation

- [`api/core/errors.js`](../api/core/errors.js) - Base error classes
- [`api/core/errors-enhanced.js`](../api/core/errors-enhanced.js) - Enhanced errors
- [`api/utils/error-suggestions.js`](../api/utils/error-suggestions.js) - Suggestion database
- [docs/troubleshooting.md](troubleshooting.md) - Troubleshooting guide
- [docs/RECIPES.md](RECIPES.md) - Error handling patterns

---

*Last updated: 2026-03-31*
