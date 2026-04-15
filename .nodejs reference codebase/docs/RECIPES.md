# Auto-AI Recipes

Common automation patterns and recipes for copy-paste usage.

---

## Table of Contents

1. [Navigation](#navigation)
2. [Element Interaction](#element-interaction)
3. [Forms](#forms)
4. [Scrolling](#scrolling)
5. [Waiting](#waiting)
6. [Screenshots](#screenshots)
7. [Twitter Automation](#twitter-automation)
8. [Error Handling](#error-handling)
9. [Advanced Patterns](#advanced-patterns)

---

## Navigation

### Basic Page View

```javascript
import { api } from './api/index.js';

await api.withPage(page, async () => {
    await api.init(page);
    await api.goto('https://example.com');
});
```

### Navigate and Wait for Load

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.goto('https://example.com', { waitUntil: 'networkidle' });
});
```

### Navigate with Timeout

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.goto('https://example.com', { 
        timeout: 30000,
        waitUntil: 'domcontentloaded'
    });
});
```

### Go Back/Forward

```javascript
await api.withPage(page, async () => {
    await api.goBack();
    await api.goForward();
    await api.reload();
});
```

---

## Element Interaction

### Click Element

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.click('.button');
});
```

### Click with Recovery

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.click('.button', {
        retries: 3,
        delay: 500
    });
});
```

### Double Click

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.dblclick('.item');
});
```

### Right Click

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.rightClick('.menu-target');
});
```

### Click at Coordinates

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.clickAt(100, 200);  // x, y
});
```

---

## Forms

### Fill Single Field

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.type('input[name="email"]', 'user@example.com');
});
```

### Fill Multiple Fields

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    
    await api.type('input[name="username"]', 'john_doe');
    await api.type('input[name="email"]', 'john@example.com');
    await api.type('input[name="password"]', 'secret123');
    
    await api.click('button[type="submit"]');
});
```

### Fill with Typing Delay

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.type('textarea', 'Long text content', {
        delay: 100  // ms between keystrokes
    });
});
```

### Clear and Fill

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.fill('input', 'new value');  // Clears first
});
```

### Press Keys

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    
    await api.type('input', 'search query');
    await api.press('Enter');
    
    // Or special keys
    await api.press('Tab');
    await api.press('Control+A');
    await api.press('Delete');
});
```

### Select Dropdown

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.select('select[name="country"]', 'US');
});
```

---

## Scrolling

### Scroll Down

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.scroll.down(500);  // pixels
});
```

### Scroll Up

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.scroll.up(300);
});
```

### Scroll to Top

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.scroll.toTop();
});
```

### Scroll to Bottom

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.scroll.toBottom();
});
```

### Scroll to Element

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.scroll.toElement('.target');
});
```

### Natural Reading Scroll

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.scroll.read();  // Simulates reading pattern
});
```

---

## Waiting

### Wait Fixed Time

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.wait(2000);  // ms
});
```

### Wait for Element

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.wait.forElement('.loaded');
});
```

### Wait for Element Hidden

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.wait.forElementHidden('.spinner');
});
```

### Wait for Navigation

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.click('a[href="/next"]');
    await api.wait.forNavigation();
});
```

### Wait for Network Idle

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.wait.forNetworkIdle();
});
```

### Wait for Text

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.wait.forText('Success');
});
```

---

## Screenshots

### Full Page Screenshot

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.screenshot.full();
    await api.screenshot.save('full-page.png');
});
```

### Visible Area Screenshot

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.screenshot.visible();
    await api.screenshot.save('visible.png');
});
```

### Element Screenshot

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.screenshot.element('.component');
    await api.screenshot.save('element.png');
});
```

---

## Twitter Automation

### Follow User

```javascript
await api.withPage(page, async () => {
    await api.init(page, { persona: 'casual' });
    await api.goto('https://twitter.com/username');
    await api.wait.forElement('[data-testid="follow"]');
    await api.click('[data-testid="follow"]');
});
```

### Like Tweet

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.goto('https://twitter.com/user/status/123');
    await api.click('[data-testid="like"]');
});
```

### Retweet

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.goto('https://twitter.com/user/status/123');
    await api.click('[data-testid="retweet"]');
    await api.click('[role="menuitem"]:has-text("Retweet")');
});
```

### Reply to Tweet

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    await api.goto('https://twitter.com/user/status/123');
    
    await api.click('[data-testid="reply"]');
    await api.type('[data-testid="tweetTextarea"]', 'Great post!');
    await api.click('[data-testid="tweetButton"]');
});
```

---

## Error Handling

### Try-Catch with Fallback

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    
    try {
        await api.click('.primary-action');
    } catch (error) {
        console.log('Primary not found, trying alternative...');
        await api.click('.secondary-action');
    }
});
```

### Check Element Exists

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    
    if (await api.exists('.optional-element')) {
        await api.click('.optional-element');
    }
});
```

### Retry Logic

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    
    for (let i = 0; i < 3; i++) {
        try {
            await api.click('.flaky-element');
            break;  // Success
        } catch (error) {
            if (i === 2) throw error;  // Last attempt
            await api.wait(1000 * (i + 1));  // Exponential backoff
        }
    }
});
```

---

## Advanced Patterns

### Multiple Tabs

```javascript
const pages = [];

for (let i = 0; i < 3; i++) {
    const page = await browser.newPage();
    pages.push(page);
}

// Process in parallel
await Promise.all(pages.map(page => 
    api.withPage(page, async () => {
        await api.init(page);
        await api.goto('https://example.com');
    })
));
```

### Session Persistence

```javascript
// Save cookies
const cookies = await page.context().cookies();
fs.writeFileSync('cookies.json', JSON.stringify(cookies));

// Load cookies
const cookies = JSON.parse(fs.readFileSync('cookies.json'));
await page.context().addCookies(cookies);
```

### Wait for Condition

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    
    // Wait for custom condition
    await page.waitForFunction(() => {
        return document.querySelector('.ready') !== null;
    });
});
```

### Loop with Condition

```javascript
await api.withPage(page, async () => {
    await api.init(page);
    
    let loaded = false;
    let attempts = 0;
    
    while (!loaded && attempts < 10) {
        await api.scroll.down(500);
        await api.wait(1000);
        
        loaded = await api.exists('.all-loaded');
        attempts++;
    }
});
```

---

## Related Documentation

- [API-CHEATSHEET.md](../API-CHEATSHEET.md) - Quick reference
- [docs/api.md](api.md) - Full API documentation
- [docs/USER-GUIDE.md](USER-GUIDE.md) - User guide
- [examples/](../examples/) - Working code examples

---

*Last updated: 2026-03-31*
