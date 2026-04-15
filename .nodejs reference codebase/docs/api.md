# API Reference

Complete reference for all Auto-AI API methods.

## Table of Contents

- [Getting Started](#getting-started)
- [Context Methods](#context-methods)
- [Actions](#actions)
- [Scroll](#scroll)
- [Cursor](#cursor)
- [Queries](#queries)
- [Wait](#wait)
- [Navigation](#navigation)
- [Warmup](#warmup)
- [Timing](#timing)
- [Persona](#persona)
- [Recovery](#recovery)
- [Attention](#attention)
- [Idle](#idle)
- [Patch](#patch)
- [File I/O](#file-io)
- [Agent](#agent)
- [Game](#game)
- [Twitter](#twitter)
- [Events & Plugins](#events--plugins)
- [Middleware](#middleware)
- [Config](#config)
- [Errors](#errors)

---

## Getting Started

```javascript
import { api } from './api/index.js';

// All operations must be wrapped in withPage
await api.withPage(async (page) => {
    await api.init(page);
    // ... use API methods
});
```

---

## Context Methods

### `api.withPage(page, fn)`

Execute code within an isolated page context. Required for all API methods.

```javascript
await api.withPage(async (page) => {
    const title = await page.title();
    return title;
});
```

| Parameter | Type     | Description               |
| --------- | -------- | ------------------------- |
| `page`    | Page     | Playwright page instance  |
| `fn`      | Function | Async function to execute |

---

### `api.init(page, options)`

Initialize a page with humanization, persona, and detection patching.

```javascript
await api.init(page, {
    persona: 'casual',
    patch: true,
    humanizationPatch: true,
    sensors: true,
});
```

| Parameter                   | Type    | Default   | Description               |
| --------------------------- | ------- | --------- | ------------------------- |
| `page`                      | Page    | -         | Playwright page           |
| `options.persona`           | string  | 'default' | Persona name              |
| `options.patch`             | boolean | true      | Enable detection patching |
| `options.humanizationPatch` | boolean | true      | Enable humanization       |
| `options.sensors`           | boolean | false     | Enable sensor simulation  |

---

### `api.getPage()`

Get the current page instance within context.

```javascript
const page = api.getPage();
```

---

### `api.isSessionActive()`

Check if session is active.

```javascript
const active = api.isSessionActive();
```

---

### `api.checkSession()`

Verify session status, throws if not active.

```javascript
api.checkSession();
```

---

### `api.clearContext()`

Clear the current context.

```javascript
await api.clearContext();
```

---

### `api.screenshot(options)`

Take a screenshot of the current page.

```javascript
const buffer = await api.screenshot({
    path: 'screenshot.png',
    fullPage: false,
    type: 'jpeg',
    quality: 80,
});
```

| Parameter          | Type    | Default | Description       |
| ------------------ | ------- | ------- | ----------------- |
| `options.path`     | string  | -       | Output file path  |
| `options.fullPage` | boolean | false   | Capture full page |
| `options.type`     | string  | 'jpeg'  | Image type        |
| `options.quality`  | number  | 80      | JPEG quality      |

---

### `api.diagnose(page)`

Diagnose page state and return debug info.

```javascript
const info = await api.diagnose(page);
```

---

### `api.config`

Configuration manager.

```javascript
const settings = api.config.load();
api.config.get('key');
api.config.set('key', value);
```

---

## Actions

### `api.click(selector, options)`

Click an element with human-like mouse movement.

```javascript
await api.click('#submit-button');
await api.click('.btn', { precision: 'safe', recovery: true });
```

| Parameter           | Type    | Default | Description              |
| ------------------- | ------- | ------- | ------------------------ |
| `selector`          | string  | -       | Element selector         |
| `options.recovery`  | boolean | true    | Auto-recovery on failure |
| `options.precision` | string  | 'safe'  | 'exact', 'safe', 'rough' |
| `options.button`    | string  | 'left'  | Mouse button             |
| `options.force`     | boolean | false   | Force click              |

---

### `api.type(selector, text, options)`

Type text with keystroke dynamics.

```javascript
await api.type('input[name="email"]', 'user@example.com');
await api.type('#search', 'query', { delay: 50, humanize: true });
```

| Parameter          | Type    | Default | Description              |
| ------------------ | ------- | ------- | ------------------------ |
| `selector`         | string  | -       | Element selector         |
| `text`             | string  | -       | Text to type             |
| `options.delay`    | number  | 0       | Delay between keystrokes |
| `options.noClear`  | boolean | false   | Don't clear field first  |
| `options.humanize` | boolean | true    | Apply human-like timing  |

---

### `api.hover(selector)`

Hover over an element.

```javascript
await api.hover('.menu-item');
```

---

### `api.rightClick(selector)`

Right-click an element.

```javascript
await api.rightClick('.context-menu');
```

---

### `api.drag(startSelector, endSelector)`

Drag element from start to end.

```javascript
await api.drag('.draggable', '#drop-target');
```

---

### `api.clickAt(x, y, options)`

Click at specific coordinates.

```javascript
await api.clickAt(500, 300);
```

---

### `api.multiSelect(selector, values)`

Select multiple options.

```javascript
await api.multiSelect('select[name="colors"]', ['red', 'blue']);
```

---

### `api.press(selector, key)`

Press a key on an element.

```javascript
await api.press('input', 'Enter');
await api.press('textarea', 'Control+a');
```

---

### `api.hold(selector, duration)`

Hold down mouse button on element.

```javascript
await api.hold('.slider', 2000);
```

---

### `api.releaseAll()`

Release all held mouse buttons.

```javascript
await api.releaseAll();
```

---

## Scroll

### `api.scroll(amount, direction)`

Scroll the page.

```javascript
await api.scroll(500);
await api.scroll(300, 'down');
await api.scroll(100, 'up');
```

| Parameter   | Type   | Default | Description                   |
| ----------- | ------ | ------- | ----------------------------- |
| `amount`    | number | 300     | Pixels to scroll              |
| `direction` | string | 'down'  | 'up', 'down', 'left', 'right' |

---

### `api.scroll.focus(selector)`

Scroll until element is in focus.

```javascript
await api.scroll.focus('#footer');
```

---

### `api.scroll.toTop()`

Scroll to top of page.

```javascript
await api.scroll.toTop();
```

---

### `api.scroll.toBottom()`

Scroll to bottom of page.

```javascript
await api.scroll.toBottom();
```

---

### `api.scroll.read()`

Read-scroll: smooth scroll with pauses.

```javascript
await api.scroll.read();
```

---

### `api.scroll.back()`

Scroll back to previous position.

```javascript
await api.scroll.back();
```

---

## Cursor

### `api.cursor(selector)`

Move cursor to element.

```javascript
await api.cursor('.button');
```

---

### `api.cursor.move(x, y)`

Move cursor to coordinates.

```javascript
await api.cursor.move(100, 200);
```

---

### `api.cursor.up(pixels)`

Move cursor up.

```javascript
await api.cursor.up(50);
```

---

### `api.cursor.down(pixels)`

Move cursor down.

```javascript
await api.cursor.down(50);
```

---

### `api.cursor.startFidgeting()`

Start cursor fidgeting (random small movements).

```javascript
api.cursor.startFidgeting();
```

---

### `api.cursor.stopFidgeting()`

Stop cursor fidgeting.

```javascript
api.cursor.stopFidgeting();
```

---

## Queries

### `api.text(selector)`

Get text content of element.

```javascript
const text = await api.text('.title');
```

---

### `api.attr(selector, attribute)`

Get element attribute.

```javascript
const href = await api.attr('a.link', 'href');
```

---

### `api.visible(selector)`

Check if element is visible.

```javascript
const isVisible = await api.visible('.modal');
```

---

### `api.count(selector)`

Count elements matching selector.

```javascript
const count = await api.count('.item');
```

---

### `api.exists(selector)`

Check if element exists.

```javascript
const exists = await api.exists('.error');
```

---

### `api.getCurrentUrl()`

Get current page URL.

```javascript
const url = await api.getCurrentUrl();
```

---

## Wait

### `api.wait(ms)`

Wait for specified milliseconds.

```javascript
await api.wait(1000);
```

---

### `api.waitFor(selector, options)`

Wait for element to meet condition.

```javascript
await api.waitFor('.loaded', { state: 'visible', timeout: 5000 });
```

| Parameter                | Type    | Default   | Description                     |
| ------------------------ | ------- | --------- | ------------------------------- |
| `selector`               | string  | -         | Element selector                |
| `options.timeout`        | number  | 30000     | Timeout in ms                   |
| `options.state`          | string  | 'visible' | 'visible', 'hidden', 'attached' |
| `options.throwOnTimeout` | boolean | true      | Throw or return false           |

---

### `api.waitVisible(selector, timeout)`

Wait for element to be visible.

```javascript
await api.waitVisible('.modal', 5000);
```

---

### `api.waitHidden(selector, timeout)`

Wait for element to be hidden.

```javascript
await api.waitHidden('.loader', 5000);
```

---

### `api.waitForLoadState(state, timeout)`

Wait for page load state.

```javascript
await api.waitForLoadState('networkidle');
```

---

### `api.waitForURL(pattern, options)`

Wait for URL to match pattern.

```javascript
await api.waitForURL('**/dashboard/**');
```

---

## Navigation

### `api.goto(url, options)`

Navigate to URL.

```javascript
await api.goto('https://example.com');
await api.goto('https://example.com', { timeout: 30000 });
```

| Parameter           | Type   | Default | Description                               |
| ------------------- | ------ | ------- | ----------------------------------------- |
| `url`               | string | -       | Target URL                                |
| `options.timeout`   | number | 30000   | Navigation timeout                        |
| `options.waitUntil` | string | 'load'  | 'load', 'domcontentloaded', 'networkidle' |
| `options.headers`   | object | -       | Extra HTTP headers                        |

---

### `api.reload(options)`

Reload current page.

```javascript
await api.reload();
```

---

### `api.back()`

Go back in history.

```javascript
await api.back();
```

---

### `api.forward()`

Go forward in history.

```javascript
await api.forward();
```

---

### `api.setExtraHTTPHeaders(headers)`

Set extra HTTP headers for navigation.

```javascript
await api.setExtraHTTPHeaders({ 'X-Custom-Header': 'value' });
```

---

## Warmup

### `api.beforeNavigate()`

Perform warmup actions before navigation.

```javascript
await api.beforeNavigate();
```

---

### `api.randomMouse()`

Move mouse randomly (human-like).

```javascript
await api.randomMouse();
```

---

### `api.fakeRead()`

Simulate reading behavior (scrolling with pauses).

```javascript
await api.fakeRead();
```

---

### `api.warmupPause()`

Pause for warmup.

```javascript
await api.warmupPause();
```

---

## Timing

### `api.think(ms)`

Think pause - simulate thinking time.

```javascript
await api.think(2000);
```

---

### `api.delay(ms)`

Simple delay.

```javascript
await api.delay(500);
```

---

### `api.gaussian(mean, stdDev)`

Gaussian random delay.

```javascript
await api.gaussian(1000, 200);
```

---

### `api.randomInRange(min, max)`

Random delay in range.

```javascript
await api.randomInRange(500, 2000);
```

---

## Persona

### `api.setPersona(name)`

Set active persona.

```javascript
await api.setPersona('stealth');
```

---

### `api.getPersona()`

Get current persona object.

```javascript
const persona = api.getPersona();
```

---

### `api.getPersonaName()`

Get current persona name.

```javascript
const name = api.getPersonaName();
```

---

### `api.listPersonas()`

List available personas.

```javascript
const personas = api.listPersonas();
```

---

### `api.getSessionDuration()`

Get session duration in ms.

```javascript
const duration = api.getSessionDuration();
```

---

## Recovery

### `api.recover(action)`

Attempt to recover from error by retrying action.

```javascript
await api.recover(async () => {
    await api.click('.btn');
});
```

---

### `api.goBack()`

Go back with recovery.

```javascript
await api.goBack();
```

---

### `api.findElement(selector)`

Find element with fallback strategies.

```javascript
const element = await api.findElement('#btn');
```

---

### `api.smartClick(selector)`

Smart click with recovery.

```javascript
await api.smartClick('.submit');
```

---

### `api.undo()`

Undo last action.

```javascript
await api.undo();
```

---

### `api.urlChanged()`

Check if URL changed.

```javascript
const changed = await api.urlChanged(initialUrl);
```

---

## Attention

### `api.gaze()`

Simulate looking at page.

```javascript
await api.gaze();
```

---

### `api.attention()`

Attention behavior - focus simulation.

```javascript
await api.attention();
```

---

### `api.distraction()`

Simulate distraction.

```javascript
await api.distraction();
```

---

### `api.beforeLeave()`

Behavior before leaving page.

```javascript
await api.beforeLeave();
```

---

### `api.focusShift()`

Shift focus between elements.

```javascript
await api.focusShift();
```

---

### `api.maybeDistract()`

Randomly simulate distraction.

```javascript
await api.maybeDistract();
```

---

### `api.setDistractionChance(chance)`

Set probability of distraction.

```javascript
api.setDistractionChance(0.3);
```

---

### `api.getDistractionChance()`

Get distraction probability.

```javascript
const chance = api.getDistractionChance();
```

---

## Idle

### `api.idle.start()`

Start idle behavior.

```javascript
api.idle.start();
```

---

### `api.idle.stop()`

Stop idle behavior.

```javascript
api.idle.stop();
```

---

### `api.idle.isRunning()`

Check if idle is running.

```javascript
const running = api.idle.isRunning();
```

---

### `api.idle.wiggle()`

Small random cursor movement.

```javascript
await api.idle.wiggle();
```

---

### `api.idle.scroll()`

Idle-time scrolling.

```javascript
await api.idle.scroll();
```

---

### `api.idle.heartbeat()`

Start heartbeat for idle detection.

```javascript
api.idle.heartbeat();
```

---

## Patch

### `api.patch.apply(page)`

Apply detection patches to page.

```javascript
await api.patch.apply(page);
```

---

### `api.patch.stripCDPMarkers()`

Strip CDP markers from content.

```javascript
const clean = api.patch.stripCDPMarkers(html);
```

---

### `api.patch.check(page)`

Check for detection signs.

```javascript
const detected = await api.patch.check(page);
```

---

## File I/O

### `api.file.readline(filename)`

Read a random line from file.

```javascript
const line = await api.file.readline('proxies.txt');
```

---

### `api.file.consumeline(filename)`

Read and remove a random line.

```javascript
const line = await api.file.consumeline('proxies.txt');
```

---

## Agent

### `api.agent(goal, config)`

Run AI agent with goal.

```javascript
const result = await api.agent('Click the login button');
```

---

### `api.agent.run(goal, config)`

Run agent loop.

```javascript
await api.agent.run('Follow @twitter', { maxSteps: 50 });
```

---

### `api.agent.stop()`

Stop running agent.

```javascript
await api.agent.stop();
```

---

### `api.agent.isRunning()`

Check if agent is running.

```javascript
const running = api.agent.isRunning();
```

---

### `api.agent.see()`

Get semantic view of page.

```javascript
const view = await api.agent.see();
```

---

### `api.agent.do(action, target)`

Execute action on target.

```javascript
await api.agent.do('click', 'Login');
```

---

### `api.agent.find(goal)`

Find element for goal.

```javascript
const element = await api.agent.find('search box');
```

---

### `api.agent.screenshot()`

Take agent-aware screenshot.

```javascript
const screenshot = await api.agent.screenshot();
```

---

### `api.agent.captureAXTree()`

Capture accessibility tree.

```javascript
const tree = await api.agent.captureAXTree();
```

---

### `api.agent.captureState(options)`

Capture full agent state.

```javascript
const state = await api.agent.captureState({ vprep: true });
```

---

### `api.agent.vision`

Vision processing module.

```javascript
const processed = await api.agent.vision.process(image);
```

---

### `api.agent.engine`

Action engine instance.

```javascript
const engine = api.agent.engine;
```

---

### `api.agent.llm`

LLM client instance.

```javascript
const llm = api.agent.llm;
```

---

### `api.agent.getStats()`

Get agent usage statistics.

```javascript
const stats = api.agent.getStats();
```

---

## Game

### `api.game.state`

Game state operations.

```javascript
const state = await api.game.state.get();
await api.game.state.waitFor('resources');
```

---

### `api.game.units`

Unit operations.

```javascript
await api.game.units.select('.selected');
const units = await api.game.units.getVisible();
```

---

### `api.game.resources`

Resource tracking.

```javascript
const gold = await api.game.resources.get('gold');
await api.game.resources.waitFor({ gold: 500 });
```

---

### `api.game.menus`

Menu operations.

```javascript
await api.game.menus.open('build');
await api.game.menus.select('barracks');
```

---

### `api.gameAgent.run(goal, config)`

Run game-specific agent.

```javascript
const result = await api.gameAgent.run('Build barracks and train units');
```

---

### `api.gameAgent.stop()`

Stop game agent.

```javascript
await api.gameAgent.stop();
```

---

## V-PREP (Vision Preprocessor)

### `api.vprep.process(input, config)`

Process image for LLM vision.

```javascript
const result = await api.vprep.process(buffer, {
    targetWidth: 800,
    grayscale: true,
    contrast: 1.3,
});
```

---

### `api.vprep.presets`

Predefined processing configs.

```javascript
const preset = api.vprep.presets.GAME_UI;
```

---

### `api.vprep.getStats()`

Get processing statistics.

```javascript
const stats = api.vprep.getStats();
```

---

## Twitter

### `api.twitter.intent.like(tweetUrl)`

Like a tweet via Twitter intent.

```javascript
await api.twitter.intent.like('https://twitter.com/user/status/123');
```

---

### `api.twitter.intent.quote(tweetUrl, text)`

Quote a tweet.

```javascript
await api.twitter.intent.quote('https://twitter.com/user/status/123', 'Great post!');
```

---

### `api.twitter.intent.retweet(tweetUrl)`

Retweet.

```javascript
await api.twitter.intent.retweet('https://twitter.com/user/status/123');
```

---

### `api.twitter.intent.follow(url)`

Follow a user.

```javascript
await api.twitter.intent.follow('https://twitter.com/username');
```

---

### `api.twitter.intent.post(text)`

Post a tweet.

```javascript
await api.twitter.intent.post('Hello world!');
```

---

### `api.twitter.home()`

Navigate to Twitter home.

```javascript
await api.twitter.home();
```

---

### `api.twitter.isOnHome()`

Check if on Twitter home.

```javascript
const onHome = await api.twitter.isOnHome();
```

---

## Events & Plugins

### `api.events`

Get available events.

```javascript
const events = api.events;
```

---

### `api.plugins.register(name, plugin)`

Register a plugin.

```javascript
api.plugins.register('myPlugin', {
    onInit: () => {},
    onAction: () => {},
});
```

---

### `api.plugins.enable(name)`

Enable plugin.

```javascript
api.plugins.enable('myPlugin');
```

---

### `api.plugins.disable(name)`

Disable plugin.

```javascript
api.plugins.disable('myPlugin');
```

---

### `api.plugins.list()`

List all plugins.

```javascript
const plugins = api.plugins.list();
```

---

### `api.plugins.listEnabled()`

List enabled plugins.

```javascript
const enabled = api.plugins.listEnabled();
```

---

## Middleware

### `api.middleware.createPipeline(middlewares)`

Create async middleware pipeline.

```javascript
const pipeline = api.middleware.createPipeline([api.middleware.logging, api.middleware.retry]);
```

---

### `api.middleware.logging`

Logging middleware.

```javascript
const logged = api.middleware.logging(next);
```

---

### `api.middleware.validation`

Validation middleware.

```javascript
const validated = api.middleware.validation(next);
```

---

### `api.middleware.retry`

Retry middleware.

```javascript
const retried = api.middleware.retry(next);
```

---

### `api.middleware.recovery`

Recovery middleware.

```javascript
const recovered = api.middleware.recovery(next);
```

---

### `api.middleware.metrics`

Metrics middleware.

```javascript
const measured = api.middleware.metrics(next);
```

---

### `api.middleware.rateLimit`

Rate limit middleware.

```javascript
const limited = api.middleware.rateLimit(next);
```

---

## Memory

### `api.memory`

Memory profiler.

```javascript
const mem = api.memory;
mem.start();
const stats = mem.get();
```

---

## Visual Debug

### `api.visualDebug`

Visual debugging tools.

```javascript
api.visualDebug.enable();
await api.visualDebug.capture();
```

---

## Config

### `api.config.load()`

Load all configuration.

```javascript
const config = api.config.load();
```

---

### `api.config.get(key)`

Get config value.

```javascript
const value = api.config.get('llm.model');
```

---

### `api.config.set(key, value)`

Set config value.

```javascript
api.config.set('llm.model', 'claude-3');
```

---

## Errors

All error classes are exported:

```javascript
import {
    AutomationError,
    SessionError,
    SessionTimeoutError,
    ElementNotFoundError,
    ElementObscuredError,
    NavigationError,
    LLMError,
    LLMTimeoutError,
    ConfigError,
    ValidationError,
} from './api/index.js';
```

### Error Types

| Error                  | Description            |
| ---------------------- | ---------------------- |
| `AutomationError`      | Base automation error  |
| `SessionError`         | Session-related errors |
| `SessionTimeoutError`  | Session timeout        |
| `SessionNotFoundError` | Session not found      |
| `ElementNotFoundError` | Element not found      |
| `ElementObscuredError` | Element is obscured    |
| `ElementTimeoutError`  | Element wait timeout   |
| `NavigationError`      | Navigation failed      |
| `LLMError`             | LLM generic error      |
| `LLMTimeoutError`      | LLM timeout            |
| `LLMRateLimitError`    | LLM rate limit         |
| `ConfigError`          | Configuration error    |
| `ValidationError`      | Validation failed      |
