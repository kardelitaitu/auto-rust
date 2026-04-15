# API Cheatsheet

Quick reference for Auto-AI API methods and patterns.

---

## Quick Start Pattern

```javascript
import { api } from './api/index.js';

await api.withPage(page, async () => {
    await api.init(page, { persona: 'casual' });
    await api.goto('https://example.com');
    // ... your actions
});
```

**Always use `api.withPage()`** for session isolation.

---

## Navigation

```javascript
// Navigate to URL
await api.goto('https://example.com');

// Navigate with wait
await api.goto('https://example.com', { waitUntil: 'networkidle' });

// Go back
await api.goBack();

// Go forward
await api.goForward();

// Refresh page
await api.reload();
```

---

## Clicking

```javascript
// Basic click
await api.click('.button');

// Click with options
await api.click('#submit', {
    delay: 500,  // Wait before click
    retries: 3   // Retry on failure
});

// Click at coordinates
await api.clickAt(100, 200);

// Double click
await api.dblclick('.item');

// Right click
await api.rightClick('.menu');
```

---

## Typing

```javascript
// Basic typing
await api.type('input[name="email"]', 'user@example.com');

// Typing with delay
await api.type('textarea', 'Hello world', {
    delay: 100  // ms between keystrokes
});

// Clear and type
await api.fill('input', 'new value');

// Press keys
await api.press('Enter');
await api.press('Tab');
await api.press('Control+A');
```

---

## Scrolling

```javascript
// Scroll down
await api.scroll.down(500);  // pixels

// Scroll up
await api.scroll.up(300);

// Scroll to element
await api.scroll.toElement('.target');

// Scroll to top
await api.scroll.toTop();

// Scroll to bottom
await api.scroll.toBottom();

// Natural reading scroll
await api.scroll.read();  // Simulates reading pattern
```

---

## Waiting

```javascript
// Wait fixed time
await api.wait(2000);  // ms

// Wait for element
await api.wait.forElement('.loaded');

// Wait for element hidden
await api.wait.forElementHidden('.spinner');

// Wait for navigation
await api.wait.forNavigation();

// Wait for network idle
await api.wait.forNetworkIdle();

// Wait for text
await api.wait.forText('Success');
```

---

## Finding Elements

```javascript
// Find by selector
const element = await api.find('.button');

// Find by text
const element = await api.findByText('Click me');

// Find by role
const element = await api.find.byRole('button');

// Find all
const elements = await api.findAll('.item');

// Check if exists
const exists = await api.exists('.optional');
```

---

## Screenshots

```javascript
// Full page screenshot
await api.screenshot.full();

// Visible area only
await api.screenshot.visible();

// Element screenshot
await api.screenshot.element('.component');

// Save to file
await api.screenshot.save('output.png');
```

---

## Session Management

```javascript
// Initialize with persona
await api.init(page, {
    persona: 'casual',
    humanizationPatch: true
});

// Get current session
const session = api.getSession();

// Set persona
api.setPersona('focused');

// Cleanup
await api.cleanup();
```

### Available Personas

| Persona | Description | Speed |
|---------|-------------|-------|
| `casual` | Relaxed browsing | Normal |
| `focused` | Task-oriented | Fast |
| `careful` | Cautious, slow | Slow |
| `stealth` | Anti-detection | Very Slow |

---

## Game Agent (OWB)

```javascript
import { gameRunner } from './api/agent/gameRunner.js';

// Auto-play mode
await gameRunner.run('Build army', {
    maxSteps: 50,
    stepDelay: 500,
    stuckDetection: true
});

// Strategy modes
await gameRunner.run('rush');      // Fast attack
await gameRunner.run('turtle');    // Defensive
await gameRunner.run('economy');   // Resource focus
await gameRunner.run('balanced');  // Mixed
```

---

## Error Handling

```javascript
try {
    await api.withPage(page, async () => {
        await api.click('.button');
    });
} catch (error) {
    if (error.code === 'ELEMENT_NOT_FOUND') {
        // Handle missing element
    } else if (error.code === 'TIMEOUT') {
        // Handle timeout
    }
}
```

### Common Error Codes

| Code | Description |
|------|-------------|
| `ELEMENT_NOT_FOUND` | Selector didn't match |
| `TIMEOUT` | Operation exceeded timeout |
| `SESSION_DISCONNECTED` | Browser disconnected |
| `CONTEXT_NOT_INITIALIZED` | withPage() not called |

---

## Task Chaining

```bash
# CLI chaining
node main.js pageview=url then like=tweetUrl then retweet=tweetUrl

# Programmatic
await api.withPage(page, async () => {
    await api.goto('https://example.com');
    await api.wait(1000);
    await api.click('.action');
    await api.scroll.down(500);
});
```

---

## Humanization Options

```javascript
await api.init(page, {
    persona: 'casual',
    
    // Mouse movement
    mouse: {
        enabled: true,
        speed: 2.0,
        jitter: true
    },
    
    // Keystroke dynamics
    keyboard: {
        enabled: true,
        delay: 120,
        punctuationPause: 300
    },
    
    // Idle behavior
    idle: {
        enabled: true,
        wiggleFrequency: 2000
    }
});
```

---

## Selector Tips

```javascript
// CSS selectors
'.class'
'#id'
'div > p'
'input[name="email"]'

// XPath (use with api.find)
'//button[text()="Submit"]'
'//div[@class="container"]'

// Text-based
api.findByText('Click me')
api.find.byRole('button')

// Best practices:
// - Prefer data-testid attributes
// - Use specific selectors
// - Avoid brittle XPaths
```

---

## Complete Example

```javascript
import { api } from './api/index.js';

async function automate() {
    const browser = await playwright.chromium.connect({
        wsEndpoint: 'ws://localhost:9222/devtools/browser/...'
    });
    
    const page = await browser.newPage();
    
    try {
        await api.withPage(page, async () => {
            // Initialize
            await api.init(page, { persona: 'casual' });
            
            // Navigate
            await api.goto('https://example.com');
            
            // Interact
            await api.type('input#search', 'query');
            await api.press('Enter');
            
            // Wait for results
            await api.wait.forElement('.results');
            
            // Click first result
            await api.click('.result-item:first-child');
            
            // Take screenshot
            await api.screenshot.save('result.png');
        });
    } catch (error) {
        console.error('Automation failed:', error);
    } finally {
        await browser.close();
    }
}

automate();
```

---

## Related Documentation

- [docs/api.md](api.md) - Full API reference
- [docs/tasks.md](tasks.md) - Available tasks
- [AGENTS.md](../AGENTS.md) - Agent usage
- [QUICKSTART.md](../QUICKSTART.md) - Getting started
