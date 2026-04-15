# Tasks

Auto-AI comes with built-in automation tasks. Tasks are loaded dynamically from the `tasks/` directory.

## Running Tasks

### From Command Line

```bash
node main.js taskName=param
```

### Built-in Tasks

| Task             | Usage                              | Description           |
| ---------------- | ---------------------------------- | --------------------- |
| `pageview`       | `pageview=<url>`                   | Navigate to URL       |
| `twitterFollow`  | `twitterFollow=<url>`              | Follow a Twitter user |
| `twitterLike`    | `like tweetUrl=<url>`              | Like a tweet          |
| `twitterRetweet` | `retweet tweetUrl=<url>`           | Retweet               |
| `twitterQuote`   | `quote tweetUrl=<url> text=<text>` | Quote tweet           |

## Task Examples

### Page View

```bash
node main.js pageview=example.com
```

### Twitter Follow

```bash
node main.js twitterFollow=https://twitter.com/username
```

### Twitter Like

```bash
node main.js like tweetUrl="https://twitter.com/user/status/123456789"
```

### Twitter Retweet

```bash
node main.js retweet tweetUrl="https://twitter.com/user/status/123456789"
```

### Chain Tasks

```bash
node main.js pageview=twitter.com then twitterFollow=https://twitter.com/user
```

## Creating Custom Tasks

### Task Structure

Create a file in `tasks/` with this pattern:

```javascript
// tasks/my-task.js
export default async function myTask(page, payload) {
    const { targetUrl, options } = payload;

    await page.goto(targetUrl);
    // ... task logic

    return { success: true };
}
```

### Task Metadata

Add metadata for auto-discovery:

```javascript
/**
 * My custom task
 * @param {string} targetUrl - URL to navigate to
 * @param {object} options - Task options
 */
export default async function myTask(page, payload) {
    // ...
}
```

### Register Task

Tasks are auto-loaded. Just add to `tasks/` folder.

## Agent Tasks

For AI-driven automation, use `agent-main.js`:

```bash
# OWB game agent
node agent-main.js owb play

# OWB with specific strategy
node agent-main.js owb rush
node agent-main.js owb turtle
node agent-main.js owb economy
```

## Task Payload

Tasks receive a payload object:

```javascript
{
    taskName: 'pageview',
    targetUrl: 'https://example.com',
    browser: { port: 9222, type: 'chrome' },
    session: { id: 'session-1' },
    // ... task-specific options
}
```

## Error Handling

Tasks should return:

```javascript
return {
    success: true,      // or false
    message: 'Done',   // status message
    data: { ... }       // optional result data
};
```

## More Information

- [API Reference](api.md) - Task API
- [Configuration](configuration.md) - Task settings
